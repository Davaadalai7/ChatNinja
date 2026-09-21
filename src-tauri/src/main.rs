#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod connections;

use axum::{extract::State as HttpState, http::header, response::Html, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    version: u8,
    language: String,
    font_size: u32,
    opacity: f64,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    visibility: String,
    click_through: bool,
    timestamp: bool,
    hide_bots: bool,
    bot_names: String,
    platform_labels: bool,
    enabled_platforms: Vec<String>,
    demo: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 1,
            language: "mn".into(),
            font_size: 17,
            opacity: 0.65,
            width: 360,
            height: 480,
            x: 40,
            y: 80,
            visibility: "streamer".into(),
            click_through: false,
            timestamp: false,
            hide_bots: true,
            bot_names: "nightbot, kicklet, botrix".into(),
            platform_labels: true,
            enabled_platforms: vec!["youtube".into(), "kick".into(), "twitch".into()],
            demo: true,
        }
    }
}
impl Settings {
    fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || !["mn", "en"].contains(&self.language.as_str())
            || !(12..=32).contains(&self.font_size)
            || !self.opacity.is_finite()
            || !(0.0..=1.0).contains(&self.opacity)
            || !(160..=1600).contains(&self.width)
            || !(100..=1600).contains(&self.height)
            || !(-32000..=32000).contains(&self.x)
            || !(-32000..=32000).contains(&self.y)
            || !["streamer", "obs", "both"].contains(&self.visibility.as_str())
            || self.bot_names.len() > 4000
            || self.enabled_platforms.len() > 3
            || self
                .enabled_platforms
                .iter()
                .any(|p| !["youtube", "kick", "twitch"].contains(&p.as_str()))
        {
            return Err("Invalid settings".into());
        }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Snapshot {
    settings: Settings,
    messages: Vec<serde_json::Value>,
}
type Shared = Arc<Mutex<Snapshot>>;
struct AppState {
    snapshot: Shared,
    obs_url: String,
    startup_warnings: Vec<String>,
}

#[tauri::command]
fn get_snapshot(state: tauri::State<AppState>) -> Result<Snapshot, String> {
    state
        .snapshot
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "State unavailable".into())
}
#[tauri::command]
fn get_obs_url(state: tauri::State<AppState>) -> String {
    state.obs_url.clone()
}

#[tauri::command]
fn get_startup_warnings(state: tauri::State<AppState>) -> Vec<String> {
    state.startup_warnings.clone()
}

#[tauri::command]
fn set_snapshot(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    mut snapshot: Snapshot,
) -> Result<(), String> {
    snapshot.settings.validate()?;
    if snapshot.messages.len() > 200
        || serde_json::to_vec(&snapshot)
            .map_err(|e| e.to_string())?
            .len()
            > 1_000_000
    {
        return Err("Message buffer exceeded".into());
    }
    if let Some(window) = app.get_webview_window("overlay") {
        // Window manager and capture APIs can differ by Windows version. A failure
        // here must not discard validated chat state or prevent overlay display.
        let _ = window.set_ignore_cursor_events(snapshot.settings.click_through);
        let _ = window.set_content_protected(snapshot.settings.visibility == "streamer");
        if snapshot.settings.visibility == "obs" {
            window.hide().map_err(|e| e.to_string())?;
        }
    }
    // The native buffer owns live messages. Renderer snapshots cannot overwrite it.
    if !snapshot.settings.demo {
        let connections = app.state::<Arc<connections::Connections>>();
        snapshot.messages = connections
            .buffer
            .lock()
            .map_err(|_| "State unavailable")?
            .snapshot()
            .iter()
            .filter_map(|m| serde_json::to_value(m).ok())
            .collect();
    }
    *state.snapshot.lock().map_err(|_| "State unavailable")? = snapshot;
    Ok(())
}

#[tauri::command]
fn control_overlay(
    app: tauri::AppHandle,
    action: String,
    settings: Settings,
) -> Result<(), String> {
    settings.validate()?;
    if !["show", "hide", "apply"].contains(&action.as_str()) {
        return Err("Invalid action".into());
    }
    if action == "hide" {
        if let Some(window) = app.get_webview_window("overlay") {
            window.hide().map_err(|e| e.to_string())?;
        }
        let _ = app.emit_to("main", "overlay-visibility", false);
        return Ok(());
    }
    if action == "show" && settings.visibility == "obs" {
        return Err("Desktop overlay disabled in OBS-only mode".into());
    }
    let window = match app.get_webview_window("overlay") {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            &app,
            "overlay",
            WebviewUrl::App("index.html?overlay=1".into()),
        )
        .title("ChatNinja Overlay")
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .inner_size(settings.width as f64, settings.height as f64)
        .min_inner_size(160.0, 100.0)
        .max_inner_size(1600.0, 1600.0)
        .build()
        .map_err(|e| e.to_string())?,
    };
    window
        .set_size(tauri::LogicalSize::new(settings.width, settings.height))
        .map_err(|e| e.to_string())?;
    let (x, y) = visible_position(&app, &settings);
    window
        .set_position(tauri::LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    if action == "show" {
        window.show().map_err(|e| e.to_string())?;
        let _ = app.emit_to("main", "overlay-visibility", true);
    }
    let _ = window.set_ignore_cursor_events(settings.click_through);
    // Capture exclusion is optional and must not block the overlay from opening.
    let _ = window.set_content_protected(settings.visibility == "streamer");
    Ok(())
}

