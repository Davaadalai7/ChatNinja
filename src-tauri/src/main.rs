#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod commands;
mod lifecycle;
mod state;
use jutsu_config::Store;
use state::AppState;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::Manager;

fn main() {
    // Register single-instance first: a second process must not acquire resources.
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            log::info!("Second launch focused the existing instance");
            lifecycle::show_settings(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("jutsu".into()),
                    },
                )])
                .build(),
        )
        .setup(|app| {
            let store = Store::new(app.path().app_config_dir()?.join("config.json"));
            let logs = app.path().app_log_dir()?;
            std::fs::create_dir_all(&logs)?;
            let (config, warning, startup_error) = match store.load() {
                Ok(loaded) => (Some(loaded.config), loaded.warning, None),
                Err(error) => {
                    log::error!("Configuration startup error: {error}");
                    (None, None, Some(error))
                }
            };
            if let Some(ref message) = warning {
                log::warn!("{message}");
            }
            app.manage(AppState {
                config: Mutex::new(config),
                store,
                logs,
                warning,
                startup_error,
                tray_available: AtomicBool::new(false),
                quitting: AtomicBool::new(false),
            });
            match lifecycle::install_tray(app.handle()) {
                Ok(()) => {
                    app.state::<AppState>()
                        .tray_available
                        .store(true, Ordering::SeqCst);
                }
                Err(error) => log::error!("Tray unavailable; closing settings will quit: {error}"),
            }
            log::info!(
                "Jutsu {} started; pid={}",
                app.package_info().version,
                std::process::id()
            );
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "settings" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let state = app.state::<AppState>();
                if state.quitting.load(Ordering::SeqCst) {
                    return;
                }
                let hide = state.tray_available.load(Ordering::SeqCst)
                    && state
                        .config
                        .lock()
                        .ok()
                        .and_then(|c| c.as_ref().map(|c| c.app.close_to_tray))
                        .unwrap_or(false);
                if hide {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        log::error!("Hide failed: {error}");
                        lifecycle::quit(app);
                    }
                } else {
                    lifecycle::quit(app);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::save_settings,
            commands::reset_settings,
            commands::open_logs_folder,
            commands::quit_app
        ])
        .build(tauri::generate_context!())
        .expect("Jutsu startup failed; check the app logs and WebView2 installation");
    app.run(|_, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            log::info!("Desktop event loop exited");
            log::logger().flush();
        }
    });
}
