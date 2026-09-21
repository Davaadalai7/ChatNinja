use crate::{auth::Token, Config};

// Credential Manager only. Unsupported OS deliberately has no plaintext fallback.
#[cfg(windows)]
fn entry(config: &Config) -> Result<keyring::Entry, String> {
    keyring::Entry::new(
        "app.chatninja.oauth",
        &format!("{}:{}", config.provider.name(), config.client_id),
    )
    .map_err(|_| "vault_unavailable".into())
}
#[cfg(windows)]
pub fn load(config: &Config) -> Result<Option<Token>, String> {
    match entry(config)?.get_password() {
        Ok(value) => serde_json::from_str(&value)
            .map(Some)
            .map_err(|_| "vault_data_invalid".into()),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("vault_read_failed".into()),
    }
}
#[cfg(windows)]
pub fn save(config: &Config, token: &Token) -> Result<(), String> {
    let value = serde_json::to_string(token).map_err(|_| "vault_data_invalid")?;
    entry(config)?
        .set_password(&value)
        .map_err(|_| "vault_write_failed".into())
}
#[cfg(windows)]
pub fn delete(config: &Config) -> Result<(), String> {
    match entry(config)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("vault_delete_failed".into()),
    }
}
#[cfg(not(windows))]
pub fn load(_: &Config) -> Result<Option<Token>, String> {
    Err("windows_credential_manager_required".into())
}
#[cfg(not(windows))]
pub fn save(_: &Config, _: &Token) -> Result<(), String> {
    Err("windows_credential_manager_required".into())
}
#[cfg(not(windows))]
pub fn delete(_: &Config) -> Result<(), String> {
    Err("windows_credential_manager_required".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::Provider;

    #[test]
    fn credential_manager_roundtrip_and_logout() {
        // A unique test-only target; never read or overwrite an actual account.
        let config = Config {
            provider: Provider::Twitch,
            client_id: format!("test-{}-{}", std::process::id(), rand::random::<u64>()),
            desktop_secret: None,
        };
        let token = Token {
            access_token: "test-access-token".into(),
            refresh_token: Some("test-refresh-token".into()),
            expires_at: 123456789,
        };
        assert!(load(&config).unwrap().is_none());
        save(&config, &token).unwrap();
        let loaded = load(&config);
        // Clean up before asserting fields, including when the read fails.
        let deleted = delete(&config);
        let loaded = loaded.unwrap().unwrap();
        deleted.unwrap();
        assert_eq!(loaded.access_token, token.access_token);
        assert_eq!(loaded.refresh_token, token.refresh_token);
        assert_eq!(loaded.expires_at, token.expires_at);
        assert!(load(&config).unwrap().is_none());
        delete(&config).unwrap(); // Repeated logout is harmless.
    }
}
