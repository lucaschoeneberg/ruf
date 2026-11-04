//! Native menus, background lifecycle and state delivery. No SIP/audio implementation.
use crate::{desktop, phone::PhoneState, shell_policy};
use phone_core::{CallState, EngineSnapshot, RegistrationState};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    Emitter, Manager,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_notification::NotificationExt;

const TRAY_ID: &str = "phone";
const OPEN: &str = "open-phone";
const QUIT: &str = "quit-app";
const SETTINGS: &str = "settings:";
const ANSWER: &str = "answer:";
const HANGUP: &str = "hangup:";
const ACCOUNT: &str = "account:";

#[derive(Default)]
pub(crate) struct ShellState {
    stopping: AtomicBool,
    allow_exit: AtomicBool,
    quit_prompt: AtomicBool,
}

pub(crate) fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    let handle = app.handle();
    let initial = EngineSnapshot::default();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(status_icon(initial.call, false))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("SIP · Telefonie wird geladen")
        .menu(&tray_menu(handle, &initial)?)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_tray_icon_event(|tray, event| {
            if !cfg!(target_os = "macos")
                && matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                )
            {
                let _ = show_phone(tray.app_handle());
            }
        })
        .build(app)?;
    app.on_menu_event(|app, event| dispatch(app, event.id().as_ref()));
    let hidden = shell_policy::start_hidden(&std::env::args().collect::<Vec<_>>());
    if hidden {
        sync_activation(app.handle());
    } else {
        show_phone(app.handle())?;
    }
    start_observer(app.handle().clone());
    Ok(())
}

pub(crate) fn show_phone(app: &tauri::AppHandle) -> tauri::Result<()> {
    make_regular(app)?;
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(windows)]
        window.set_skip_taskbar(false)?;
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

pub(crate) fn make_regular(app: &tauri::AppHandle) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Regular)?;
    #[cfg(not(target_os = "macos"))]
    let _ = app;
    Ok(())
}

pub(crate) fn sync_activation(app: &tauri::AppHandle) {
    let visible = app
        .webview_windows()
        .values()
        .any(|window| window.is_visible().unwrap_or(false));
    #[cfg(target_os = "macos")]
    if !visible {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
    #[cfg(windows)]
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_skip_taskbar(!visible);
    }
    let _ = visible;
}

pub(crate) fn window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if window.label() == "main"
        && let tauri::WindowEvent::CloseRequested { api, .. } = event
    {
        api.prevent_close();
        let _ = window.hide();
        sync_activation(window.app_handle());
    }
    if matches!(event, tauri::WindowEvent::Destroyed) {
        let app = window.app_handle().clone();
        let queued = app.clone();
        let _ = app.run_on_main_thread(move || sync_activation(&queued));
    }
}

pub(crate) fn exit_requested(app: &tauri::AppHandle, api: &tauri::ExitRequestApi) {
    if !app.state::<ShellState>().allow_exit.load(Ordering::Acquire) {
        api.prevent_exit();
        request_quit(app);
    }
}

fn request_quit(app: &tauri::AppHandle) {
    let state = app.state::<ShellState>();
    if state.stopping.load(Ordering::Acquire) || state.quit_prompt.swap(true, Ordering::AcqRel) {
        return;
    }
    let handle = app.clone();
    // Read authoritative state off the UI thread: a just-queued dial/answer may
    // precede the observer's next publication.
    tauri::async_runtime::spawn_blocking(move || {
        let active = handle
            .state::<PhoneState>()
            .with(|session| Ok(session.snapshot()?.call != CallState::Idle))
            .unwrap_or(true);
        let main = handle.clone();
        let _ = handle.run_on_main_thread(move || confirm_quit(&main, active));
    });
}

fn confirm_quit(app: &tauri::AppHandle, active: bool) {
    if active {
        let handle = app.clone();
        app.dialog()
            .message("Ein Gespräch läuft oder wird aufgebaut. Gespräch beenden und SIP schließen?")
            .title("SIP beenden")
            .buttons(MessageDialogButtons::OkCancelCustom(
                "Gespräch beenden und schließen".into(),
                "Zurück".into(),
            ))
            .show(move |confirmed| {
                handle
                    .state::<ShellState>()
                    .quit_prompt
                    .store(false, Ordering::Release);
                if confirmed {
                    finish_quit(handle);
                }
            });
    } else {
        finish_quit(app.clone());
    }
}

fn finish_quit(app: tauri::AppHandle) {
    if app
        .state::<ShellState>()
        .stopping
        .swap(true, Ordering::AcqRel)
    {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        if app.state::<PhoneState>().shutdown().is_err() {
            eprintln!("Native phone shutdown failed");
        }
        app.state::<ShellState>()
            .allow_exit
            .store(true, Ordering::Release);
        app.exit(0);
    });
}

