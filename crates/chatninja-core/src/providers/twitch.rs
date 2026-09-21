use crate::{
    auth::{self, json, required, Token},
    model::{timestamp, ChatMessage, Fragment, Removal},
    status, Config, Event, Provider, Sink,
};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde_json::{json as value, Value};
use tokio::time::{sleep, timeout, Duration, Instant};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub fn normalize(packet: &Value) -> Option<Event> {
    let payload = &packet["payload"];
    let event = &payload["event"];
    let kind = payload["subscription"]["type"].as_str()?;
    let channel = event["broadcaster_user_id"].as_str()?;
    if kind != "channel.chat.message" {
        let (message_id, author_id) = match kind {
            "channel.chat.message_delete" => {
                (Some(event["message_id"].as_str()?.to_string()), None)
            }
            "channel.chat.clear_user_messages" => {
                (None, Some(event["target_user_id"].as_str()?.to_string()))
            }
            "channel.chat.clear" => (None, None),
            _ => return None,
        };
        return Some(Event::Remove(Removal {
            platform: Provider::Twitch,
            channel_id: channel.into(),
            message_id,
            author_id,
        }));
    }
    let mut fragments = vec![];
    if let Some(items) = event["message"]["fragments"].as_array() {
        for fragment in items {
            let text = fragment["text"].as_str().unwrap_or("");
            if fragment["type"] == "emote" {
                if let Some(id) = fragment["emote"]["id"].as_str().filter(|id| {
                    id.len() < 256
                        && id
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                }) {
                    fragments.push(Fragment::Emote {
                        id: id.into(),
                        name: text.into(),
                        url: format!(
                            "https://static-cdn.jtvnw.net/emoticons/v2/{id}/default/dark/2.0"
                        ),
                    });
                    continue;
                }
            }
            fragments.push(Fragment::Text { text: text.into() });
        }
    }
    if fragments.is_empty() {
        fragments.push(Fragment::Text {
            text: event["message"]["text"].as_str()?.into(),
        });
    }
    Some(Event::Message(ChatMessage {
        id: event["message_id"].as_str()?.into(),
        platform: Provider::Twitch,
        channel_id: channel.into(),
        author: event["chatter_user_name"].as_str()?.into(),
        author_id: event["chatter_user_id"].as_str()?.into(),
        timestamp: timestamp(
            packet["metadata"]["message_timestamp"]
                .as_str()
                .unwrap_or(""),
        ),
        fragments,
        moderator: event["badges"].as_array().is_some_and(|badges| {
            badges
                .iter()
                .any(|badge| badge["set_id"] == "moderator" || badge["set_id"] == "broadcaster")
        }),
    }))
}
pub fn valid_reconnect(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "wss"
            && url.host_str() == Some("eventsub.wss.twitch.tv")
            && url.port_or_known_default() == Some(443)
            && url.username().is_empty()
            && url.password().is_none()
    })
}
async fn validate(client: &Client, config: &Config, token: &mut Token) -> Result<String, String> {
    for attempt in 0..2 {
        auth::ensure_fresh(client, config, token, false).await?;
        let response = client
            .get("https://id.twitch.tv/oauth2/validate")
            .header("Authorization", format!("OAuth {}", token.access_token))
            .send()
            .await
            .map_err(|_| "network_error")?;
        if response.status() == 401 && attempt == 0 {
            auth::ensure_fresh(client, config, token, true).await?;
            continue;
        }
        if !response.status().is_success() {
            return Err("reauthorization_required".into());
        }
        let data = json(response).await?;
        if data["client_id"].as_str() != Some(config.client_id.as_str())
            || !data["scopes"]
                .as_array()
                .is_some_and(|scopes| scopes.iter().any(|s| s == "user:read:chat"))
        {
            return Err("twitch_scope_or_client_mismatch".into());
        }
        return Ok(required(&data, "user_id")?.into());
    }
    Err("reauthorization_required".into())
}
async fn session(
    client: &Client,
    config: &Config,
    token: &mut Token,
    sink: &Sink,
) -> Result<(), String> {
    let user_id = validate(client, config, token).await?;
    let (mut socket, _) = timeout(
        Duration::from_secs(20),
        connect_async("wss://eventsub.wss.twitch.tv/ws"),
    )
    .await
    .map_err(|_| "network_error")?
    .map_err(|_| "network_error")?;
    let mut deadline = Instant::now() + Duration::from_secs(15);
    let mut keepalive = Duration::from_secs(20);
    let mut last_validation = Instant::now();
    let mut welcomed = false;
    let mut timer = tokio::time::interval(Duration::from_secs(60));
    loop {
        let message = tokio::select! {
            _ = tokio::time::sleep_until(deadline) => return Err("network_error".into()),
            _ = timer.tick() => {
                auth::ensure_fresh(client, config, token, false).await?;
                if last_validation.elapsed() >= Duration::from_secs(3500) {
                    if validate(client, config, token).await? != user_id { return Err("twitch_identity_changed".into()); }
                    last_validation = Instant::now();
                }
                continue;
            },
            message = socket.next() => message.ok_or("network_error")?.map_err(|_| "network_error")?,
        };
        if let Message::Ping(data) = message {
            socket
                .send(Message::Pong(data))
                .await
                .map_err(|_| "network_error")?;
            continue;
        }
        if matches!(message, Message::Close(_)) {
            return Err("network_error".into());
        }
        let Message::Text(text) = message else {
            continue;
        };
        if text.len() > 1_000_000 {
            return Err("provider_response_too_large".into());
        }
        let packet: Value = serde_json::from_str(&text).map_err(|_| "invalid_provider_response")?;
        match packet["metadata"]["message_type"].as_str().unwrap_or("") {
            "session_welcome" => {
                if welcomed {
                    return Err("invalid_provider_response".into());
                }
                welcomed = true;
                let session_id = required(&packet["payload"]["session"], "id")?;
                keepalive = Duration::from_secs(
                    packet["payload"]["session"]["keepalive_timeout_seconds"]
                        .as_u64()
                        .unwrap_or(10)
                        + 5,
                );
                for kind in [
                    "channel.chat.message",
                    "channel.chat.message_delete",
                    "channel.chat.clear",
                    "channel.chat.clear_user_messages",
                ] {
                    let response = client.post("https://api.twitch.tv/helix/eventsub/subscriptions")
                        .bearer_auth(&token.access_token).header("Client-Id", &config.client_id)
                        .json(&value!({"type":kind,"version":"1","condition":{"broadcaster_user_id":user_id,"user_id":user_id},"transport":{"method":"websocket","session_id":session_id}}))
                        .send().await.map_err(|_| "network_error")?;
                    if !response.status().is_success() {
                        return Err("twitch_subscription_failed".into());
                    }
                }
                status(sink, Provider::Twitch, "connected", "");
            }
            "session_reconnect" => {
                let reconnect = required(&packet["payload"]["session"], "reconnect_url")?;
                if !valid_reconnect(reconnect) {
                    return Err("invalid_reconnect_url".into());
                }
                // Keep the old connection alive until replacement welcomes us. Subscriptions transfer.
                let (mut next, _) = timeout(Duration::from_secs(15), connect_async(reconnect))
                    .await
                    .map_err(|_| "network_error")?
                    .map_err(|_| "network_error")?;
                let welcome = timeout(Duration::from_secs(15), next.next())
                    .await
                    .map_err(|_| "network_error")?
                    .ok_or("network_error")?
                    .map_err(|_| "network_error")?;
                let Message::Text(text) = welcome else {
                    return Err("invalid_provider_response".into());
                };
                let data: Value =
                    serde_json::from_str(&text).map_err(|_| "invalid_provider_response")?;
                if data["metadata"]["message_type"] != "session_welcome" {
                    return Err("invalid_provider_response".into());
                }
                keepalive = Duration::from_secs(
                    data["payload"]["session"]["keepalive_timeout_seconds"]
                        .as_u64()
                        .unwrap_or(10)
                        + 5,
                );
                let _ = socket.close(None).await;
                socket = next;
            }
            "notification" => {
                if let Some(event) = normalize(&packet) {
                    sink(event);
                }
            }
            "revocation" => return Err("twitch_subscription_revoked".into()),
            "session_keepalive" => {}
            _ => {}
        }
        deadline = Instant::now() + keepalive;
    }
}
pub async fn run(
    client: &Client,
    config: &Config,
    token: &mut Token,
    sink: &Sink,
) -> Result<(), String> {
    let mut failures = 0u32;
    loop {
        let start = Instant::now();
        match session(client, config, token, sink).await {
            Err(error) if error == "network_error" => {
                if start.elapsed() > Duration::from_secs(60) {
                    failures = 0;
                }
                failures += 1;
                if failures > 8 {
                    return Err("connection_retry_exhausted".into());
                }
                status(sink, Provider::Twitch, "reconnecting", "");
                sleep(Duration::from_secs(
                    2u64.pow(failures.min(6)) + rand::random::<u8>() as u64 % 3,
                ))
                .await;
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reject_offsite_reconnect() {
        assert!(valid_reconnect(
            "wss://eventsub.wss.twitch.tv/ws?reconnect=x"
        ));
        for url in [
            "ws://eventsub.wss.twitch.tv/ws",
            "wss://eventsub.wss.twitch.tv.evil.test/",
            "wss://user@eventsub.wss.twitch.tv/",
            "wss://eventsub.wss.twitch.tv:444/",
        ] {
            assert!(!valid_reconnect(url));
        }
    }
    #[test]
    fn native_emote_normalized() {
        let data = value!({"payload":{"subscription":{"type":"channel.chat.message"},"event":{"broadcaster_user_id":"c","chatter_user_name":"A","chatter_user_id":"u","message_id":"id","message":{"fragments":[{"type":"emote","text":"Kappa","emote":{"id":"25"}}]}}}});
        let Some(Event::Message(message)) = normalize(&data) else {
            panic!("expected message");
        };
        assert!(
            matches!(&message.fragments[0], Fragment::Emote { url, .. } if url == "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/2.0")
        );
    }
    #[test]
    fn clear_channel_normalized() {
        assert!(matches!(
            normalize(
                &value!({"payload":{"subscription":{"type":"channel.chat.clear"},"event":{"broadcaster_user_id":"c"}}})
            ),
            Some(Event::Remove(_))
        ));
    }
}
