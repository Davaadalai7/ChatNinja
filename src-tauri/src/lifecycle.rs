use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

pub fn show_settings(app: &AppHandle) {
    // Only focus an existing window here. Creating a WebView synchronously from
    // a Windows event callback can deadlock WebView2.
    if let Some(window) = app.get_webview_window("settings") {
        if let Err(error) = window
            .unminimize()
            .and_then(|_| window.show())
            .and_then(|_| window.set_focus())
        {
            log::error!("Could not show settings: {error}");
        }
    }
}
pub fn quit(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.quitting.swap(true, Ordering::SeqCst) {
        return;
    }
    // There are no sockets or registered shortcuts in Phase 1. Future modules
    // must cancel and join their workers here before requesting application exit.
    log::info!("Quit requested; exiting desktop process");
    log::logger().flush();
    app.exit(0);
}

pub fn install_tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    };
    let settings = MenuItem::with_id(app, "settings", "Jutsu — Settings", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Jutsu", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &quit_item])?;
    let mut tray = TrayIconBuilder::with_id("jutsu-tray")
        .tooltip("Jutsu — Developed by star0x7f")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => show_settings(app),
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_settings(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
