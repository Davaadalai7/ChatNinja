use crate::{
    auth::{self, json, required, Token},
    model::{timestamp, ChatMessage, Fragment, Removal},
    status, Config, Event, Provider, Sink,
};
use reqwest::Client;
use serde_json::Value;
use tokio::time::{sleep, Duration, Instant};

pub fn relay_url(config: &Config, path: &str) -> Result<url::Url, String> {
    config.validate()?;
    url::Url::parse(&config.client_id)
        .map_err(|_| "invalid_kick_relay_origin".into())
        .and_then(|url| {
            url.join(path)
                .map_err(|_| "invalid_kick_relay_origin".into())
        })
}
pub async fn authorize(client: &Client, config: &Config) -> Result<Token, String> {
    let response = client
        .post(relay_url(config, "/oauth/start")?)
        .send()
        .await
        .map_err(|_| "network_error")?;
    if !response.status().is_success() {
        return Err("kick_relay_unavailable".into());
    }
    let data = json(response).await?;
    let poll = required(&data, "pollToken")?;
    let authorization = url::Url::parse(required(&data, "authorizationUrl")?)
        .map_err(|_| "invalid_verification_url")?;
    if authorization.scheme() != "https"
        || authorization.host_str() != Some("id.kick.com")
        || authorization.path() != "/oauth/authorize"
        || !authorization.username().is_empty()
        || authorization.password().is_some()
    {
        return Err("invalid_verification_url".into());
    }
    webbrowser::open(authorization.as_str()).map_err(|_| "browser_open_failed")?;
    let deadline =
        Instant::now() + Duration::from_secs(data["expiresIn"].as_u64().unwrap_or(300).min(300));
    while Instant::now() < deadline {
        sleep(Duration::from_secs(2)).await;
        let response = client
            .post(relay_url(config, "/oauth/result")?)
            .bearer_auth(poll)
            .send()
            .await
            .map_err(|_| "network_error")?;
        if response.status() == 202 {
            continue;
        }
        if !response.status().is_success() {
            return Err("kick_authorization_failed".into());
        }
        let result = json(response).await?;
        return Ok(Token {
            access_token: required(&result, "sessionToken")?.into(),
            refresh_token: None,
            expires_at: auth::now() + result["expiresIn"].as_u64().unwrap_or(0),
        });
    }
    Err("authorization_timeout".into())
}
pub fn normalize(kind: &str, payload: &Value) -> Option<Event> {
    let channel = payload["broadcaster"]["user_id"].as_u64()?.to_string();
    if kind == "moderation.banned" {
        return Some(Event::Remove(Removal {
            platform: Provider::Kick,
            channel_id: channel,
            message_id: None,
            author_id: Some(payload["banned_user"]["user_id"].as_u64()?.to_string()),
        }));
    }
    if kind != "chat.message.sent" {
        return None;
    }
    let sender = &payload["sender"];
    // Keep native emote markup as text until asset URLs are verified against provider catalogues.
    Some(Event::Message(ChatMessage {
        id: payload["message_id"].as_str()?.into(),
        platform: Provider::Kick,
        channel_id: channel,
        author: sender["username"].as_str()?.into(),
        author_id: sender["user_id"].as_u64()?.to_string(),
        timestamp: timestamp(payload["created_at"].as_str().unwrap_or("")),
        fragments: vec![Fragment::Text {
            text: payload["content"].as_str()?.into(),
        }],
        moderator: sender["identity"]["badges"]
            .as_array()
            .is_some_and(|badges| {
                badges
                    .iter()
                    .any(|badge| badge["type"] == "moderator" || badge["type"] == "broadcaster")
            }),
    }))
}
pub async fn run(
    client: &Client,
    config: &Config,
    token: &Token,
    sink: &Sink,
) -> Result<(), String> {
    let mut cursor = 0u64;
    let mut epoch = String::new();
    let mut failures = 0u32;
    loop {
        if token.expires_at <= auth::now() + 30 {
            return Err("reauthorization_required".into());
        }
        let response = client
            .get(relay_url(config, "/events")?)
            .bearer_auth(&token.access_token)
            .query(&[("after", cursor.to_string()), ("epoch", epoch.clone())])
            .send()
            .await;
        let response = match response {
            Ok(response) if response.status() == 401 => {
                return Err("reauthorization_required".into())
            }
            Ok(response) if response.status().is_success() => {
                failures = 0;
                response
            }
            _ => {
                failures += 1;
                if failures > 8 {
                    return Err("kick_relay_unavailable".into());
                }
                status(sink, Provider::Kick, "reconnecting", "");
                sleep(Duration::from_secs(2u64.pow(failures.min(6)))).await;
                continue;
            }
        };
        let data = json(response).await?;
        let next_epoch = required(&data, "epoch")?.to_string();
        epoch = next_epoch;
        let events = data["events"]
            .as_array()
            .ok_or("invalid_provider_response")?;
        for event in events {
            if let Some(normalized) =
                normalize(event["kind"].as_str().unwrap_or(""), &event["payload"])
            {
                sink(normalized);
            }
        }
        cursor = data["cursor"].as_u64().ok_or("invalid_provider_response")?;
        status(sink, Provider::Kick, "connected", "");
        sleep(Duration::from_secs(1)).await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_payload_ignored() {
        assert!(normalize("chat.message.sent", &serde_json::json!({})).is_none());
    }
    #[test]
    fn normalize_kick_message() {
        let event = normalize("chat.message.sent", &serde_json::json!({"message_id":"x","broadcaster":{"user_id":1},"sender":{"user_id":2,"username":"Bold"},"content":"Hello"})).unwrap();
        assert!(
            matches!(event, Event::Message(m) if m.platform == Provider::Kick && m.channel_id == "1")
        );
    }
    #[test]
    fn relay_origin_must_be_https() {
        let mut c = Config {
            provider: Provider::Kick,
            client_id: "http://localhost:8787".into(),
            desktop_secret: None,
        };
        assert!(c.validate().is_err());
        c.client_id = "https://relay.example.test".into();
        assert!(c.validate().is_ok());
        c.client_id = "https://relay.example.test/?token=x".into();
        assert!(c.validate().is_err());
    }
}
