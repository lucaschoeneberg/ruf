//! Desktop lifecycle and IPC composition; application logic lives in phone-core.

mod commands;
mod phone;
use tauri::Manager;

/// Start the desktop application.
///
/// # Panics
/// Panics if the operating system cannot initialize the Tauri runtime.
pub fn run() {
    let app = tauri::Builder::default()
        .manage(phone::PhoneState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::configure_account,
            commands::get_engine_snapshot,
            commands::dial,
            commands::answer,
            commands::hangup
        ])
        .build(tauri::generate_context!())
        .expect("Failed to build the SIP desktop application");
    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit)
            && let Err(error) = app.state::<phone::PhoneState>().shutdown()
        {
            eprintln!("Engine shutdown: {error}");
        }
    });
}
