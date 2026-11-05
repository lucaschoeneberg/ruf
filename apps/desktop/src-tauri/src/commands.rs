use crate::phone::{PhoneSession, PhoneState};
use phone_core::{
    AccountConfig, AppInfo, AudioDevices, EngineSnapshot, ErrorCode, PhoneError, SoundSettings,
};
use tauri::{Emitter, Manager};

async fn with_session(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    operation: impl FnOnce(&mut PhoneSession) -> Result<EngineSnapshot, PhoneError> + Send + 'static,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = crate::store::data_directory(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    let mut snapshot = state
        .execute(move |session| {
            session.load(directory, cache)?;
            operation(session)
        })
        .await?;
    crate::contacts::enrich_snapshot(&app, &mut snapshot);
    Ok(snapshot)
}

/// Read startup information without initializing telephony or accessing credentials.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        telephony: baresip_adapter::status(),
    }
}

/// Restore saved accounts once, then read shared password-free state from either window.
#[tauri::command]
pub async fn get_engine_snapshot(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, |session| session.snapshot()).await
}

/// Save credentials to the OS keychain and queue independent registration.
/// Null id creates an account; an existing id updates it. Empty password on update keeps the saved secret.
#[tauri::command]
pub async fn save_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: Option<u32>,
    name: String,
    account: AccountConfig,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = crate::store::data_directory(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state
        .save_account(directory, cache, id, name, account)
        .await
}

/// Reconnect a saved account after explicit foreground credential authorization.
/// Required id must refer to an existing, non-removing account; requires idle call state.
/// Does not save form drafts, rewrite metadata, rotate credentials or change outgoing selection.
#[tauri::command]
pub async fn reconnect_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: u32,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = crate::store::data_directory(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state.reconnect_account(directory, cache, id).await
}

/// Toggle saved account activation, preserving identity, credentials and selection.
/// Requires a known idle account. Only enabling may authorize saved credentials;
/// native acknowledgment and newer-intent guards determine the returned snapshot.
#[tauri::command]
pub async fn set_account_enabled(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: u32,
    enabled: bool,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = crate::store::data_directory(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state
        .set_account_enabled(directory, cache, id, enabled)
        .await
}

/// Compatibility command: save/replace the selected account using its username as name.
#[tauri::command]
pub async fn configure_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    account: AccountConfig,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = crate::store::data_directory(&app)?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state.configure_account(directory, cache, account).await
}

/// Persist outgoing selection; other user agents remain registered.
#[tauri::command]
pub async fn select_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: u32,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| session.select(id)).await
}

/// Delete saved credentials/metadata and queue unregistering. Requires an idle phone.
#[tauri::command]
pub async fn remove_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    id: u32,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| session.remove(id)).await
}

/// Queue a validated number on the selected account, reserving the single call slot.
#[tauri::command]
pub async fn dial(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    destination: String,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| {
        session.engine()?.dial(&destination)?;
        session.snapshot()
    })
    .await
}

/// Accept the single offered incoming call on its own account.
#[tauri::command]
pub async fn answer(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    call_id: String,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| {
        session.engine()?.answer_call(&call_id)?;
        session.snapshot()
    })
    .await
}

/// Reject only the offered incoming call; an already answered call rejects with InvalidState.
#[tauri::command]
pub async fn reject(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    call_id: String,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| {
        session.engine()?.reject_call(&call_id)?;
        session.snapshot()
    })
    .await
}

/// End the current call in any non-idle state; completion follows through native state.
#[tauri::command]
pub async fn hangup(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    call_id: String,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| {
        session.engine()?.hangup_call(&call_id)?;
        session.snapshot()
    })
    .await
}

/// Open or focus the single settings window. Async avoids Windows WebView2 deadlocks.
#[tauri::command]
pub async fn open_settings(
    app: tauri::AppHandle,
    section: Option<String>,
) -> Result<(), PhoneError> {
    let section = section.unwrap_or_else(|| "accounts".into());
    if !matches!(
        section.as_str(),
        "general"
            | "accounts"
            | "audio"
            | "contacts"
            | "about"
            | "notifications"
            | "privacy"
            | "diagnostics"
    ) {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Unknown settings section.",
        ));
    }
    static OPEN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = OPEN.lock().map_err(|_| window_error())?;
    crate::shell::make_regular(&app).map_err(|_| window_error())?;
    if let Some(window) = app.get_webview_window("settings") {
        window.unminimize().map_err(|_| window_error())?;
        window.show().map_err(|_| window_error())?;
        app.emit_to("settings", "settings-section", &section)
            .map_err(|_| window_error())?;
        return window.set_focus().map_err(|_| window_error());
    }
    tauri::WebviewWindowBuilder::new(
        &app,
        "settings",
        tauri::WebviewUrl::App(format!("index.html?window=settings&section={section}").into()),
    )
    .title("Ruf · Einstellungen")
    .inner_size(900.0, 660.0)
    .min_inner_size(740.0, 540.0)
    .theme(Some(tauri::Theme::Dark))
    .center()
    .build()
    .map_err(|_| window_error())?;
    Ok(())
}

