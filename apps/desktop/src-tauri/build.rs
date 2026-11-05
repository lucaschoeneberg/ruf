//! Generate Tauri metadata and explicit application-command permissions.

fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_app_info",
            "get_contacts",
            "refresh_contacts",
            "copy_phone_number",
            "open_contacts_app",
            "get_history",
            "delete_history_entry",
            "clear_history",
            "callback_history",
            "request_contacts_access",
            "get_contacts_permission",
            "request_contacts_permission",
            "open_contacts_settings",
            "set_contact_favorite",
            "configure_account",
            "get_engine_snapshot",
            "dial",
            "answer",
            "reject",
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
            "request_notification_permission",
            "settings_ready",
            "quit_app",
            "save_desktop_settings",
        ]),
    ))
    .expect("Failed to build the Tauri application and command permissions");
}
