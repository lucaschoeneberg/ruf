//! Password-free local preferences using the existing session and ordered history worker.
use crate::{desktop::with_session, store};
use phone_core::{AppPreferences, PhoneError};

// Serialize full preference saves without holding the phone mutex while waiting
// for SQLite. Native callbacks continue to enqueue observations throughout.
static UPDATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Read local preferences; no OS permission request or credential write is made.
#[tauri::command]
pub(crate) async fn get_preferences(app: tauri::AppHandle) -> Result<AppPreferences, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        store::AccountStore {
            path: store::data_directory(&app)?.join("accounts.json"),
        }
        .load()
        .map(|settings| settings.preferences)
    })
    .await
    .map_err(|_| store::storage_error())?
}

/// Save the complete required preference object. Confirmation defaults to false.
/// History changes share the ordered lifecycle queue; shortening retention that
/// would remove completed records requires `confirm_history_deletion`.
#[tauri::command]
pub(crate) async fn save_preferences(
    app: tauri::AppHandle,
    preferences: AppPreferences,
    confirm_history_deletion: Option<bool>,
) -> Result<AppPreferences, PhoneError> {
    tauri::async_runtime::spawn_blocking(move || {
        preferences.history.validate()?;
        let _update = UPDATE.lock().map_err(|_| store::storage_error())?;
        let previous = with_session(&app, |session| Ok(session.preferences()))?;
        if previous.history == preferences.history {
            with_session(&app, |session| {
                session.save_preferences(preferences.clone())
            })?;
        } else {
            let persistence_app = app.clone();
            let next = preferences.clone();
            let result = crate::history::configure_preferences(
                &app,
                preferences.history.clone(),
                confirm_history_deletion.unwrap_or(false),
                move || with_session(&persistence_app, |session| session.save_preferences(next)),
            );
            if let Err(error) = result {
                // A SQLite commit may fail after metadata was replaced. Restore the
                // previous metadata in that case; validation/write failures have
                // already kept the prior values and require no extra write.
                with_session(&app, |session| {
                    if session.preferences() == preferences {
                        session.rollback_preferences(&preferences, previous)?;
                    }
                    Ok(())
                })?;
                return Err(error);
            }
        }
        crate::contacts::update_matching(&app, preferences.contacts.match_german_numbers);
        Ok(preferences)
    })
    .await
    .map_err(|_| store::storage_error())?
}