/// Read actual OS login-start state and saved notification preference; permission may be unknown.
#[tauri::command]
pub async fn get_desktop_settings(
    app: tauri::AppHandle,
) -> Result<phone_core::DesktopSettings, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || crate::desktop::settings(&app))
        .await
        .map_err(|_| window_error())?
}

/// Ask for native notification access only after a foreground settings action.
/// Returns current desktop settings; denial is a normal permission result.
#[tauri::command]
pub async fn request_notification_permission(
    app: tauri::AppHandle,
) -> Result<phone_core::DesktopSettings, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::call_notifications::permission(true)?;
        crate::call_notifications::refresh_authorization(&app);
        crate::desktop::settings(&app)
    })
    .await
    .map_err(|_| window_error())?
}

/// Change login-start/notification preferences from the foreground settings window only.
/// Temporary or development builds reject enabling login start with InvalidInput.
#[tauri::command]
pub async fn save_desktop_settings(
    app: tauri::AppHandle,
    autostart_enabled: bool,
    notifications_enabled: bool,
) -> Result<phone_core::DesktopSettings, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::desktop::save_settings(&app, autostart_enabled, notifications_enabled)
    })
    .await
    .map_err(|_| window_error())?
}
fn window_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The settings window could not be opened. Please try again.",
    )
}

/// Mark the settings discard/quit listeners ready before the form becomes editable.
#[tauri::command]
pub fn settings_ready(app: tauri::AppHandle) {
    crate::shell::settings_ready(&app);
}

/// Continue an explicit quit after the settings window has handled its drafts.
/// Running calls still require native confirmation; this does not force an exit.
#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    crate::shell::quit_after_settings(&app);
}

/// Read current native audio endpoint metadata without starting SIP or microphone capture.
#[tauri::command]
pub async fn get_audio_devices() -> Result<AudioDevices, PhoneError> {
    tauri::async_runtime::spawn_blocking(baresip_adapter::audio_devices)
        .await
        .map_err(|_| {
            PhoneError::new(
                ErrorCode::NativeFailure,
                "Audio inventory could not complete.",
            )
        })?
}
/// Persist shared routing for subsequent calls; rejects active calls and invalid devices.
#[tauri::command]
pub async fn save_sound_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    sound: SoundSettings,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| session.save_sound(sound)).await
}
/// Play the built-in two-second preview on a draft output without saving it.
#[tauri::command]
pub async fn preview_ringtone(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    device: String,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, move |session| session.preview_ring(device)).await
}
/// Stop only a preview, preserving an actual incoming-call alert.
#[tauri::command]
pub async fn stop_ringtone_preview(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
) -> Result<EngineSnapshot, PhoneError> {
    with_session(app, state, |session| session.stop_preview()).await
}

/// Read OS notification authorization without accounts, metadata or a permission prompt.
#[tauri::command]
pub async fn get_notification_permission(
    app: tauri::AppHandle,
) -> Result<phone_core::NotificationPermission, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        let permission = crate::call_notifications::permission(false)?;
        crate::call_notifications::authorization_observed(&app, permission);
        Ok(permission)
    })
    .await
    .map_err(|_| window_error())?
}

/// Open the fixed OS notification pane; never changes permission or accepts a URL.
#[tauri::command]
pub async fn open_notification_settings() -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(|| open_permission_settings(false))
        .await
        .map_err(|_| window_error())?
}
/// Open the fixed OS microphone privacy pane; never changes permission.
#[tauri::command]
pub async fn open_microphone_settings() -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(|| open_permission_settings(true))
        .await
        .map_err(|_| window_error())?
}
fn open_permission_settings(microphone: bool) -> Result<(), PhoneError> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("/usr/bin/open").arg(if microphone {
        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
    } else { "x-apple.systempreferences:com.apple.Notifications-Settings.extension?id=org.rufapp.desktop" }).status();
    #[cfg(windows)]
    let result = std::process::Command::new("cmd")
        .args([
            "/C",
            "start",
            "",
            if microphone {
                "ms-settings:privacy-microphone"
            } else {
                "ms-settings:notifications"
            },
        ])
        .status();
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = microphone;
        return Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "System permission settings are unavailable on this platform.",
        ));
    }
    #[cfg(any(target_os = "macos", windows))]
    if result.is_ok_and(|status| status.success()) {
        Ok(())
    } else {
        Err(PhoneError::new(
            ErrorCode::NativeFailure,
            "System permission settings could not be opened.",
        ))
    }
}