fn dispatch(app: &tauri::AppHandle, id: &str) {
    if id == OPEN {
        let _ = show_phone(app);
        return;
    }
    if id == QUIT {
        request_quit(app);
        return;
    }
    if let Some(section) = id.strip_prefix(SETTINGS) {
        let section = section.to_string();
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let _ = crate::commands::open_settings(app, Some(section)).await;
        });
        return;
    }
    let id = id.to_owned();
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = desktop::with_session(&app, |session| {
            if let Some(call) = id.strip_prefix(ANSWER) {
                session.engine()?.answer_call(call)?;
            } else if let Some(call) = id.strip_prefix(HANGUP) {
                session.engine()?.hangup_call(call)?;
            } else if let Some(account) =
                id.strip_prefix(ACCOUNT).and_then(|v| v.parse::<u32>().ok())
            {
                session.select(account)?;
            } else {
                return Ok(());
            }
            Ok(())
        });
        if result.is_err() {
            let _ = show_phone(&app);
        }
    });
}

fn item(
    app: &tauri::AppHandle,
    id: &str,
    label: &str,
    enabled: bool,
    key: Option<&str>,
) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, label, enabled, key)
}
fn account_menu(
    app: &tauri::AppHandle,
    state: &EngineSnapshot,
) -> tauri::Result<Submenu<tauri::Wry>> {
    let accounts = Submenu::new(app, "Ausgehendes Konto", !state.accounts.is_empty())?;
    for account in &state.accounts {
        let status = if account.registration == RegistrationState::Registered {
            "Verbunden"
        } else {
            "Nicht verbunden"
        };
        accounts.append(&CheckMenuItem::with_id(
            app,
            format!("{ACCOUNT}{}", account.id),
            format!("{} · {status}", account.name.replace('&', "&&")),
            state.call == CallState::Idle && !account.removing,
            Some(account.id) == state.selected_account_id,
            None::<&str>,
        )?)?;
    }
    Ok(accounts)
}
fn call_menu(app: &tauri::AppHandle, state: &EngineSnapshot) -> tauri::Result<Submenu<tauri::Wry>> {
    let call = Submenu::new(app, "Gespräch", true)?;
    call.append(&item(
        app,
        &format!("{ANSWER}{}", state.call_id.as_deref().unwrap_or("")),
        "Annehmen",
        shell_policy::can_answer(state.call) && state.call_id.is_some(),
        None,
    )?)?;
    call.append(&item(
        app,
        &format!("{HANGUP}{}", state.call_id.as_deref().unwrap_or("")),
        if state.call == CallState::Incoming {
            "Ablehnen"
        } else {
            "Auflegen"
        },
        shell_policy::can_hangup(state.call) && state.call_id.is_some(),
        None,
    )?)?;
    Ok(call)
}
fn status_text(state: &EngineSnapshot) -> String {
    match state.call {
        CallState::Incoming => "SIP · Eingehender Anruf".into(),
        CallState::Answering => "SIP · Wird angenommen …".into(),
        CallState::Active => "SIP · Gespräch läuft".into(),
        CallState::Calling | CallState::Ringing => "SIP · Verbindungsaufbau".into(),
        CallState::Idle => {
            let ready = state
                .accounts
                .iter()
                .filter(|a| a.registration == RegistrationState::Registered && !a.removing)
                .count();
            if state.last_error.is_some() {
                "SIP · Verbindung oder Einstellungen prüfen".into()
            } else if state.accounts.is_empty() {
                "SIP · Kein Konto eingerichtet".into()
            } else {
                format!("SIP · {ready}/{} Konten verbunden", state.accounts.len())
            }
        }
    }
}
fn tray_menu(app: &tauri::AppHandle, state: &EngineSnapshot) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::with_items(
        app,
        &[
            &item(app, "status", &status_text(state), false, None)?,
            &item(app, OPEN, "Telefon öffnen", true, None)?,
            &PredefinedMenuItem::separator(app)?,
            &call_menu(app, state)?,
            &account_menu(app, state)?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "settings:accounts", "SIP-Konten …", true, None)?,
            &item(app, "settings:audio", "Ton …", true, None)?,
            &item(app, "settings:general", "Einstellungen …", true, None)?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, QUIT, "SIP beenden", true, None)?,
        ],
    )
}
pub(crate) fn app_menu(
    app: &tauri::AppHandle,
    state: &EngineSnapshot,
) -> tauri::Result<Menu<tauri::Wry>> {
    let application = Submenu::new(
        app,
        if cfg!(target_os = "macos") {
            "SIP"
        } else {
            "Datei"
        },
        true,
    )?;
    application.append(&item(app, "settings:about", "Über SIP …", true, None)?)?;
    application.append(&item(
        app,
        "settings:general",
        "Einstellungen …",
        true,
        Some("CmdOrCtrl+,"),
    )?)?;
    #[cfg(target_os = "macos")]
    {
        application.append(&PredefinedMenuItem::services(app, None)?)?;
        application.append(&PredefinedMenuItem::separator(app)?)?;
        application.append(&PredefinedMenuItem::hide(app, None)?)?;
        application.append(&PredefinedMenuItem::hide_others(app, None)?)?;
        application.append(&PredefinedMenuItem::show_all(app, None)?)?;
    }
    application.append(&PredefinedMenuItem::separator(app)?)?;
    application.append(&item(app, QUIT, "SIP beenden", true, Some("CmdOrCtrl+Q"))?)?;
    let edit = Submenu::with_items(
        app,
        "Bearbeiten",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let windows = Submenu::with_items(
        app,
        "Fenster",
        true,
        &[
            &item(app, OPEN, "Telefon", true, Some("CmdOrCtrl+1"))?,
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    Menu::with_items(
        app,
        &[
            &application,
            &edit,
            &call_menu(app, state)?,
            &account_menu(app, state)?,
            &windows,
        ],
    )
}

fn start_observer(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("phone-observer".into())
        .spawn(move || {
            let mut previous = String::new();
            let mut notified = None;
            let mut login_attempted = false;
            while !app.state::<ShellState>().stopping.load(Ordering::Acquire) {
                let result = desktop::with_session(&app, |session| {
                    Ok((
                        session.snapshot()?,
                        session.desktop_preferences().notifications_enabled,
                    ))
                });
                let (snapshot, notifications) = match result {
                    Ok(value) => value,
                    Err(error) => (
                        EngineSnapshot {
                            last_error: Some(error),
                            ..Default::default()
                        },
                        false,
                    ),
                };
                if !login_attempted
                    && snapshot
                        .accounts
                        .iter()
                        .any(|account| account.registration == RegistrationState::Registered)
                {
                    login_attempted = true;
                    if desktop::configure_login_once(&app).is_err() {
                        eprintln!("Login start could not be configured; check desktop settings");
                    }
                }
                if snapshot.call == CallState::Incoming
                    && snapshot.call_id.is_some()
                    && snapshot.call_id != notified
                {
                    notified = snapshot.call_id.clone();
                    if notifications {
                        let account = snapshot
                            .accounts
                            .iter()
                            .find(|a| Some(a.id) == snapshot.call_account_id)
                            .map_or("SIP", |a| a.name.as_str());
                        let _ = app
                            .notification()
                            .builder()
                            .title("Eingehender Anruf")
                            .body(format!(
                                "{account} · Über das SIP-Statussymbol annehmen oder ablehnen."
                            ))
                            .silent()
                            .show();
                    }
                }
                let signature = serde_json::to_string(&snapshot).unwrap_or_default();
                if signature != previous {
                    previous = signature;
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
                            let _ = tray.set_tooltip(Some(status_text(&snapshot)));
                            let ready = snapshot
                                .accounts
                                .iter()
                                .any(|a| a.registration == RegistrationState::Registered);
                            let _ = tray.set_icon(Some(status_icon(snapshot.call, ready)));
                            if let Ok(menu) = tray_menu(&handle, &snapshot) {
                                let _ = tray.set_menu(Some(menu));
                            }
                        }
                        if let Ok(menu) = app_menu(&handle, &snapshot) {
                            let _ = handle.set_menu(menu);
                        }
                        let _ = handle.emit_to("main", "phone-state-changed", &snapshot);
                        let _ = handle.emit_to("settings", "phone-state-changed", &snapshot);
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
        .expect("Failed to start phone state observer");
}

fn status_icon(call: CallState, ready: bool) -> tauri::image::Image<'static> {
    // A small monochrome handset drawn directly as RGBA; macOS applies template tint.
    let size = 32usize;
    let mut rgba = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let xf = x as f64;
            let yf = y as f64;
            let arc = ((xf - 17.0).powi(2) + (yf - 13.0).powi(2)).sqrt();
            let handset = (9.0..=13.0).contains(&arc) && yf > 9.0 && xf < 25.0;
            let ends = ((5.0..=10.0).contains(&xf) && (7.0..=15.0).contains(&yf))
                || ((20.0..=28.0).contains(&xf) && (20.0..=25.0).contains(&yf));
            let dot = (xf - 26.0).powi(2) + (yf - 6.0).powi(2)
                <= if call != CallState::Idle {
                    16.0
                } else if ready {
                    7.0
                } else {
                    0.0
                };
            if handset || ends || dot {
                let c = if cfg!(target_os = "macos") { 0 } else { 230 };
                rgba[(y * size + x) * 4..(y * size + x) * 4 + 4].copy_from_slice(&[c, c, c, 255]);
            }
        }
    }
    tauri::image::Image::new_owned(rgba, size as u32, size as u32)
}
