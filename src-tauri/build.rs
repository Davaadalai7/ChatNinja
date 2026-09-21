fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "bootstrap",
            "save_settings",
            "reset_settings",
            "open_logs_folder",
            "quit_app",
        ]),
    ))
    .expect("Tauri build configuration is invalid");
}