static TEST_NOTICE_PENDING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
/// Explicit anonymous test after five seconds. Never asks for access or changes SIP/call state.
/// Completion means native submission, not proof that Focus/OS showed a banner.
#[tauri::command]
pub async fn test_notification() -> Result<(), PhoneError> {
    use std::sync::atomic::Ordering;
    if TEST_NOTICE_PENDING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(PhoneError::new(
            ErrorCode::Busy,
            "A notification test is already pending.",
        ));
    }
    struct Pending;
    impl Drop for Pending {
        fn drop(&mut self) {
            TEST_NOTICE_PENDING.store(false, Ordering::Release);
        }
    }
    let pending = Pending;
    tauri::async_runtime::spawn_blocking(move || {
        let _pending = pending;
        if crate::call_notifications::permission(false)?
            != phone_core::NotificationPermission::Authorized
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "Notification authorization is required for a test.",
            ));
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
        baresip_adapter::show_test_notification()
    })
    .await
    .map_err(|_| window_error())?
}
/// Read microphone authorization without opening an audio stream or prompting.
#[tauri::command]
pub async fn get_microphone_permission() -> Result<phone_core::MicrophonePermission, PhoneError> {
    tauri::async_runtime::spawn_blocking(|| baresip_adapter::microphone_permission(false))
        .await
        .map_err(|_| window_error())?
}
/// Explicit foreground request for undecided microphone access; never starts capture.
#[tauri::command]
pub async fn request_microphone_permission() -> Result<phone_core::MicrophonePermission, PhoneError>
{
    tauri::async_runtime::spawn_blocking(|| baresip_adapter::microphone_permission(true))
        .await
        .map_err(|_| window_error())?
}
/// Begin one bounded idle-only native audio test on the specified opaque endpoint.
#[tauri::command]
pub async fn start_audio_test(
    app: tauri::AppHandle,
    kind: phone_core::AudioTestKind,
    device: String,
) -> Result<String, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::desktop::with_session(&app, |session| {
            session.engine()?.start_audio_test(kind, &device)
        })
    })
    .await
    .map_err(|_| window_error())?
}
/// Stop only the owned test; stale IDs do not stop a replacement or a real call.
#[tauri::command]
pub async fn stop_audio_test(app: tauri::AppHandle, id: String) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::desktop::with_session(&app, |session| session.engine()?.stop_audio_test(&id))
    })
    .await
    .map_err(|_| window_error())?
}

/// Read an anonymous whitelist of health metadata without restoring accounts,
/// reading credentials, enumerating contacts or opening audio capture/playback.
/// Individual failed readings stay null; only worker failure rejects the command.
#[tauri::command]
pub async fn get_diagnostics(
    app: tauri::AppHandle,
) -> Result<phone_core::DiagnosticsReport, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || read_diagnostics(&app))
        .await
        .map_err(|_| diagnostic_error())
}

/// Copy a newly read anonymous report as compact JSON; accepts no caller content.
/// Clipboard support/failures retain the existing native clipboard error codes.
#[tauri::command]
pub async fn copy_diagnostics(app: tauri::AppHandle) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        let text =
            serde_json::to_string(&read_diagnostics(&app)).map_err(|_| diagnostic_error())?;
        baresip_adapter::copy_plain_text(&text)
    })
    .await
    .map_err(|_| diagnostic_error())?
}

fn diagnostic_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The diagnostic report could not be processed.",
    )
}

fn read_diagnostics(app: &tauri::AppHandle) -> phone_core::DiagnosticsReport {
    let snapshot = app.state::<PhoneState>().diagnostics_snapshot();
    diagnostic_report(
        snapshot,
        crate::call_notifications::permission(false),
        baresip_adapter::microphone_permission(false),
        baresip_adapter::audio_devices(),
        crate::history::recording_error(app),
    )
}

