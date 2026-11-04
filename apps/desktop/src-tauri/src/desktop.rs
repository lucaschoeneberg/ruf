//! Desktop preferences. OS login-start state is authoritative; secrets stay in PhoneSession.
use crate::{phone::PhoneSession, store};
use phone_core::{DesktopSettings, ErrorCode, PhoneError};
use std::path::Path;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

static PREFERENCES_UPDATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn with_session<T>(
    app: &tauri::AppHandle,
    operation: impl FnOnce(&mut PhoneSession) -> Result<T, PhoneError>,
) -> Result<T, PhoneError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| store::storage_error())?;
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
                relative == Path::new("SIP.app/Contents/MacOS/sip-desktop")
            }
            #[cfg(not(target_os = "macos"))]
            {
                relative == Path::new("SIP/sip-desktop.exe")
                    || relative == Path::new("Programs/SIP/sip-desktop.exe")
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
    Ok(DesktopSettings {
        autostart_enabled: available
            && app.autolaunch().is_enabled().map_err(|_| desktop_error())?,
        autostart_available: available,
        notifications_enabled: preferences.notifications_enabled,
        // Tauri's desktop implementation unconditionally returns Granted, which
        // is not an OS authorization query. Do not report that as real permission.
        notifications_granted: None,
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
    let previous = available && app.autolaunch().is_enabled().map_err(|_| desktop_error())?;
    if available {
        set_autostart(app, autostart)?;
        preferences.autostart_configured = true;
    }
    // Only called from the settings window. Never request permission from the
    // background observer. Desktop plugins currently defer authorization to OS UI.
    if notifications && !preferences.notifications_enabled {
        app.notification()
            .request_permission()
            .map_err(|_| desktop_error())?;
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

pub(crate) fn configure_login_once(app: &tauri::AppHandle) -> Result<(), PhoneError> {
    let _update = PREFERENCES_UPDATE.lock().map_err(|_| desktop_error())?;
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
    fn login_start_excludes_temporary_or_development_bundles() {
        let root = std::path::PathBuf::from("/installed");
        #[cfg(target_os = "macos")]
        let suffix = "SIP.app/Contents/MacOS/sip-desktop";
        #[cfg(not(target_os = "macos"))]
        let suffix = "SIP/sip-desktop.exe";
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
