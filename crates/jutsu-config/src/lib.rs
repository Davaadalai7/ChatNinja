//! Versioned, non-secret configuration. No platform credentials belong here.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_BYTES: u64 = 65_536;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Mn,
    En,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppSettings {
    pub language: Language,
    pub close_to_tray: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub app: AppSettings,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            app: AppSettings {
                language: Language::Mn,
                close_to_tray: false,
            },
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported configuration version".into());
        }
        Ok(())
    }
}
pub struct Loaded {
    pub config: Config,
    pub warning: Option<String>,
}
pub struct Store {
    path: PathBuf,
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Recover an interrupted replace before reading the primary file.
    pub fn load(&self) -> Result<Loaded, String> {
        let backup = self.path.with_extension("json.bak");
        if !self.path.exists() && backup.exists() {
            fs::rename(&backup, &self.path).map_err(|e| e.to_string())?;
        }
        if !self.path.exists() {
            let config = Config::default();
            self.save(&config)?;
            return Ok(Loaded {
                config,
                warning: None,
            });
        }
        let file = File::open(&self.path).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        file.take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let value = if bytes.len() as u64 > MAX_BYTES {
            None
        } else {
            serde_json::from_slice::<serde_json::Value>(&bytes).ok()
        };
        // A downgrade must not overwrite settings from a newer app version.
        if value
            .as_ref()
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_u64())
            .is_some_and(|v| v > 1)
        {
            return Err("This config belongs to a newer Jutsu version. Upgrade Jutsu; the file has been preserved.".into());
        }
        if let Some(value) = value {
            if value.get("version").and_then(|v| v.as_u64()) == Some(0) {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct V0 {
                    version: u32,
                    language: Language,
                }
                if let Ok(old) = serde_json::from_value::<V0>(value.clone()) {
                    if old.version == 0 {
                        let mut config = Config::default();
                        config.app.language = old.language;
                        self.save(&config)?;
                        return Ok(Loaded {
                            config,
                            warning: Some("Configuration migrated from v0 to v1.".into()),
                        });
                    }
                }
            }
            if let Ok(config) = serde_json::from_value::<Config>(value) {
                if config.validate().is_ok() {
                    return Ok(Loaded {
                        config,
                        warning: None,
                    });
                }
            }
        }
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let quarantine = self.path.with_extension(format!("corrupt-{stamp}.json"));
        fs::rename(&self.path, &quarantine).map_err(|e| e.to_string())?;
        let config = Config::default();
        self.save(&config)?;
        Ok(Loaded {
            config,
            warning: Some(format!(
                "Invalid config preserved as {}; defaults restored.",
                quarantine.file_name().unwrap_or_default().to_string_lossy()
            )),
        })
    }
    /// Windows-safe replace with a synced staging file and recoverable backup.
    /// Caller serializes writers with a mutex; never publish partially written JSON.
    pub fn save(&self, config: &Config) -> Result<(), String> {
        config.validate()?;
        let parent = self.path.parent().ok_or("Config path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("json.tmp");
        let backup = self.path.with_extension("json.bak");
        let bytes = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
        let mut file = File::create(&temp).map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        if self.path.exists() {
            if backup.exists() {
                fs::remove_file(&backup).map_err(|e| e.to_string())?;
            }
            fs::rename(&self.path, &backup).map_err(|e| e.to_string())?;
        }
        if let Err(error) = fs::rename(&temp, &self.path) {
            if backup.exists() {
                let _ = fs::rename(&backup, &self.path);
            }
            return Err(error.to_string());
        }
        // Keep the last known-good backup until the next successful replacement.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let n = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p = std::env::temp_dir().join(format!("jutsu-{}-{n}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn store(&self) -> Store {
            Store::new(self.0.join("config.json"))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn first_run_and_restart() {
        let f = Fixture::new();
        let s = f.store();
        let mut c = s.load().unwrap().config;
        c.app.language = Language::En;
        c.app.close_to_tray = true;
        s.save(&c).unwrap();
        assert_eq!(s.load().unwrap().config, c);
    }
    #[test]
    fn corruption_is_preserved() {
        let f = Fixture::new();
        let s = f.store();
        fs::write(s.path(), b"broken").unwrap();
        let l = s.load().unwrap();
        assert!(l.warning.is_some());
        assert_eq!(l.config, Config::default());
        assert!(fs::read_dir(&f.0).unwrap().any(|p| p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("corrupt-")));
    }
    #[test]
    fn interrupted_replace_recovers_backup() {
        let f = Fixture::new();
        let s = f.store();
        s.save(&Config::default()).unwrap();
        fs::rename(s.path(), s.path().with_extension("json.bak")).unwrap();
        assert_eq!(s.load().unwrap().config, Config::default());
    }
    #[test]
    fn migrates_v0() {
        let f = Fixture::new();
        let s = f.store();
        fs::write(s.path(), br#"{"version":0,"language":"en"}"#).unwrap();
        assert_eq!(s.load().unwrap().config.app.language, Language::En);
    }
    #[test]
    fn future_version_is_never_destroyed() {
        let f = Fixture::new();
        let s = f.store();
        let bytes = br#"{"version":99,"future":true}"#;
        fs::write(s.path(), bytes).unwrap();
        assert!(s.load().is_err());
        assert_eq!(fs::read(s.path()).unwrap(), bytes);
    }
    #[test]
    fn invalid_fields_reset() {
        let f = Fixture::new();
        let s = f.store();
        fs::write(
            s.path(),
            br#"{"version":1,"app":{"language":"invalid","closeToTray":false}}"#,
        )
        .unwrap();
        assert!(s.load().unwrap().warning.is_some());
    }
    #[test]
    fn oversized_config_recovers() {
        let f = Fixture::new();
        let s = f.store();
        fs::write(s.path(), vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(s.load().unwrap().warning.is_some());
    }
    #[test]
    fn save_rejects_bad_version() {
        let f = Fixture::new();
        let s = f.store();
        let c = Config {
            version: 2,
            ..Config::default()
        };
        assert!(s.save(&c).is_err());
        assert!(!s.path().exists());
    }
}
