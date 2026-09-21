use crate::{status, vault, Config, Provider, Sink};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{sleep, timeout, Instant},
};

/// Not Debug: token values must never enter logs, UI, OBS or error strings.
#[derive(Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
pub fn required<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "invalid_provider_response".into())
}
pub async fn json(response: reqwest::Response) -> Result<Value, String> {
    if response.content_length().unwrap_or(0) > 2_000_000 {
        return Err("provider_response_too_large".into());
    }
    let mut response = response;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "network_error")? {
        if body.len() + chunk.len() > 2_000_000 {
            return Err("provider_response_too_large".into());
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| "invalid_provider_response".into())
}
pub fn token_from(value: &Value, previous: Option<&Token>) -> Result<Token, String> {
    let access_token = required(value, "access_token")?.to_string();
    let expires_in = value["expires_in"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("invalid_token_expiry")?;
    Ok(Token {
        access_token,
        refresh_token: value["refresh_token"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| previous.and_then(|t| t.refresh_token.clone())),
        expires_at: now().saturating_add(expires_in),
    })
}
fn endpoint(provider: Provider) -> Result<&'static str, String> {
    match provider {
        Provider::Youtube => Ok("https://oauth2.googleapis.com/token"),
        Provider::Twitch => Ok("https://id.twitch.tv/oauth2/token"),
        Provider::Kick => Err("kick_backend_required".into()),
    }
}
pub async fn authorize(client: &Client, config: &Config, sink: &Sink) -> Result<Token, String> {
    match config.provider {
        Provider::Youtube => google(client, config).await,
        Provider::Twitch => twitch_device(client, config, sink).await,
        Provider::Kick => crate::providers::kick::authorize(client, config).await,
    }
}

