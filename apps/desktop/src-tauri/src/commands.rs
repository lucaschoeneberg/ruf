use phone_core::AppInfo;

/// Read startup information without accessing credentials or initializing telephony.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        telephony: baresip_adapter::status(),
    }
}
