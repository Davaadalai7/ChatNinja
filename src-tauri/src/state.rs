use jutsu_config::{Config, Store};
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Mutex},
};

pub struct AppState {
    pub config: Mutex<Option<Config>>,
    pub store: Store,
    pub logs: PathBuf,
    pub warning: Option<String>,
    pub startup_error: Option<String>,
    pub tray_available: AtomicBool,
    pub quitting: AtomicBool,
}
