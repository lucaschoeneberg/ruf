//! Desktop preferences. OS login-start state is authoritative; secrets stay in PhoneSession.
use crate::{phone::PhoneSession, store};
use phone_core::{DesktopSettings, ErrorCode, NotificationPermission, PhoneError};
use std::path::Path;
use tauri::Manager;
#[cfg(not(feature = "app-store"))]
use tauri_plugin_autostart::ManagerExt;

static PREFERENCES_UPDATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn with_session<T>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&mut PhoneSession) -> Result<T, PhoneError>,
) -> Result<T, PhoneError> {
    let (directory, cache) = store::session_directories(app)?;
    app.state::<crate::phone::PhoneState>().with(|session| {
        session.load(directory, cache)?;
        operation(session)
    })
}

/// Read startup presentation only for Ruf-owned login launches. Explicit
/// background/notification activation stays hidden; manual open stays visible.
/// A failed metadata read conservatively keeps an automatic login launch hidden.
pub(crate) fn start_hidden(app: &tauri::AppHandle, args: &[String]) -> bool {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    let login_args = if baresip_adapter::launched_as_login_item() {
        let mut args = args.to_vec();
        args.push("--autostart".to_owned());
        Some(args)
    } else {
        None
    };
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    let args = login_args.as_deref().unwrap_or(args);
    start_hidden_with(args, || {
        let path = store::data_directory(app)?.join("accounts.json");
        store::AccountStore { path }
            .load()
            .map(|settings| settings.preferences.start_hidden)
    })
}

fn start_hidden_with(args: &[String], read: impl FnOnce() -> Result<bool, PhoneError>) -> bool {
    if crate::shell_policy::start_hidden(args) {
        return true;
    }
    args.iter().any(|arg| arg == "--autostart") && read().unwrap_or(true)
}

pub(crate) fn installed_path(executable: &Path, installation_roots: &[std::path::PathBuf]) -> bool {
    installation_roots.iter().any(|root| {
        executable.strip_prefix(root).is_ok_and(|relative| {
            #[cfg(target_os = "macos")]
            {
                relative == Path::new("Ruf.app/Contents/MacOS/sip-desktop")
            }
            #[cfg(not(target_os = "macos"))]
            {
                relative == Path::new("Ruf/sip-desktop.exe")
                    || relative == Path::new("Programs/Ruf/sip-desktop.exe")
            }
        })
    })
}

pub(crate) fn autostart_available(app: &tauri::AppHandle) -> bool {
    if cfg!(debug_assertions) {
        return false;
    }
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    #[cfg(target_os = "macos")]
    let roots = vec![
        std::path::PathBuf::from("/Applications"),
        app.path()
            .home_dir()
            .unwrap_or_default()
            .join("Applications"),
    ];
    #[cfg(windows)]
    let roots: Vec<_> = ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"]
        .iter()
        .filter_map(std::env::var_os)
        .map(std::path::PathBuf::from)
        .collect();
    #[cfg(not(any(windows, target_os = "macos")))]
    let roots = {
        let _ = app;
        Vec::new()
    };
    installed_path(&executable, &roots)
}

pub(crate) fn settings(app: &tauri::AppHandle) -> Result<DesktopSettings, PhoneError> {
    let preferences = store::AccountStore {
        path: store::data_directory(app)?.join("accounts.json"),
    }
    .load()?
    .desktop;
    let available = autostart_available(app);
    let notification_permission = crate::call_notifications::permission(false)?;
    crate::call_notifications::authorization_observed(app, notification_permission);
    Ok(DesktopSettings {
        autostart_enabled: available && autostart_enabled(app)?,
        autostart_available: available,
        notifications_enabled: preferences.notifications_enabled,
        notification_permission,
        notifications_granted: match notification_permission {
            NotificationPermission::Authorized => Some(true),
            NotificationPermission::NotDetermined | NotificationPermission::Denied => Some(false),
            NotificationPermission::Unsupported => None,
        },
    })
}

