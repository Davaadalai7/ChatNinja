use crate::{lifecycle, state::AppState};
use jutsu_config::Config;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    config: Config,
    version: String,
    config_path: String,
    logs_path: String,
    warning: Option<String>,
    tray_available: bool,
}
#[tauri::command]
pub async fn bootstrap(app: tauri::AppHandle) -> Result<Bootstrap, String> {
    log::info!("Settings frontend reached native bootstrap");
    let state = app.state::<AppState>();
    if let Some(error) = &state.startup_error {
        return Err(error.clone());
    }
    let config = state
        .config
        .lock()
        .map_err(|_| "Configuration lock failed")?
        .clone()
        .ok_or("Configuration unavailable")?;
    Ok(Bootstrap {
        config,
        version: app.package_info().version.to_string(),
        config_path: state.store.path().display().to_string(),
        logs_path: state.logs.display().to_string(),
        warning: state.warning.clone(),
        tray_available: state.tray_available.load(Ordering::SeqCst),
    })
}
#[tauri::command]
pub async fn save_settings(app: tauri::AppHandle, config: Config) -> Result<Config, String> {
    let state = app.state::<AppState>();
    if let Some(error) = &state.startup_error {
        return Err(error.clone());
    }
    config.validate()?;
    if config.app.close_to_tray && !state.tray_available.load(Ordering::SeqCst) {
        return Err("Tray unavailable. Close-to-tray cannot be enabled.".into());
    }
    let mut current = state
        .config
        .lock()
        .map_err(|_| "Configuration lock failed")?;
    state.store.save(&config).inspect_err(|error| {
        log::error!("Config save failed: {error}");
    })?;
    *current = Some(config.clone());
    log::info!("Non-secret preferences saved");
    Ok(config)
}
#[tauri::command]
pub async fn reset_settings(app: tauri::AppHandle) -> Result<Config, String> {
    save_settings(app, Config::default()).await
}
#[tauri::command]
pub async fn open_logs_folder(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    std::fs::create_dir_all(&state.logs).map_err(|e| e.to_string())?;
    // Only the app's own directory is accepted; the webview cannot launch
    // arbitrary programs or supply a shell command/path.
    #[cfg(target_os = "windows")]
    let command = "explorer.exe";
    #[cfg(target_os = "macos")]
    let command = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let command = "xdg-open";
    let mut child = std::process::Command::new(command)
        .arg(&state.logs)
        .spawn()
        .map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
#[tauri::command]
pub async fn quit_app(app: tauri::AppHandle) {
    lifecycle::quit(&app);
}