fn diagnostic_report(
    snapshot: Option<EngineSnapshot>,
    notification: Result<phone_core::NotificationPermission, PhoneError>,
    microphone: Result<phone_core::MicrophonePermission, PhoneError>,
    devices: Result<AudioDevices, PhoneError>,
    history_error: Option<PhoneError>,
) -> phone_core::DiagnosticsReport {
    // Select explicit fields. Never serialize a snapshot or raw native error text.
    let snapshot_errors: Vec<_> = snapshot
        .iter()
        .flat_map(|snapshot| {
            snapshot.last_error.iter().chain(
                snapshot
                    .accounts
                    .iter()
                    .filter_map(|account| account.last_error.as_ref()),
            )
        })
        .collect();
    let credential_error = snapshot_errors.iter().map(|error| error.code).find(|code| {
        matches!(
            code,
            ErrorCode::CredentialAccessRequired
                | ErrorCode::CredentialMissing
                | ErrorCode::CredentialFailure
        )
    });
    let last_error = snapshot_errors
        .first()
        .map(|error| error.code)
        .or_else(|| history_error.as_ref().map(|error| error.code))
        .or_else(|| notification.as_ref().err().map(|error| error.code))
        .or_else(|| microphone.as_ref().err().map(|error| error.code))
        .or_else(|| devices.as_ref().err().map(|error| error.code));
    phone_core::DiagnosticsReport {
        version: env!("CARGO_PKG_VERSION").into(),
        platform: std::env::consts::OS.into(),
        telephony: baresip_adapter::status(),
        accounts_total: snapshot
            .as_ref()
            .map(|snapshot| snapshot.accounts.len() as u32),
        accounts_registered: snapshot.as_ref().map(|snapshot| {
            snapshot
                .accounts
                .iter()
                .filter(|account| account.registration == phone_core::RegistrationState::Registered)
                .count() as u32
        }),
        call_active: snapshot
            .as_ref()
            .map(|snapshot| snapshot.call != phone_core::CallState::Idle),
        notification_permission: notification.ok(),
        microphone_permission: microphone.ok(),
        audio_devices_available: devices
            .ok()
            .map(|devices| !devices.inputs.is_empty() && !devices.outputs.is_empty()),
        credential_error,
        history_available: history_error.is_none(),
        last_error,
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn report_whitelists_counts_and_error_codes_and_excludes_identity() {
        let mut snapshot = EngineSnapshot {
            call: phone_core::CallState::Incoming,
            call_id: Some("PRIVATE_CALL_TOKEN".into()),
            caller: Some(phone_core::CallerIdentity {
                name: Some("PRIVATE_CALLER".into()),
                number: Some("+4930555000".into()),
            }),
            accounts: vec![phone_core::AccountSnapshot {
                id: 7,
                name: "PRIVATE_ACCOUNT".into(),
                registrar: "private.example.test".into(),
                username: "PRIVATE_USER".into(),
                enabled: true,
                enabled_changing: false,
                registered_at_ms: Some(12345),
                registration: phone_core::RegistrationState::Registered,
                last_error: Some(PhoneError::new(
                    ErrorCode::CredentialAccessRequired,
                    "PRIVATE_ERROR_PAYLOAD",
                )),
                removing: false,
            }],
            ..Default::default()
        };
        let mut unregistered = snapshot.accounts[0].clone();
        unregistered.id = 8;
        unregistered.registration = phone_core::RegistrationState::Failed;
        unregistered.last_error = None;
        snapshot.accounts.push(unregistered);
        snapshot.caller_contact = Some(phone_core::ContactReference {
            id: "PRIVATE_CONTACT_ID".into(),
            source: phone_core::ContactSource::Macos,
            name: "PRIVATE_CONTACT_NAME".into(),
        });
        let report = diagnostic_report(
            Some(snapshot),
            Ok(phone_core::NotificationPermission::Denied),
            Ok(phone_core::MicrophonePermission::Authorized),
            Ok(AudioDevices::default()),
            None,
        );
        assert_eq!(report.accounts_total, Some(2));
        assert_eq!(report.accounts_registered, Some(1));
        assert_eq!(report.call_active, Some(true));
        assert_eq!(
            report.credential_error,
            Some(ErrorCode::CredentialAccessRequired)
        );
        assert_eq!(report.audio_devices_available, Some(false));
        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 12);
        let text = serde_json::to_string(&report).unwrap();
        assert!(text.len() <= 1024);
        for private in ["PRIVATE_", "private.example.test", "+4930555000", "12345"] {
            assert!(!text.contains(private));
        }
    }
    #[test]
    fn unavailable_readings_remain_unknown_and_errors_are_classified_only() {
        let report = diagnostic_report(
            None,
            Err(PhoneError::new(
                ErrorCode::NativeFailure,
                "PRIVATE_NATIVE_DETAIL",
            )),
            Err(PhoneError::new(
                ErrorCode::NativeFailure,
                "PRIVATE_MIC_DETAIL",
            )),
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "PRIVATE_DEVICE_DETAIL",
            )),
            Some(PhoneError::new(
                ErrorCode::StorageFailure,
                "PRIVATE_STORAGE_DETAIL",
            )),
        );
        assert_eq!(report.accounts_total, None);
        assert_eq!(report.accounts_registered, None);
        assert_eq!(report.call_active, None);
        assert_eq!(report.notification_permission, None);
        assert_eq!(report.microphone_permission, None);
        assert_eq!(report.audio_devices_available, None);
        assert_eq!(report.credential_error, None);
        assert!(!report.history_available);
        assert_eq!(report.last_error, Some(ErrorCode::StorageFailure));
        assert!(!serde_json::to_string(&report).unwrap().contains("PRIVATE_"));
    }
}
