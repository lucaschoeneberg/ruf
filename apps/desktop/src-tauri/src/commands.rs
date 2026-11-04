use crate::phone::{PhoneState, active_engine};
use baresip_adapter::Engine;
use phone_core::{AccountConfig, AppInfo, AudioMode, EngineSnapshot, ErrorCode, PhoneError};
use tauri::Manager;

/// Read startup information without accessing credentials or initializing telephony.
#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        telephony: baresip_adapter::status(),
    }
}

/// Configure one ephemeral account. Password is never returned or persisted.
#[tauri::command]
pub async fn configure_account(
    app: tauri::AppHandle,
    state: tauri::State<'_, PhoneState>,
    account: AccountConfig,
) -> Result<EngineSnapshot, PhoneError> {
    account.validate()?;
    let recording = if matches!(account.audio_mode, AudioMode::TestTone) {
        let directory = app.path().app_cache_dir().map_err(|_| {
            PhoneError::new(
                ErrorCode::NativeFailure,
                "The application cache directory is unavailable.",
            )
        })?;
        Some(directory.join("received.wav"))
    } else {
        None
    };
    state
        .execute(move |slot| {
            if let Some(existing) = slot.as_mut() {
                existing.configure_account(&account)?;
                return existing.snapshot();
            }
            if let Some(directory) = recording.as_ref().and_then(|path| path.parent()) {
                std::fs::create_dir_all(directory).map_err(|_| {
                    PhoneError::new(
                        ErrorCode::NativeFailure,
                        "Cannot create the test-audio directory.",
                    )
                })?;
            }
            let mut engine = Engine::start(account.audio_mode, recording)?;
            engine.configure_account(&account)?;
            let snapshot = engine.snapshot()?;
            *slot = Some(engine);
            Ok(snapshot)
        })
        .await
}

/// Read real engine state without credentials; safe to poll during a lab call.
#[tauri::command]
pub async fn get_engine_snapshot(
    state: tauri::State<'_, PhoneState>,
) -> Result<EngineSnapshot, PhoneError> {
    state
        .execute(|slot| {
            slot.as_ref()
                .map_or_else(|| Ok(EngineSnapshot::default()), Engine::snapshot)
        })
        .await
}

/// Queue a validated number/extension. Connection follows through native state.
#[tauri::command]
pub async fn dial(
    state: tauri::State<'_, PhoneState>,
    destination: String,
) -> Result<EngineSnapshot, PhoneError> {
    state
        .execute(move |slot| {
            let engine = active_engine(slot)?;
            engine.dial(&destination)?;
            engine.snapshot()
        })
        .await
}

/// Accept the single currently offered incoming call.
#[tauri::command]
pub async fn answer(state: tauri::State<'_, PhoneState>) -> Result<EngineSnapshot, PhoneError> {
    state
        .execute(|slot| {
            let engine = active_engine(slot)?;
            engine.answer()?;
            engine.snapshot()
        })
        .await
}

/// Reject or end the current call and let its native close event update state.
#[tauri::command]
pub async fn hangup(state: tauri::State<'_, PhoneState>) -> Result<EngineSnapshot, PhoneError> {
    state
        .execute(|slot| {
            let engine = active_engine(slot)?;
            engine.hangup()?;
            engine.snapshot()
        })
        .await
}
