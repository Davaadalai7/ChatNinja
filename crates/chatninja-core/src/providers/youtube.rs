use crate::{
    auth::{self, json, Token},
    model::{timestamp, ChatMessage, Fragment, Removal},
    status, Config, Event, Provider, Sink,
};
use reqwest::Client;
use serde_json::Value;
use tokio::time::{sleep, Duration};

pub fn normalize(item: &Value) -> Option<Event> {
    let snippet = &item["snippet"];
    let channel = snippet["liveChatId"].as_str()?;
    match snippet["type"].as_str()? {
        "messageDeletedEvent" => Some(Event::Remove(Removal {
            platform: Provider::Youtube,
            channel_id: channel.into(),
            message_id: Some(
                snippet["messageDeletedDetails"]["deletedMessageId"]
                    .as_str()?
                    .into(),
            ),
            author_id: None,
        })),
        "userBannedEvent" => Some(Event::Remove(Removal {
            platform: Provider::Youtube,
            channel_id: channel.into(),
            message_id: None,
            author_id: Some(
                snippet["userBannedDetails"]["bannedUserDetails"]["channelId"]
                    .as_str()?
                    .into(),
            ),
        })),
        _ => {
            let text = snippet["displayMessage"].as_str()?;
            let author = &item["authorDetails"];
            Some(Event::Message(ChatMessage {
                id: item["id"].as_str()?.into(),
                platform: Provider::Youtube,
                channel_id: channel.into(),
                author: author["displayName"].as_str()?.into(),
                author_id: author["channelId"].as_str()?.into(),
                timestamp: timestamp(snippet["publishedAt"].as_str().unwrap_or("")),
                fragments: vec![Fragment::Text { text: text.into() }],
                moderator: author["isChatModerator"].as_bool().unwrap_or(false)
                    || author["isChatOwner"].as_bool().unwrap_or(false),
            }))
        }
    }
}
async fn get(
    client: &Client,
    config: &Config,
    token: &mut Token,
    path: &str,
    params: &[(&str, &str)],
) -> Result<Value, String> {
    for attempt in 0..2 {
        auth::ensure_fresh(client, config, token, false).await?;
        let response = client
            .get(format!("https://www.googleapis.com/youtube/v3/{path}"))
            .bearer_auth(&token.access_token)
            .query(params)
            .send()
            .await
            .map_err(|_| "network_error")?;
        if response.status() == 401 && attempt == 0 {
            auth::ensure_fresh(client, config, token, true).await?;
            continue;
        }
        let code = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30);
        let data = json(response).await?;
        if code.is_success() {
            return Ok(data);
        }
        if code == 429 || code.is_server_error() {
            sleep(Duration::from_secs(retry_after.max(5))).await;
            return Err("retryable_provider_error".into());
        }
        return Err(
            match data["error"]["errors"][0]["reason"].as_str().unwrap_or("") {
                "liveChatEnded" | "liveChatNotFound" => "live_chat_ended",
                "quotaExceeded" | "dailyLimitExceeded" => "youtube_quota_exceeded",
                "liveChatDisabled" => "live_chat_disabled",
                "rateLimitExceeded" => "retryable_provider_error",
                _ if code == 401 => "reauthorization_required",
                _ => "youtube_permission_or_api_error",
            }
            .into(),
        );
    }
    Err("reauthorization_required".into())
}
/// Official REST polling fallback. Honor pollingIntervalMillis; never hard-poll.
/// Upgrade to streamList before broad distribution to reduce quota consumption.
pub async fn run(
    client: &Client,
    config: &Config,
    token: &mut Token,
    sink: &Sink,
) -> Result<(), String> {
    let mut chat_id = String::new();
    let mut page = String::new();
    let mut failures = 0u32;
    loop {
        let result = if chat_id.is_empty() {
            get(
                client,
                config,
                token,
                "liveBroadcasts",
                &[
                    ("part", "snippet"),
                    ("broadcastStatus", "active"),
                    ("maxResults", "50"),
                ],
            )
            .await
        } else {
            let mut params = vec![
                ("part", "id,snippet,authorDetails"),
                ("liveChatId", &chat_id),
                ("maxResults", "200"),
            ];
            if !page.is_empty() {
                params.push(("pageToken", &page));
            }
            get(client, config, token, "liveChat/messages", &params).await
        };
        let data = match result {
            Ok(data) => {
                failures = 0;
                data
            }
            Err(error) if error == "live_chat_ended" => {
                chat_id.clear();
                page.clear();
                continue;
            }
            Err(error) if error == "network_error" || error == "retryable_provider_error" => {
                failures += 1;
                if failures > 8 {
                    return Err("connection_retry_exhausted".into());
                }
                status(sink, Provider::Youtube, "reconnecting", "");
                sleep(Duration::from_secs(
                    (2u64.pow(failures.min(6))) + rand::random::<u8>() as u64 % 3,
                ))
                .await;
                continue;
            }
            Err(error) => return Err(error),
        };
        if chat_id.is_empty() {
            let chats = data["items"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item["snippet"]["liveChatId"].as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if chats.len() > 1 {
                return Err("multiple_broadcasts_select_required".into());
            }
            if let Some(id) = chats.first() {
                chat_id = id.to_string();
                continue;
            }
            status(sink, Provider::Youtube, "waiting_live", "");
            sleep(Duration::from_secs(30)).await;
            continue;
        }
        status(sink, Provider::Youtube, "connected", "");
        if let Some(items) = data["items"].as_array() {
            for item in items {
                if let Some(event) = normalize(item) {
                    sink(event);
                }
            }
        }
        page = data["nextPageToken"].as_str().unwrap_or("").into();
        if data.get("offlineAt").is_some() {
            chat_id.clear();
            page.clear();
        }
        sleep(Duration::from_millis(
            data["pollingIntervalMillis"]
                .as_u64()
                .unwrap_or(5000)
                .max(1000),
        ))
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_data_ignored() {
        assert!(normalize(&serde_json::json!({"id":"x"})).is_none());
    }
    #[test]
    fn normalize_plain_text_without_html_interpretation() {
        let event = normalize(&serde_json::json!({"id":"1","snippet":{"liveChatId":"c","type":"textMessageEvent","displayMessage":"<script>x</script>"},"authorDetails":{"displayName":"A","channelId":"u"}})).unwrap();
        if let Event::Message(m) = event {
            assert!(
                matches!(&m.fragments[0], Fragment::Text { text } if text == "<script>x</script>")
            );
        } else {
            panic!("message expected");
        }
    }
    #[test]
    fn normalize_deletion() {
        assert!(matches!(
            normalize(
                &serde_json::json!({"snippet":{"liveChatId":"c","type":"messageDeletedEvent","messageDeletedDetails":{"deletedMessageId":"x"}}})
            ),
            Some(Event::Remove(_))
        ));
    }
}
