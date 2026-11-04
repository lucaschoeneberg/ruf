//! Generate Tauri metadata and explicit application-command permissions.

fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_app_info",
            "configure_account",
            "get_engine_snapshot",
            "dial",
            "answer",
            "hangup",
            "save_account",
            "select_account",
            "remove_account",
            "open_settings",
            "get_audio_devices",
            "save_sound_settings",
            "preview_ringtone",
            "stop_ringtone_preview",
            "get_desktop_settings",
            "save_desktop_settings",
        ]),
    ))
    .expect("Failed to build the Tauri application and command permissions");
}