pub(crate) fn save_settings(
    app: &tauri::AppHandle,
    autostart: bool,
    notifications: bool,
) -> Result<DesktopSettings, PhoneError> {
    let _update = PREFERENCES_UPDATE.lock().map_err(|_| desktop_error())?;
    let available = autostart_available(app);
    if autostart && !available {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Login start requires an installed release build.",
        ));
    }
    let mut preferences = with_session(app, |session| Ok(session.desktop_preferences()))?;
    // Saving the app preference never requests OS authorization. The dedicated
    // notification-permission command owns that explicit foreground action.
    let previous = available && autostart_enabled(app)?;
    if available {
        set_autostart(app, autostart)?;
        preferences.autostart_configured = true;
    }
    preferences.notifications_enabled = notifications;
    if let Err(error) = with_session(app, |session| session.save_desktop_preferences(preferences)) {
        if available {
            set_autostart(app, previous)?;
        }
        return Err(error);
    }
    settings(app)
}

fn set_autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), PhoneError> {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    {
        let _ = app;
        baresip_adapter::set_login_item_enabled(enabled)
    }
    #[cfg(not(feature = "app-store"))]
    {
        if enabled {
            app.autolaunch().enable()
        } else {
            app.autolaunch().disable()
        }
        .map_err(|_| desktop_error())
    }
}

fn autostart_enabled(app: &tauri::AppHandle) -> Result<bool, PhoneError> {
    #[cfg(all(target_os = "macos", feature = "app-store"))]
    {
        let _ = app;
        baresip_adapter::login_item_enabled()
    }
    #[cfg(not(feature = "app-store"))]
    {
        app.autolaunch().is_enabled().map_err(|_| desktop_error())
    }
}

fn try_preferences_update(
    update: impl FnOnce() -> Result<(), PhoneError>,
) -> Result<bool, PhoneError> {
    // Foreground updates may be waiting for an OS permission decision. Automatic
    // login setup runs from the phone observer and must never wait behind them.
    let _update = match PREFERENCES_UPDATE.try_lock() {
        Ok(update) => update,
        Err(std::sync::TryLockError::WouldBlock) => return Ok(false),
        Err(std::sync::TryLockError::Poisoned(_)) => return Err(desktop_error()),
    };
    update()?;
    Ok(true)
}

/// Return false when foreground preferences must finish before automatic setup.
pub(crate) fn configure_login_once(app: &tauri::AppHandle) -> Result<bool, PhoneError> {
    try_preferences_update(|| configure_login(app))
}

/// Migrate this installed app's existing enabled login entry independently of
/// SIP/account restoration. Never enables login for an unconfigured or disabled installation.
pub(crate) fn refresh_configured_login_entry(app: &tauri::AppHandle) -> Result<(), PhoneError> {
    if !autostart_available(app) {
        return Ok(());
    }
    try_preferences_update(|| {
        let configured = store::AccountStore {
            path: store::data_directory(app)?.join("accounts.json"),
        }
        .load()?
        .desktop
        .autostart_configured;
        refresh_configured_login_entry_with(
            configured,
            || autostart_enabled(app),
            || set_autostart(app, true),
        )
    })
    .map(|_| ())
}

fn refresh_configured_login_entry_with(
    configured: bool,
    enabled: impl FnOnce() -> Result<bool, PhoneError>,
    refresh: impl FnOnce() -> Result<(), PhoneError>,
) -> Result<(), PhoneError> {
    if configured {
        refresh_enabled_login_entry(enabled()?, refresh)?;
    }
    Ok(())
}

fn configure_login(app: &tauri::AppHandle) -> Result<(), PhoneError> {
    if !autostart_available(app) {
        return Ok(());
    }
    let mut preferences = with_session(app, |session| Ok(session.desktop_preferences()))?;
    let previous = autostart_enabled(app)?;
    if preferences.autostart_configured {
        return refresh_enabled_login_entry(previous, || set_autostart(app, true));
    }
    set_autostart(app, true)?;
    preferences.autostart_configured = true;
    if let Err(error) = with_session(app, |session| session.save_desktop_preferences(preferences)) {
        set_autostart(app, previous)?;
        return Err(error);
    }
    Ok(())
}

