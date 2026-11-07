//! Native menus, background lifecycle and state delivery. No SIP/audio implementation.
use crate::{desktop, phone::PhoneState, shell_policy};
use phone_core::{CallState, EngineSnapshot, RegistrationState};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{
    Emitter, Manager,
    menu::{CheckMenuItem, HELP_SUBMENU_ID, Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

const TRAY_ID: &str = "phone";
const OPEN: &str = "open-phone";
const QUIT: &str = "quit-app";
const SETTINGS: &str = "settings:";
const ANSWER: &str = "answer:";
const REJECT: &str = "reject:";
const HANGUP: &str = "hangup:";
const ACCOUNT: &str = "account:";
// Payload-free navigation requests; call requests carry the captured opaque call token.
const NAVIGATION: &[(&str, &str, &str)] = &[
    ("dialer", "Tastatur", "CmdOrCtrl+1"),
    ("contacts", "Kontakte", "CmdOrCtrl+2"),
    ("history", "Verlauf", "CmdOrCtrl+3"),
    ("number", "Nummer eingeben", "CmdOrCtrl+L"),
    ("find", "Suchen …", "CmdOrCtrl+F"),
    ("search", "Ruf durchsuchen …", "CmdOrCtrl+K"),
];
const CALL_ACTIONS: &[(&str, &str)] = &[
    ("mute", "CmdOrCtrl+Shift+M"),
    ("hold", "CmdOrCtrl+Shift+H"),
    ("transfer", "CmdOrCtrl+Shift+T"),
    ("tones", "CmdOrCtrl+Shift+D"),
];

#[derive(Default)]
pub(crate) struct ShellState {
    stopping: AtomicBool,
    allow_exit: AtomicBool,
    quit_prompt: AtomicBool,
    settings_guard_ready: AtomicBool,
    pending_snapshot: Mutex<shell_policy::PendingSnapshot>,
}

pub(crate) fn setup(app: &mut tauri::App) -> tauri::Result<()> {
    crate::call_notifications::setup(app.handle());
    let handle = app.handle();
    let initial = EngineSnapshot::default();
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(status_icon(initial.call, false))
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip("Ruf · Telefonie wird geladen")
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
    let hidden = desktop::start_hidden(app.handle(), &std::env::args().collect::<Vec<_>>());
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
        if window.label() == "settings" {
            window
                .app_handle()
                .state::<ShellState>()
                .settings_guard_ready
                .store(false, Ordering::Release);
        }
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
    if app
        .state::<ShellState>()
        .settings_guard_ready
        .load(Ordering::Acquire)
        && let Some(window) = app.get_webview_window("settings")
    {
        // Quit is an explicit user action: make draft confirmation visible even
        // when Settings was minimized, instead of leaving a hidden dead end.
        let visible = make_regular(app)
            .and_then(|()| window.unminimize())
            .and_then(|()| window.show())
            .and_then(|()| window.set_focus());
        if visible.is_err()
            || app
                .emit_to("settings", "settings-quit-requested", ())
                .is_err()
        {
            app.dialog().message("Die Einstellungen konnten nicht erreicht werden. Öffne sie erneut, um deine Änderungen vor dem Beenden zu prüfen.")
                .title("Ruf beenden").show(|_| {});
        }
        return;
    }
    quit_after_settings(app);
}

pub(crate) fn settings_ready(app: &tauri::AppHandle) {
    app.state::<ShellState>()
        .settings_guard_ready
        .store(true, Ordering::Release);
}

pub(crate) fn quit_after_settings(app: &tauri::AppHandle) {
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
            .message("Ein Gespräch läuft oder wird aufgebaut. Gespräch beenden und Ruf schließen?")
            .title("Ruf beenden")
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
        crate::call_notifications::shutdown(&app);
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
    if let Some(action) = id.strip_prefix("phone:") {
        if action == "find"
            && let Some(window) = app.get_webview_window("settings")
            && window.is_focused().unwrap_or(false)
        {
            let _ = app.emit_to("settings", "settings-find-requested", ());
            return;
        }
        if (NAVIGATION.iter().any(|(name, _, _)| *name == action) || action == "start")
            && show_phone(app).is_ok()
        {
            let _ = app.emit_to("main", &format!("phone-{action}-requested"), ());
        }
        return;
    }
    for (action, _) in CALL_ACTIONS {
        if let Some(token) = id.strip_prefix(&format!("phone-{action}:")) {
            if show_phone(app).is_ok() {
                let _ = app.emit_to("main", &format!("phone-{action}-requested"), token);
            }
            return;
        }
    }

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
            } else if let Some(call) = id.strip_prefix(REJECT) {
                session.engine()?.reject_call(call)?;
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
fn call_menu(
    app: &tauri::AppHandle,
    state: &EngineSnapshot,
    shortcuts: bool,
) -> tauri::Result<Submenu<tauri::Wry>> {
    let call = Submenu::new(app, "Gespräch", true)?;
    call.append(&item(
        app,
        &format!("{ANSWER}{}", state.call_id.as_deref().unwrap_or("")),
        "Annehmen",
        shell_policy::can_answer(state.call) && state.call_id.is_some(),
        shortcuts.then_some("CmdOrCtrl+Enter"),
    )?)?;
    call.append(&item(
        app,
        &format!(
            "{}{}",
            if state.call == CallState::Incoming {
                REJECT
            } else {
                HANGUP
            },
            state.call_id.as_deref().unwrap_or("")
        ),
        if state.call == CallState::Incoming {
            "Ablehnen"
        } else {
            "Auflegen"
        },
        shell_policy::can_hangup(state.call) && state.call_id.is_some(),
        shortcuts.then_some("CmdOrCtrl+."),
    )?)?;
    call.append(&PredefinedMenuItem::separator(app)?)?;
    let selected = state
        .calls
        .iter()
        .find(|call| Some(&call.id) == state.call_id.as_ref());
    let established =
        selected.is_some_and(|call| matches!(call.state, CallState::Active | CallState::Held));
    let pending = state
        .calls
        .iter()
        .any(|call| call.hold_pending || call.transfer == phone_core::TransferState::Pending);
    for (action, key) in CALL_ACTIONS {
        let (label, enabled) = match *action {
            "mute" => (
                if selected.is_some_and(|call| call.muted) {
                    "Mikrofon einschalten"
                } else {
                    "Stummschalten"
                },
                established,
            ),
            "hold" => (
                if selected.is_some_and(|call| call.local_held) {
                    "Fortsetzen"
                } else {
                    "Halten"
                },
                established && !pending,
            ),
            "transfer" => (
                "Vermitteln / Rückfrage …",
                established && !pending && state.calls.len() == 1,
            ),
            _ => ("Gesprächstastatur ein-/ausblenden", established),
        };
        call.append(&item(
            app,
            &format!("phone-{action}:{}", state.call_id.as_deref().unwrap_or("")),
            label,
            enabled,
            shortcuts.then_some(*key),
        )?)?;
    }
    Ok(call)
}
fn status_text(state: &EngineSnapshot) -> String {
    match state.call {
        CallState::Incoming => "Ruf · Eingehender Anruf".into(),
        CallState::Answering => "Ruf · Wird angenommen …".into(),
        CallState::Active => "Ruf · Gespräch läuft".into(),
        CallState::Held => "Ruf · Gespräch gehalten".into(),
        CallState::EarlyMedia => "Ruf · Frühe Medien".into(),
        CallState::Calling | CallState::Ringing => "Ruf · Verbindungsaufbau".into(),
        CallState::Idle => {
            let ready = state
                .accounts
                .iter()
                .filter(|a| a.registration == RegistrationState::Registered && !a.removing)
                .count();
            if state.last_error.is_some() {
                "Ruf · Verbindung oder Einstellungen prüfen".into()
            } else if state.accounts.is_empty() {
                "Ruf · Kein Konto eingerichtet".into()
            } else {
                format!("Ruf · {ready}/{} Konten verbunden", state.accounts.len())
            }
        }
    }
}
fn tray_menu(app: &tauri::AppHandle, state: &EngineSnapshot) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::with_items(
        app,
        &[
            &item(app, "status", &status_text(state), false, None)?,
            &item(app, OPEN, "Ruf öffnen", true, None)?,
            &PredefinedMenuItem::separator(app)?,
            &call_menu(app, state, false)?,
            &account_menu(app, state)?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "settings:accounts", "SIP-Konten …", true, None)?,
            &item(app, "settings:audio", "Ton …", true, None)?,
            &item(app, "settings:general", "Einstellungen …", true, None)?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, QUIT, "Ruf beenden", true, None)?,
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
            "Ruf"
        } else {
            "Datei"
        },
        true,
    )?;
    application.append(&item(app, "settings:about", "Über Ruf …", true, None)?)?;
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
    application.append(&item(app, QUIT, "Ruf beenden", true, Some("CmdOrCtrl+Q"))?)?;
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
    let view = Submenu::new(app, "Ansicht", true)?;
    for (action, label, key) in NAVIGATION {
        if *action == "number" || *action == "search" {
            view.append(&PredefinedMenuItem::separator(app)?)?;
        }
        view.append(&item(
            app,
            &format!("phone:{action}"),
            label,
            *action != "number" || state.call == CallState::Idle,
            Some(key),
        )?)?;
    }
    let windows = Submenu::with_items(
        app,
        "Fenster",
        true,
        &[
            &item(app, OPEN, "Telefon öffnen", true, None)?,
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    // Tauri registers this ID with NSApplication: macOS supplies the native menu search.
    let help = Submenu::with_id_and_items(
        app,
        HELP_SUBMENU_ID,
        "Hilfe",
        true,
        &[
            &item(app, "phone:search", "Tastenkombinationen …", true, None)?,
            &PredefinedMenuItem::separator(app)?,
            &item(app, "phone:start", "Starthilfe", true, None)?,
            &item(
                app,
                "settings:diagnostics",
                "Diagnose & Hilfe …",
                true,
                None,
            )?,
            &item(app, "settings:about", "Über Ruf …", true, None)?,
        ],
    )?;
    Menu::with_items(
        app,
        &[
            &application,
            &edit,
            &view,
            &call_menu(app, state, true)?,
            &account_menu(app, state)?,
            &windows,
            &help,
        ],
    )
}

fn start_observer(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("phone-observer".into())
        .spawn(move || {
            let mut previous = String::new();
            let mut login_attempted = false;
            let mut observed = shell_policy::ObservedState::default();
            while !app.state::<ShellState>().stopping.load(Ordering::Acquire) {
                let result = desktop::with_session(&app, |session| {
                    Ok((
                        session.snapshot()?,
                        session.desktop_preferences().notifications_enabled,
                    ))
                });
                // A failed read is not evidence that calls or accounts vanished.
                let (mut snapshot, notifications) = observed.update(result);
                if !login_attempted
                    && snapshot
                        .accounts
                        .iter()
                        .any(|account| account.registration == RegistrationState::Registered)
                {
                    match desktop::configure_login_once(&app) {
                        Ok(completed) => login_attempted = completed,
                        Err(_) => {
                            login_attempted = true;
                            eprintln!(
                                "Login start could not be configured; check desktop settings"
                            );
                        }
                    }
                }
                crate::contacts::enrich_snapshot(&app, &mut snapshot);
                crate::call_notifications::sync(&app, &snapshot, notifications);
                let signature = snapshot_signature(&snapshot);
                if signature != previous && queue_snapshot(&app, snapshot).is_ok() {
                    previous = signature;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
        .expect("Failed to start phone state observer");
}

fn snapshot_signature(snapshot: &EngineSnapshot) -> String {
    let mut state = snapshot.clone();
    // Reads have their own ordering revision; publish only actual state changes.
    state.revision = 0;
    serde_json::to_string(&state).unwrap_or_default()
}

fn queue_snapshot(app: &tauri::AppHandle, snapshot: EngineSnapshot) -> tauri::Result<()> {
    let needs_delivery = app
        .state::<ShellState>()
        .pending_snapshot
        .lock()
        .map(|mut pending| pending.replace(snapshot))
        .unwrap_or(false);
    if !needs_delivery {
        return Ok(());
    }
    let handle = app.clone();
    let result = app.run_on_main_thread(move || {
        let snapshot = handle
            .state::<ShellState>()
            .pending_snapshot
            .lock()
            .ok()
            .and_then(|mut pending| pending.take());
        let Some(snapshot) = snapshot else {
            return;
        };
        if let Some(tray) = handle.tray_by_id(TRAY_ID) {
            let _ = tray.set_tooltip(Some(status_text(&snapshot)));
            let ready = snapshot
                .accounts
                .iter()
                .any(|a| a.registration == RegistrationState::Registered);
            // Replacing an image with set_icon clears macOS template tint.
            // Keep the template flag on every state update, without flicker.
            let _ = tray.set_icon_with_as_template(
                Some(status_icon(snapshot.call, ready)),
                cfg!(target_os = "macos"),
            );
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
    if result.is_err()
        && let Ok(mut pending) = app.state::<ShellState>().pending_snapshot.lock()
    {
        pending.take();
    }
    result
}

fn status_icon(call: CallState, ready: bool) -> tauri::image::Image<'static> {
    // The same r/handset mark as the app icon, without its dark tile. macOS
    // tints the alpha mask for light/dark menu bars; Windows keeps the brand blue.
    const MARK: tauri::image::Image<'_> = tauri::include_image!("icons/tray.png");
    let size = MARK.width() as usize;
    let mut rgba = MARK.rgba().to_vec();
    let color = if cfg!(target_os = "macos") {
        [0, 0, 0]
    } else {
        [88, 166, 255]
    };
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel[..3].copy_from_slice(&color);
    }
    let radius: i32 = if call != CallState::Idle { 3 } else { 2 };
    if ready || call != CallState::Idle {
        for y in 0..size {
            for x in 0..size {
                if (x as i32 - 26).pow(2) + (y as i32 - 6).pow(2) <= radius.pow(2) {
                    let pixel = &mut rgba[(y * size + x) * 4..][..4];
                    pixel[..3].copy_from_slice(&color);
                    pixel[3] = 255;
                }
            }
        }
    }
    tauri::image::Image::new_owned(rgba, MARK.width(), MARK.height())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_revisions_do_not_trigger_state_publication_by_themselves() {
        let mut snapshot = EngineSnapshot {
            revision: 17,
            ..Default::default()
        };
        let before = snapshot_signature(&snapshot);
        snapshot.revision = 18;
        assert_eq!(snapshot_signature(&snapshot), before);
        snapshot.call = CallState::Incoming;
        assert_ne!(snapshot_signature(&snapshot), before);
    }

    #[test]
    fn tray_mark_has_no_badge_without_a_ready_account_or_call() {
        let idle = status_icon(CallState::Idle, false);
        let ready = status_icon(CallState::Idle, true);
        let active = status_icon(CallState::Active, true);
        let badge = (6 * 32 + 26) * 4 + 3;
        assert_eq!(idle.rgba()[badge], 0);
        assert!(ready.rgba()[badge] > 0);
        assert!(active.rgba()[badge] > 0);
        let coverage = |image: &tauri::image::Image<'_>| {
            image
                .rgba()
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| u32::from(p[3]))
                .sum::<u32>()
        };
        assert!(coverage(&idle) < coverage(&ready));
        assert!(coverage(&ready) < coverage(&active));
    }

    #[test]
    fn status_identifies_ruf_and_the_current_call() {
        let mut snapshot = EngineSnapshot::default();
        assert_eq!(status_text(&snapshot), "Ruf · Kein Konto eingerichtet");
        snapshot.call = CallState::Incoming;
        assert_eq!(status_text(&snapshot), "Ruf · Eingehender Anruf");
        snapshot.call = CallState::Active;
        assert_eq!(status_text(&snapshot), "Ruf · Gespräch läuft");
    }
}
