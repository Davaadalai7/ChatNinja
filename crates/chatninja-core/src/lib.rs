//! Native-only authentication and chat pipeline. Never expose Token to IPC.
pub mod auth;
pub mod model;
pub mod providers;
pub mod vault;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Youtube,
    Twitch,
    Kick,
}
impl Provider {
    pub fn name(self) -> &'static str {
        match self {
            Self::Youtube => "youtube",
            Self::Twitch => "twitch",
            Self::Kick => "kick",
        }
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub provider: Provider,
    pub state: String,
    pub detail: String,
}

#[derive(Clone)]
pub enum Event {
    Status(Status),
    Message(model::ChatMessage),
    Remove(model::Removal),
}
pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;
pub fn status(sink: &Sink, provider: Provider, state: &str, detail: &str) {
    sink(Event::Status(Status {
        provider,
        state: state.into(),
        detail: detail.into(),
    }));
}

#[derive(Clone)]
pub struct Config {
    pub provider: Provider,
    pub client_id: String,
    pub desktop_secret: Option<String>,
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.provider == Provider::Kick {
            let url = url::Url::parse(&self.client_id).map_err(|_| "kick_backend_required")?;
            if url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err("invalid_kick_relay_origin".into());
            }
            return Ok(());
        }
        if self.client_id.trim().is_empty() {
            return Err("developer_app_required".into());
        }
        Ok(())
    }
}

pub fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "http_client_failed".into())
}

/// A cancelled future drops the socket/listener. Caller must abort + await before logout.
pub async fn run(config: Config, sink: Sink) -> Result<(), String> {
    config.validate()?;
    let client = http_client()?;
    status(&sink, config.provider, "authorizing", "");
    let mut token = match vault::load(&config)? {
        Some(token) => token,
        None => {
            let token = auth::authorize(&client, &config, &sink).await?;
            vault::save(&config, &token)?;
            token
        }
    };
    auth::ensure_fresh(&client, &config, &mut token, false).await?;
    status(&sink, config.provider, "authenticated", "");
    match config.provider {
        Provider::Youtube => providers::youtube::run(&client, &config, &mut token, &sink).await,
        Provider::Twitch => providers::twitch::run(&client, &config, &mut token, &sink).await,
        Provider::Kick => providers::kick::run(&client, &config, &token, &sink).await,
    }
}
