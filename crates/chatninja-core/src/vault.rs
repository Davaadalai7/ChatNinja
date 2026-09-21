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
