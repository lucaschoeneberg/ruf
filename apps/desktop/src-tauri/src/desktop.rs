//! Desktop preferences. OS login-start state is authoritative; secrets stay in PhoneSession.
use crate::{phone::PhoneSession, store};
use phone_core::{DesktopSettings, ErrorCode, NotificationPermission, PhoneError};
use std::path::Path;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

static PREFERENCES_UPDATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn with_session<T>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&mut PhoneSession) -> Result<T, PhoneError>,
) -> Result<T, PhoneError> {
    let directory = store::data_directory(app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| store::storage_error())?;
    app.state::<crate::phone::PhoneState>().with(|session| {
        session.load(directory, cache)?;
        operation(session)
    })
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
    let preferences = with_session(app, |session| Ok(session.desktop_preferences()))?;
    let available = autostart_available(app);
    let notification_permission = crate::call_notifications::permission(false)?;
    crate::call_notifications::authorization_observed(app, notification_permission);
    Ok(DesktopSettings {
        autostart_enabled: available
            && app.autolaunch().is_enabled().map_err(|_| desktop_error())?,
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
    // Authorization can fail or wait for a user decision. Do it before changing
    // login-start state, and without holding the phone-session mutex.
    if notifications && !preferences.notifications_enabled {
        if crate::call_notifications::permission(false)? == NotificationPermission::NotDetermined {
            crate::call_notifications::permission(true)?;
        }
        crate::call_notifications::refresh_authorization(app);
    }
    let previous = available && app.autolaunch().is_enabled().map_err(|_| desktop_error())?;
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
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|_| desktop_error())
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

fn configure_login(app: &tauri::AppHandle) -> Result<(), PhoneError> {
    if !autostart_available(app) {
        return Ok(());
    }
    let mut preferences = with_session(app, |session| Ok(session.desktop_preferences()))?;
    if preferences.autostart_configured {
        return Ok(());
    }
    let previous = app.autolaunch().is_enabled().map_err(|_| desktop_error())?;
    set_autostart(app, true)?;
    preferences.autostart_configured = true;
    if let Err(error) = with_session(app, |session| session.save_desktop_preferences(preferences)) {
        set_autostart(app, previous)?;
        return Err(error);
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