fn visible_position(app: &tauri::AppHandle, settings: &Settings) -> (i32, i32) {
    if let Ok(monitors) = app.available_monitors() {
        let intersects = monitors.iter().any(|monitor| {
            let scale = monitor.scale_factor();
            let corner = monitor.position().to_logical::<i32>(scale);
            let size = monitor.size().to_logical::<u32>(scale);
            settings.x < corner.x.saturating_add(size.width as i32 - 48)
                && settings.x.saturating_add(settings.width as i32) > corner.x + 48
                && settings.y < corner.y.saturating_add(size.height as i32 - 32)
                && settings.y.saturating_add(settings.height as i32) > corner.y + 32
        });
        if intersects {
            return (settings.x, settings.y);
        }
    }
    let anchor = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.position().to_logical::<i32>(monitor.scale_factor()));
    anchor
        .map(|position| (position.x + 40, position.y + 80))
        .unwrap_or((40, 80))
}

async fn obs_snapshot(HttpState(shared): HttpState<Shared>) -> Json<Snapshot> {
    let mut snapshot = shared.lock().map(|s| s.clone()).unwrap_or_default();
    if snapshot.settings.visibility == "streamer" {
        snapshot.messages.clear();
    }
    Json(snapshot)
}

fn main() {
    let snapshot: Shared = Arc::new(Mutex::new(Snapshot::default()));
    let show = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyO);
    let lock = Shortcut::new(Some(Modifiers::ALT | Modifiers::SHIFT), Code::KeyL);
    tauri::Builder::default()
        .manage(Arc::new(connections::Connections::default()))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let state = app.state::<AppState>();
                    let settings = match state.snapshot.lock() {
                        Ok(snapshot) => snapshot.settings.clone(),
                        Err(_) => return,
                    };
                    if settings.visibility == "obs" {
                        return;
                    }
                    if shortcut == &show {
                        let visible = app
                            .get_webview_window("overlay")
                            .and_then(|window| window.is_visible().ok())
                            .unwrap_or(false);
                        let action = if visible { "hide" } else { "show" };
                        if let Err(error) = control_overlay(app.clone(), action.into(), settings) {
                            let _ = app.emit_to("main", "overlay-failure", error);
                        }
                    } else if shortcut == &lock {
                        if app.get_webview_window("overlay").is_none() {
                            if let Err(error) =
                                control_overlay(app.clone(), "show".into(), settings)
                            {
                                let _ = app.emit_to("main", "overlay-failure", error);
                                return;
                            }
                        }
                        if let Some(window) = app.get_webview_window("overlay") {
                            let state = app.state::<AppState>();
                            if let Ok(mut snapshot) = state.snapshot.lock() {
                                let next = !snapshot.settings.click_through;
                                if let Err(error) = window.set_ignore_cursor_events(next) {
                                    let _ = app.emit_to(
                                        "main",
                                        "overlay-failure",
                                        error.to_string(),
                                    );
                                } else {
                                    snapshot.settings.click_through = next;
                                    let _ = app.emit_to("main", "overlay-lock", next);
                                }
                            };
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
            let port = listener.local_addr()?.port();
            listener.set_nonblocking(true)?;
            let token = uuid::Uuid::new_v4().to_string();
            let page_path = format!("/{token}/overlay");
            let state_path = format!("/{token}/state");
            let url = format!("http://127.0.0.1:{port}{page_path}");
            let router = Router::new()
                .route(&page_path, get(|| async { Html(include_str!("obs.html")) }))
                .route(&state_path, get(obs_snapshot))
                .with_state(snapshot.clone())
                .layer(axum::middleware::map_response(
                    |mut response: axum::response::Response| async move {
                        response
                            .headers_mut()
                            .insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
                        response
                            .headers_mut()
                            .insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
                        response
                            .headers_mut()
                            .insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
                        response
                    },
                ));
            let mut startup_warnings = Vec::new();
            if app.global_shortcut().register(show).is_err() {
                startup_warnings.push("Alt+Shift+O".into());
            }
            if app.global_shortcut().register(lock).is_err() {
                startup_warnings.push("Alt+Shift+L".into());
            }
            app.manage(AppState {
                snapshot,
                obs_url: url,
                startup_warnings,
            });
            tauri::async_runtime::spawn(async move {
                match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => {
                        if let Err(error) = axum::serve(listener, router).await {
                            eprintln!("OBS server stopped: {error}");
                        }
                    }
                    Err(error) => eprintln!("OBS server failed: {error}"),
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "overlay" {
                let scale = window.scale_factor().unwrap_or(1.0);
                match event {
                    tauri::WindowEvent::Moved(position) => {
                        let logical = position.to_logical::<i32>(scale);
                        let _ = window.app_handle().emit_to(
                            "main",
                            "overlay-geometry",
                            serde_json::json!({"x":logical.x,"y":logical.y}),
                        );
                    }
                    tauri::WindowEvent::Resized(size) => {
                        let logical = size.to_logical::<u32>(scale);
                        let _ = window.app_handle().emit_to(
                            "main",
                            "overlay-geometry",
                            serde_json::json!({"width":logical.width.clamp(160,1600),"height":logical.height.clamp(100,1600)}),
                        );
                    }
                    _ => {}
                }
            }
            // There is no tray lifecycle yet. Closing the dashboard must also stop
            // the overlay, private OBS server and provider workers.
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                window.app_handle().exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            set_snapshot,
            get_obs_url,
            get_startup_warnings,
            control_overlay,
            connections::connection_status,
            connections::connect_provider,
            connections::disconnect_provider
        ])
        .run(tauri::generate_context!())
        .expect("ChatNinja could not start");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_valid() {
        assert!(Settings::default().validate().is_ok());
    }
    #[test]
    fn reject_invalid_opacity() {
        let s = Settings {
            opacity: f64::NAN,
            ..Settings::default()
        };
        assert!(s.validate().is_err());
    }
    #[test]
    fn reject_unknown_platform() {
        let mut s = Settings::default();
        s.enabled_platforms.push("unknown".into());
        assert!(s.validate().is_err());
    }
}
