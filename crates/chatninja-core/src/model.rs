use crate::Provider;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fragment {
    Text {
        text: String,
    },
    Emote {
        id: String,
        name: String,
        url: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub platform: Provider,
    pub channel_id: String,
    pub author: String,
    pub author_id: String,
    pub timestamp: i64,
    pub fragments: Vec<Fragment>,
    pub moderator: bool,
}
#[derive(Clone)]
pub struct Removal {
    pub platform: Provider,
    pub channel_id: String,
    pub message_id: Option<String>,
    pub author_id: Option<String>,
}

#[derive(Default)]
pub struct ChatBuffer {
    messages: VecDeque<ChatMessage>,
    seen: VecDeque<String>,
}
impl ChatBuffer {
    pub fn push(&mut self, message: ChatMessage) {
        let key = format!(
            "{}:{}:{}",
            message.platform.name(),
            message.channel_id,
            message.id
        );
        if self.seen.contains(&key) {
            return;
        }
        self.seen.push_back(key);
        if self.seen.len() > 2000 {
            self.seen.pop_front();
        }
        self.messages.push_back(message);
        if self.messages.len() > 200 {
            self.messages.pop_front();
        }
    }
    pub fn remove(&mut self, removal: &Removal) {
        self.messages.retain(|m| {
            !(m.platform == removal.platform
                && m.channel_id == removal.channel_id
                && removal.message_id.as_ref().is_none_or(|id| &m.id == id)
                && removal
                    .author_id
                    .as_ref()
                    .is_none_or(|id| &m.author_id == id))
        });
    }
    pub fn clear_provider(&mut self, provider: Provider) {
        self.messages.retain(|m| m.platform != provider);
        self.seen
            .retain(|k| !k.starts_with(&format!("{}:", provider.name())));
    }
    pub fn snapshot(&self) -> Vec<ChatMessage> {
        self.messages.iter().cloned().collect()
    }
}

pub fn timestamp(value: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| t.timestamp_millis())
        .unwrap_or_else(|_| chrono::Utc::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn message(id: &str, platform: Provider) -> ChatMessage {
        ChatMessage {
            id: id.into(),
            platform,
            channel_id: "channel".into(),
            author: "A".into(),
            author_id: "u".into(),
            timestamp: 0,
            fragments: vec![],
            moderator: false,
        }
    }
    #[test]
    fn provider_ids_do_not_collide() {
        let mut b = ChatBuffer::default();
        b.push(message("1", Provider::Youtube));
        b.push(message("1", Provider::Twitch));
        b.push(message("1", Provider::Youtube));
        assert_eq!(b.snapshot().len(), 2);
    }
    #[test]
    fn removal_does_not_resurrect_duplicate() {
        let mut b = ChatBuffer::default();
        b.push(message("1", Provider::Twitch));
        b.remove(&Removal {
            platform: Provider::Twitch,
            channel_id: "channel".into(),
            message_id: Some("1".into()),
            author_id: None,
        });
        b.push(message("1", Provider::Twitch));
        assert!(b.snapshot().is_empty());
    }
    #[test]
    fn bounded_history() {
        let mut b = ChatBuffer::default();
        for i in 0..3000 {
            b.push(message(&i.to_string(), Provider::Youtube));
        }
        assert_eq!(b.snapshot().len(), 200);
        assert_eq!(b.seen.len(), 2000);
    }
    #[test]
    fn clear_user_scoped_to_channel_and_platform() {
        let mut b = ChatBuffer::default();
        b.push(message("1", Provider::Youtube));
        b.push(message("2", Provider::Twitch));
        b.remove(&Removal {
            platform: Provider::Twitch,
            channel_id: "channel".into(),
            message_id: None,
            author_id: Some("u".into()),
        });
        assert_eq!(b.snapshot().len(), 1);
    }
}
