fn main() {
    // Declaring the app's commands generates an `allow-<command>` permission for each; the engine-page capability
    // grants save_file to the page the sidecar serves (a remote origin, which gets no app command by default).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&["save_file", "set_window_title", "app_ready", "text_edit", "take_pending_opens", "confirm_exit"])))
        .expect("tauri build script");
}