/// Returns None for unrelated requests. Invalid state never consumes the pending login.
fn callback_code(target: &str, expected_state: &str) -> Result<Option<String>, String> {
    if !target.starts_with('/') || target.starts_with("//") {
        return Ok(None);
    }
    let url =
        url::Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| "invalid_callback")?;
    if url.path() != "/oauth/callback" {
        return Ok(None);
    }
    let values = |key: &str| {
        url.query_pairs()
            .filter(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
            .collect::<Vec<_>>()
    };
    let states = values("state");
    if states.len() != 1 || states[0] != expected_state {
        return Ok(None);
    }
    if !values("error").is_empty() {
        return Err("authorization_denied".into());
    }
    let codes = values("code");
    if codes.len() != 1 || codes[0].is_empty() || codes[0].len() > 4096 {
        return Err("invalid_callback".into());
    }
    Ok(Some(codes[0].clone()))
}
async fn google(client: &Client, config: &Config) -> Result<Token, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "callback_port_unavailable")?;
    let redirect = format!(
        "http://127.0.0.1:{}/oauth/callback",
        listener
            .local_addr()
            .map_err(|_| "callback_port_unavailable")?
            .port()
    );
    let state = random_secret();
    let verifier = random_secret();
    let mut url = url::Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut().extend_pairs([
        ("client_id", config.client_id.as_str()),
        ("redirect_uri", &redirect),
        ("response_type", "code"),
        ("scope", "https://www.googleapis.com/auth/youtube.readonly"),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("state", &state),
        ("code_challenge", &challenge(&verifier)),
        ("code_challenge_method", "S256"),
    ]);
    webbrowser::open(url.as_str()).map_err(|_| "browser_open_failed")?;
    let code = timeout(Duration::from_secs(300), async {
        loop {
            let (mut socket, _) = listener.accept().await.map_err(|_| "callback_failed")?;
            let mut request = Vec::new();
            let read = timeout(Duration::from_secs(3), async {
                let mut buffer = [0; 1024];
                loop { let n = socket.read(&mut buffer).await?; if n == 0 { break; } request.extend_from_slice(&buffer[..n]); if request.len() > 8192 || request.windows(4).any(|w| w == b"\r\n\r\n") { break; } }
                Ok::<_, std::io::Error>(())
            }).await;
            if !matches!(read, Ok(Ok(()))) || request.len() > 8192 { continue; }
            let text = String::from_utf8_lossy(&request);
            let parts = text.lines().next().unwrap_or("").split_whitespace().collect::<Vec<_>>();
            let outcome = if parts.len() == 3 && parts[0] == "GET" { callback_code(parts[1], &state) } else { Ok(None) };
            let accepted = matches!(outcome, Ok(Some(_)));
            let body = if accepted { "Authorization received. Return to ChatNinja." } else { "Authorization not accepted. Return to ChatNinja." };
            let response = format!("HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}", if accepted { "200 OK" } else { "400 Bad Request" }, body.len(), body);
            let _ = timeout(Duration::from_secs(2), socket.write_all(response.as_bytes())).await;
            match outcome { Ok(Some(code)) => return Ok(code), Ok(None) => continue, Err(error) => return Err(error) }
        }
    }).await.map_err(|_| "authorization_timeout")??;
    let mut form = vec![
        ("client_id", config.client_id.clone()),
        ("code", code),
        ("redirect_uri", redirect),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code".into()),
    ];
    if let Some(secret) = &config.desktop_secret {
        form.push(("client_secret", secret.clone()));
    }
    let response = client
        .post(endpoint(config.provider)?)
        .form(&form)
        .send()
        .await
        .map_err(|_| "network_error")?;
    if !response.status().is_success() {
        return Err("authorization_exchange_failed".into());
    }
    token_from(&json(response).await?, None)
}
async fn twitch_device(client: &Client, config: &Config, sink: &Sink) -> Result<Token, String> {
    let response = client
        .post("https://id.twitch.tv/oauth2/device")
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("scopes", "user:read:chat"),
        ])
        .send()
        .await
        .map_err(|_| "network_error")?;
    if !response.status().is_success() {
        return Err("device_authorization_failed".into());
    }
    let device = json(response).await?;
    let code = required(&device, "device_code")?;
    let user_code = required(&device, "user_code")?;
    let verification = url::Url::parse(required(&device, "verification_uri")?)
        .map_err(|_| "invalid_provider_response")?;
    if verification.scheme() != "https"
        || verification.host_str() != Some("www.twitch.tv")
        || verification.path() != "/activate"
        || !verification.username().is_empty()
        || verification.password().is_some()
    {
        return Err("invalid_verification_url".into());
    }
    status(sink, Provider::Twitch, "authorizing", user_code);
    webbrowser::open(verification.as_str()).map_err(|_| "browser_open_failed")?;
    let expires = device["expires_in"].as_u64().unwrap_or(300).min(1800);
    let deadline = Instant::now() + Duration::from_secs(expires);
    let mut interval = device["interval"].as_u64().unwrap_or(5).max(5);
    while Instant::now() < deadline {
        sleep(Duration::from_secs(interval)).await;
        let response = client
            .post(endpoint(config.provider)?)
            .form(&[
                ("client_id", config.client_id.as_str()),
                ("device_code", code),
                ("scopes", "user:read:chat"),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await
            .map_err(|_| "network_error")?;
        let ok = response.status().is_success();
        let data = json(response).await?;
        if ok {
            return token_from(&data, None);
        }
        match data["message"]
            .as_str()
            .or(data["error"].as_str())
            .unwrap_or("")
        {
            "authorization_pending" => {}
            "slow_down" => interval = interval.saturating_add(5),
            _ => return Err("authorization_denied_or_expired".into()),
        }
    }
    Err("authorization_timeout".into())
}

pub async fn ensure_fresh(
    client: &Client,
    config: &Config,
    token: &mut Token,
    force: bool,
) -> Result<(), String> {
    if !force && token.expires_at > now() + 120 {
        return Ok(());
    }
    if config.provider == Provider::Kick {
        return Err("reauthorization_required".into());
    }
    let refresh = token
        .refresh_token
        .as_ref()
        .ok_or("reauthorization_required")?;
    let mut form = vec![
        ("client_id", config.client_id.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh.as_str()),
    ];
    if let Some(secret) = &config.desktop_secret {
        form.push(("client_secret", secret.as_str()));
    }
    let response = client
        .post(endpoint(config.provider)?)
        .form(&form)
        .send()
        .await
        .map_err(|_| "network_error")?;
    if !response.status().is_success() {
        return Err("reauthorization_required".into());
    }
    let next = token_from(&json(response).await?, Some(token))?;
    // Persist rotating refresh token before making further requests; stop if storage fails.
    vault::save(config, &next)?;
    *token = next;
    Ok(())
}
pub async fn revoke(client: &Client, config: &Config, token: &Token) -> Result<(), String> {
    let response = match config.provider {
        Provider::Youtube => client.post("https://oauth2.googleapis.com/revoke").form(&[(
            "token",
            token
                .refresh_token
                .as_deref()
                .unwrap_or(&token.access_token),
        )]),
        Provider::Twitch => client.post("https://id.twitch.tv/oauth2/revoke").form(&[
            ("client_id", config.client_id.as_str()),
            ("token", token.access_token.as_str()),
        ]),
        Provider::Kick => {
            let response = client
                .post(crate::providers::kick::relay_url(config, "/logout")?)
                .bearer_auth(&token.access_token)
                .send()
                .await
                .map_err(|_| "remote_revocation_failed")?;
            if !response.status().is_success() || json(response).await?["remoteRevoked"] != true {
                return Err("remote_revocation_failed".into());
            }
            return Ok(());
        }
    }
    .send()
    .await
    .map_err(|_| "remote_revocation_failed")?;
    if !response.status().is_success() {
        return Err("remote_revocation_failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }
    #[test]
    fn callback_rejects_wrong_state_and_duplicate_code() {
        assert!(callback_code("/oauth/callback?code=x&state=bad", "good")
            .unwrap()
            .is_none());
        assert!(callback_code("/oauth/callback?code=x&code=y&state=good", "good").is_err());
        assert!(
            callback_code("//evil.test/oauth/callback?code=x&state=good", "good")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn callback_accepts_only_matching_path_and_state() {
        assert_eq!(
            callback_code("/oauth/callback?code=ok&state=good", "good").unwrap(),
            Some("ok".into())
        );
        assert!(callback_code("/other?code=ok&state=good", "good")
            .unwrap()
            .is_none());
        assert!(callback_code("/oauth/callback?error=access_denied&state=good", "good").is_err());
    }
    #[test]
    fn refresh_preserves_google_refresh_token_if_omitted() {
        let old = Token {
            access_token: "old".into(),
            refresh_token: Some("retained".into()),
            expires_at: 1,
        };
        let next = token_from(
            &serde_json::json!({"access_token":"new","expires_in":3600}),
            Some(&old),
        )
        .unwrap();
        assert_eq!(next.refresh_token.as_deref(), Some("retained"));
    }
    #[test]
    fn refresh_replaces_rotating_token() {
        let old = Token {
            access_token: "old".into(),
            refresh_token: Some("old-refresh".into()),
            expires_at: 1,
        };
        let next = token_from(
            &serde_json::json!({"access_token":"new","refresh_token":"rotated","expires_in":3600}),
            Some(&old),
        )
        .unwrap();
        assert_eq!(next.refresh_token.as_deref(), Some("rotated"));
    }
    #[test]
    fn invalid_token_response_rejected() {
        assert!(token_from(&serde_json::json!({"access_token":"","expires_in":0}), None).is_err());
    }
}
