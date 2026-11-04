//! Desktop lifecycle and IPC composition; application logic lives in phone-core.

mod commands;

/// Start the desktop application.
///
/// # Panics
/// Panics if the operating system cannot initialize the Tauri runtime.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::get_app_info])
        .run(tauri::generate_context!())
        .expect("Failed to run the SIP desktop application");
}
