//! Desktop lifecycle and IPC composition; application logic lives in phone-core.

mod call_notifications;
mod commands;
mod contacts;
mod desktop;
mod history;
mod phone;
mod shell;
mod shell_policy;
mod store;
use tauri::Manager;

/// Start the desktop application.
///
/// # Panics
/// Panics if the operating system cannot initialize the Tauri runtime.
pub fn run() {
    let autostart = tauri_plugin_autostart::Builder::new().args(["--background"]);
    #[cfg(target_os = "macos")]
    let autostart = autostart.macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !shell_policy::start_hidden(&args) {
                let _ = shell::show_phone(app);
            }
        }))
        .plugin(autostart.build())
        .plugin(tauri_plugin_dialog::init())
        .manage(phone::PhoneState::default())
        .manage(contacts::ContactsState::default())
        .manage(shell::ShellState::default())
        .menu(|app| shell::app_menu(app, &phone_core::EngineSnapshot::default()))
        .setup(|app| {
            contacts::setup(app.handle());
            history::setup(app.handle());
            shell::setup(app)?;
            Ok(())
        })
        .on_window_event(shell::window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            contacts::get_contacts,
            contacts::refresh_contacts,
            contacts::copy_phone_number,
            contacts::open_contacts_app,
            history::get_history,
            history::delete_history_entry,
            history::clear_history,
            history::callback_history,
            contacts::request_contacts_access,
            contacts::get_contacts_permission,
            contacts::request_contacts_permission,
            contacts::open_contacts_settings,
            contacts::set_contact_favorite,
            commands::configure_account,
            commands::get_engine_snapshot,
            commands::dial,
            commands::answer,
            commands::reject,
            commands::save_account,
            commands::reconnect_account,
            commands::select_account,
            commands::remove_account,
            commands::open_settings,
            commands::get_audio_devices,
            commands::save_sound_settings,
            commands::preview_ringtone,
            commands::stop_ringtone_preview,
            commands::get_desktop_settings,
            commands::request_notification_permission,
            commands::settings_ready,
            commands::quit_app,
            commands::save_desktop_settings,
            commands::hangup
        ])
        .build(tauri::generate_context!())
        .expect("Failed to build the Ruf desktop application");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = &event {
            shell::exit_requested(app, api);
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = &event {
            let _ = shell::show_phone(app);
        }
        if matches!(event, tauri::RunEvent::Exit) {
            contacts::shutdown(app);
            if let Err(error) = app.state::<phone::PhoneState>().shutdown() {
                eprintln!("Engine shutdown: {error}");
            }
            history::shutdown(app);
        }
    });
}
