use crate::{AppState, Shared};
use chatninja_core::{model::ChatBuffer, Config, Event, Provider, Sink, Status};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct Connections {
    tasks: tokio::sync::Mutex<HashMap<Provider, tokio::task::JoinHandle<()>>>,
    pub buffer: Mutex<ChatBuffer>,
    states: Mutex<HashMap<Provider, Status>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    provider: Provider,
    configured: bool,
    status: Status,
}

fn config(provider: Provider) -> Config {
    // Public developer identifiers baked into the release by the publisher.
    // Never add Kick/Twitch confidential client secrets to this executable.
    Config {
        provider,
        client_id: match provider {
            Provider::Youtube => option_env!("CHATNINJA_GOOGLE_CLIENT_ID").unwrap_or(""),
            Provider::Twitch => option_env!("CHATNINJA_TWITCH_CLIENT_ID").unwrap_or(""),
            Provider::Kick => option_env!("CHATNINJA_KICK_RELAY_ORIGIN").unwrap_or(""),
        }
        .into(),
        desktop_secret: if provider == Provider::Youtube {
            option_env!("CHATNINJA_GOOGLE_DESKTOP_SECRET").map(str::to_string)
        } else {
            None
        },
    }
}
fn require_dashboard(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("dashboard_required".into())
    }
}
pub fn publish_messages(connections: &Connections, shared: &Shared, app: &tauri::AppHandle) {
    if let Ok(buffer) = connections.buffer.lock() {
        let messages = buffer.snapshot();
        if let Ok(mut snapshot) = shared.lock() {
            if !snapshot.settings.demo {
                snapshot.messages = messages
                    .iter()
                    .filter_map(|m| serde_json::to_value(m).ok())
                    .collect();
            }
        }
        let _ = app.emit_to("main", "live-messages", messages);
    }
}
#[tauri::command]
pub fn connection_status(
    window: tauri::WebviewWindow,
    state: tauri::State<Arc<Connections>>,
) -> Result<Vec<ConnectionInfo>, String> {
    require_dashboard(&window)?;
    let states = state.states.lock().map_err(|_| "state_unavailable")?;
    Ok([Provider::Youtube, Provider::Kick, Provider::Twitch]
        .iter()
        .map(|provider| {
            let configured = config(*provider).validate().is_ok();
            let status = states.get(provider).cloned().unwrap_or(Status {
                provider: *provider,
                state: if configured {
                    "disconnected"
                } else {
                    "setup_required"
                }
                .into(),
                detail: if *provider == Provider::Kick && !configured {
                    "kick_backend_required"
                } else if !configured {
                    "developer_app_required"
                } else {
                    ""
                }
                .into(),
            });
            ConnectionInfo {
                provider: *provider,
                configured,
                status,
            }
        })
        .collect())
}
#[tauri::command]
pub async fn connect_provider(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Connections>>,
    provider: Provider,
) -> Result<(), String> {
    require_dashboard(&window)?;
    let config = config(provider);
    config.validate()?;
    let connections = state.inner().clone();
    let shared = app.state::<AppState>().snapshot.clone();
    let mut tasks = connections.tasks.lock().await;
    if let Some(task) = tasks.remove(&provider) {
        task.abort();
        let _ = task.await;
    }
    if let Ok(mut buffer) = connections.buffer.lock() {
        buffer.clear_provider(provider);
    }
    publish_messages(&connections, &shared, &app);
    let owned = connections.clone();
    let sink: Sink = Arc::new(move |event| match event {
        Event::Status(status) => {
            if let Ok(mut states) = owned.states.lock() {
                states.insert(status.provider, status.clone());
            }
            let _ = app.emit_to("main", "connection-status", status);
        }
        Event::Message(message) => {
            if let Ok(mut buffer) = owned.buffer.lock() {
                buffer.push(message);
            }
            publish_messages(&owned, &shared, &app);
        }
        Event::Remove(removal) => {
            if let Ok(mut buffer) = owned.buffer.lock() {
                buffer.remove(&removal);
            }
            publish_messages(&owned, &shared, &app);
        }
    });
    let task = tokio::spawn(async move {
        if let Err(error) = chatninja_core::run(config, sink.clone()).await {
            chatninja_core::status(&sink, provider, "error", &error);
        }
    });
    tasks.insert(provider, task);
    Ok(())
}
#[tauri::command]
pub async fn disconnect_provider(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Connections>>,
    provider: Provider,
    forget: bool,
) -> Result<(), String> {
    require_dashboard(&window)?;
    let mut tasks = state.tasks.lock().await;
    // Await aborted worker before deleting credentials: no late refresh can resurrect them.
    if let Some(task) = tasks.remove(&provider) {
        task.abort();
        let _ = task.await;
    }
    let mut detail = String::new();
    if forget {
        let config = config(provider);
        let token = chatninja_core::vault::load(&config)?;
        chatninja_core::vault::delete(&config)?;
        if let Some(token) = token {
            let client = chatninja_core::http_client()?;
            if chatninja_core::auth::revoke(&client, &config, &token)
                .await
                .is_err()
            {
                detail = "local_logout_done_remote_revoke_failed".into();
            }
        }
    }
    if let Ok(mut buffer) = state.buffer.lock() {
        buffer.clear_provider(provider);
    }
    publish_messages(&state, &app.state::<AppState>().snapshot, &app);
    let status = Status {
        provider,
        state: "disconnected".into(),
        detail,
    };
    state
        .states
        .lock()
        .map_err(|_| "state_unavailable")?
        .insert(provider, status.clone());
    app.emit_to("main", "connection-status", status)
        .map_err(|_| "event_failed".into())
}