fn refresh_enabled_login_entry(
    enabled: bool,
    refresh: impl FnOnce() -> Result<(), PhoneError>,
) -> Result<(), PhoneError> {
    if enabled {
        refresh()?;
    }
    Ok(())
}

fn desktop_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "Desktop integration could not be updated. Check system settings and try again.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_refresh_never_enables_an_unconfigured_or_disabled_entry() {
        refresh_configured_login_entry_with(
            false,
            || panic!("unconfigured startup must not query login state"),
            || panic!("unconfigured startup must not enable login"),
        )
        .unwrap();
        refresh_configured_login_entry_with(
            true,
            || Ok(false),
            || panic!("disabled startup must not enable login"),
        )
        .unwrap();
        let refreshed = std::cell::Cell::new(false);
        refresh_configured_login_entry_with(
            true,
            || Ok(true),
            || {
                refreshed.set(true);
                Ok(())
            },
        )
        .unwrap();
        assert!(refreshed.get());
    }

    #[test]
    fn existing_login_entries_refresh_only_if_enabled() {
        assert!(
            refresh_enabled_login_entry(false, || panic!(
                "disabled login entry must stay disabled"
            ))
            .is_ok()
        );
        let refreshed = std::cell::Cell::new(false);
        refresh_enabled_login_entry(true, || {
            refreshed.set(true);
            Ok(())
        })
        .unwrap();
        assert!(refreshed.get());
        assert_eq!(
            refresh_enabled_login_entry(true, || Err(desktop_error()))
                .unwrap_err()
                .code,
            ErrorCode::NativeFailure
        );
    }

    #[test]
    fn only_login_launches_read_start_hidden_and_errors_remain_hidden() {
        let args = |flag: &str| vec!["Ruf".into(), flag.into()];
        assert!(!start_hidden_with(&args(""), || panic!(
            "manual launch must not read metadata"
        )));
        for explicit in ["--background", "--notification-activation"] {
            assert!(start_hidden_with(&args(explicit), || panic!(
                "explicit policy must not read metadata"
            )));
        }
        assert!(start_hidden_with(&args("--autostart"), || Ok(true)));
        assert!(!start_hidden_with(&args("--autostart"), || Ok(false)));
        assert!(start_hidden_with(&args("--autostart"), || Err(
            store::storage_error()
        )));
        assert!(start_hidden_with(
            &["--autostart".into(), "--background".into()],
            || Ok(false)
        ));
        assert!(!start_hidden_with(&args("--autostart=false"), || panic!(
            "arguments must match exactly"
        )));
    }

    #[test]
    fn pending_foreground_preferences_defer_automatic_setup_without_blocking_observation() {
        let foreground = PREFERENCES_UPDATE.lock().unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            sender.send(try_preferences_update(|| Ok(()))).unwrap();
        });
        let result = receiver.recv_timeout(std::time::Duration::from_secs(1));
        drop(foreground);
        // Join before asserting so a broken blocking implementation cannot leak a worker.
        let joined = worker.join();
        assert!(joined.is_ok());
        assert!(!result.unwrap().unwrap());
        assert!(try_preferences_update(|| Ok(())).unwrap());
    }
    #[test]
    fn login_start_excludes_temporary_or_development_bundles() {
        let root = std::path::PathBuf::from("/installed");
        #[cfg(target_os = "macos")]
        let suffix = "Ruf.app/Contents/MacOS/sip-desktop";
        #[cfg(not(target_os = "macos"))]
        let suffix = "Ruf/sip-desktop.exe";
        assert!(installed_path(
            &root.join(suffix),
            std::slice::from_ref(&root)
        ));
        assert!(!installed_path(
            &Path::new("/tmp/diagnostic").join(suffix),
            std::slice::from_ref(&root)
        ));
        assert!(!installed_path(
            &root.join("native/build").join(suffix),
            &[root]
        ));
    }
}
