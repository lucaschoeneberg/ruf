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
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| crate::store::storage_error())?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state
        .execute(move |session| {
            session.load(directory, cache)?;
            operation(session)
        })
        .await
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
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| crate::store::storage_error())?;
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|_| crate::store::storage_error())?;
    state
        .save_account(directory, cache, id, name, account)
        .await
}

/// Compatibility command: save/replace the selected account using its username as name.
#[tauri::command]
pub async fn configure_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    account: AccountConfig,
) -> Result<EngineSnapshot, PhoneError> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| crate::store::storage_error())?;
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

/// Reject or end the current call; completion follows through native state.
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
    if !matches!(section.as_str(), "general" | "accounts" | "audio" | "about") {
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
    .title("SIP · Einstellungen")
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
