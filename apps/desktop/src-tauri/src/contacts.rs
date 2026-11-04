//! Read-only system contacts and app-local favorite identifiers.

use phone_core::{
    Contact, ContactsPermission, ContactsSnapshot, EngineSnapshot, ErrorCode, PhoneError,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};

/// Serialize native reads and favorite mutations without blocking the UI/phone worker.
#[derive(Clone, Default)]
pub(crate) struct ContactsState {
    gate: Arc<Mutex<()>>,
    cache: Arc<RwLock<Option<ContactsSnapshot>>>,
    error: Arc<RwLock<Option<PhoneError>>>,
    stopped: Arc<AtomicBool>,
}

/// Authorized in-memory records only; source names/numbers are never persisted.
pub(crate) fn cached_contacts(app: &tauri::AppHandle) -> Vec<Contact> {
    app.state::<ContactsState>()
        .cache
        .read()
        .ok()
        .and_then(|cache| {
            cache
                .as_ref()
                .filter(|s| s.permission == ContactsPermission::Authorized)
                .map(|s| s.contacts.clone())
        })
        .unwrap_or_default()
}

pub(crate) fn enrich_snapshot(app: &tauri::AppHandle, snapshot: &mut EngineSnapshot) {
    snapshot.caller_contact = app
        .state::<ContactsState>()
        .cache
        .read()
        .ok()
        .and_then(|cache| {
            let contacts = cache
                .as_ref()
                .filter(|snapshot| snapshot.permission == ContactsPermission::Authorized)?;
            let number = snapshot.caller.as_ref()?.number.as_deref()?;
            phone_core::resolve_contact(&contacts.contacts, number)
        });
    snapshot.history_error = crate::history::recording_error(app);
}

fn refresh_locked(
    app: &tauri::AppHandle,
    state: &ContactsState,
    request_access: bool,
) -> Result<ContactsSnapshot, PhoneError> {
    let result = (|| {
        let mut snapshot = baresip_adapter::read_contacts(request_access)?;
        if snapshot.permission == ContactsPermission::Authorized {
            let favorites =
                load_favorites(&crate::store::data_directory(app)?.join("contact-favorites.json"))?;
            for contact in &mut snapshot.contacts {
                contact.favorite = favorites.contains(&contact.id);
            }
        }
        Ok(snapshot)
    })();
    let changed = publish(state, &result)?;
    if changed {
        let _ = app.emit("contacts-changed", ());
    }
    result
}

// Replace the projection atomically even on denial/failure; stale authorized
// names must never survive a failed refresh or a revoked OS permission.
fn publish(
    state: &ContactsState,
    result: &Result<ContactsSnapshot, PhoneError>,
) -> Result<bool, PhoneError> {
    let mut cache = state.cache.write().map_err(|_| storage_error())?;
    let next = result.as_ref().ok().cloned();
    let changed = *cache != next;
    *cache = next;
    *state.error.write().map_err(|_| storage_error())? = result.as_ref().err().cloned();
    Ok(changed)
}

/// Subscribe to native revision changes; permission checks never request access.
pub(crate) fn setup(app: &tauri::AppHandle) {
    let app = app.clone();
    let state = app.state::<ContactsState>().inner().clone();
    std::thread::spawn(move || {
        let mut revision = u64::MAX;
        let mut permission = None;
        let mut ticks = 0_u8;
        while !state.stopped.load(Ordering::Relaxed) {
            let next_revision = baresip_adapter::contacts_revision();
            if (next_revision != revision || ticks == 0)
                && let Ok(_guard) = state.gate.try_lock()
            {
                let current_permission = baresip_adapter::contacts_permission(false).ok();
                let missing = state.cache.read().is_ok_and(|cache| cache.is_none());
                if next_revision != revision || current_permission != permission || missing {
                    let _ = refresh_locked(&app, &state, false);
                }
                revision = next_revision;
                permission = current_permission;
            }
            ticks = (ticks + 1) % 4;
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}

pub(crate) fn shutdown(app: &tauri::AppHandle) {
    app.state::<ContactsState>()
        .stopped
        .store(true, Ordering::Relaxed);
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    version: u8,
    favorites: BTreeSet<String>,
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control)
}

fn storage_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::StorageFailure,
        "Contact favorites could not be read or saved in the application data directory.",
    )
}

fn load_favorites(path: &Path) -> Result<BTreeSet<String>, PhoneError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(_) => return Err(storage_error()),
    };
    let preferences: Preferences = serde_json::from_slice(&bytes).map_err(|_| storage_error())?;
    if preferences.version != 1
        || preferences.favorites.len() > 1000
        || preferences.favorites.iter().any(|id| !valid_id(id))
    {
        return Err(storage_error());
    }
    Ok(preferences.favorites)
}

// Call only while ContactsState is locked, so read/modify/replace cannot lose updates.
fn write_favorite(path: &Path, id: &str, favorite: bool) -> Result<(), PhoneError> {
    if !valid_id(id) {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "Contact identifier must be 1–1024 UTF-8 bytes without control characters.",
        ));
    }
    let mut favorites = load_favorites(path)?;
    if favorite {
        favorites.insert(id.to_owned());
    } else {
        favorites.remove(id);
    }
    if favorites.len() > 1000 {
        return Err(PhoneError::new(
            ErrorCode::InvalidInput,
            "At most 1000 contact favorites can be saved.",
        ));
    }
    let parent = path.parent().ok_or_else(storage_error)?;
    fs::create_dir_all(parent).map_err(|_| storage_error())?;
    let bytes = serde_json::to_vec(&Preferences {
        version: 1,
        favorites,
    })
    .map_err(|_| storage_error())?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|_| storage_error())?;
    fs::rename(&temporary, path).map_err(|_| storage_error())
}

async fn read(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
    request_access: bool,
    force: bool,
) -> Result<ContactsSnapshot, PhoneError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state.gate.lock().map_err(|_| storage_error())?;
        if !force && !request_access {
            if let Some(snapshot) = state.cache.read().map_err(|_| storage_error())?.clone() {
                return Ok(snapshot);
            }
            if let Some(error) = state.error.read().map_err(|_| storage_error())?.clone() {
                return Err(error);
            }
        }
        refresh_locked(&app, &state, request_access)
    })
    .await
    .map_err(|_| storage_error())?
}

/// Explicitly reload current permission, contacts and favorites without prompting.
#[tauri::command]
pub(crate) async fn refresh_contacts(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
) -> Result<ContactsSnapshot, PhoneError> {
    read(app, state, false, true).await
}

/// Copy an explicitly chosen original phone number via the native clipboard.
#[tauri::command]
pub(crate) async fn copy_phone_number(number: String) -> Result<(), PhoneError> {
    tauri::async_runtime::spawn_blocking(move || baresip_adapter::copy_phone_number(&number))
        .await
        .map_err(|_| storage_error())?
}

/// Open the fixed macOS Contacts application, without arbitrary URLs or source edits.
#[tauri::command]
pub(crate) async fn open_contacts_app() -> Result<(), PhoneError> {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            if std::process::Command::new("/usr/bin/open")
                .args(["-b", "com.apple.AddressBook"])
                .status()
                .is_ok_and(|s| s.success())
            {
                Ok(())
            } else {
                Err(PhoneError::new(
                    ErrorCode::NativeFailure,
                    "The macOS Contacts application could not be opened.",
                ))
            }
        })
        .await
        .map_err(|_| storage_error())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "The macOS Contacts application is unavailable on this platform.",
        ))
    }
}

/// Read the accessible macOS phonebook and local favorite flags without prompting.
/// Returns a permission state and no contacts on unsupported, undecided or denied access.
#[tauri::command]
pub(crate) async fn get_contacts(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
) -> Result<ContactsSnapshot, PhoneError> {
    read(app, state, false, false).await
}

/// Explicit user action: ask for macOS contact access if undecided, then read the phonebook.
/// A denial is a normal permission result; previously denied access never prompts again.
#[tauri::command]
pub(crate) async fn request_contacts_access(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
) -> Result<ContactsSnapshot, PhoneError> {
    read(app, state, true, true).await
}

async fn permission(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
    request_access: bool,
) -> Result<ContactsPermission, PhoneError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state.gate.lock().map_err(|_| storage_error())?;
        let result = baresip_adapter::contacts_permission(request_access);
        if !result
            .as_ref()
            .is_ok_and(|permission| *permission == ContactsPermission::Authorized)
        {
            let snapshot = result
                .as_ref()
                .map(|permission| ContactsSnapshot {
                    permission: *permission,
                    contacts: vec![],
                })
                .map_err(Clone::clone);
            if publish(&state, &snapshot)? {
                let _ = app.emit("contacts-changed", ());
            }
        }
        result
    })
    .await
    .map_err(|_| storage_error())?
}

/// Read the current OS permission without prompting, fetching contacts or reading favorites.
#[tauri::command]
pub(crate) async fn get_contacts_permission(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
) -> Result<ContactsPermission, PhoneError> {
    permission(app, state, false).await
}

/// Explicit user action: request undecided OS access and return only its current status.
/// Previously denied access does not prompt again; no contact records are fetched.
#[tauri::command]
pub(crate) async fn request_contacts_permission(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
) -> Result<ContactsPermission, PhoneError> {
    permission(app, state, true).await
}

/// Open the fixed macOS Contacts privacy pane; no caller-controlled URL or shell arguments.
/// Does not change permission. macOS and the user retain control over access.
#[tauri::command]
pub(crate) async fn open_contacts_settings() -> Result<(), PhoneError> {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            let opened = std::process::Command::new("/usr/bin/open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Contacts")
                .status()
                .is_ok_and(|status| status.success());
            if opened {
                Ok(())
            } else {
                Err(PhoneError::new(
                    ErrorCode::NativeFailure,
                    "macOS Contacts privacy settings could not be opened.",
                ))
            }
        })
        .await
        .map_err(|_| storage_error())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "macOS contact settings are unavailable on this platform.",
        ))
    }
}

/// Save an app-local favorite by opaque macOS contact ID; never edits the system address book.
/// Validates nonempty IDs up to 1024 bytes, forbids controls and caps favorites at 1000.
#[tauri::command]
pub(crate) async fn set_contact_favorite(
    app: tauri::AppHandle,
    state: tauri::State<'_, ContactsState>,
    id: String,
    favorite: bool,
) -> Result<(), PhoneError> {
    if !cfg!(target_os = "macos") {
        return Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "System contacts are currently available only on macOS.",
        ));
    }
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = state.gate.lock().map_err(|_| storage_error())?;
        let directory = crate::store::data_directory(&app)?;
        write_favorite(&directory.join("contact-favorites.json"), &id, favorite)?;
        if let Some(snapshot) = state.cache.write().map_err(|_| storage_error())?.as_mut() {
            for contact in &mut snapshot.contacts {
                if contact.id == id {
                    contact.favorite = favorite;
                }
            }
        }
        let _ = app.emit("contacts-changed", ());
        Ok(())
    })
    .await
    .map_err(|_| storage_error())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Directory(std::path::PathBuf);
    impl Directory {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            Self(std::env::temp_dir().join(format!(
                "sip-contact-tests-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            )))
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn cache_revocation_and_read_failure_remove_authorized_records() {
        let state = ContactsState::default();
        let authorized = ContactsSnapshot {
            permission: ContactsPermission::Authorized,
            contacts: vec![phone_core::Contact {
                id: "synthetic-contact".into(),
                source: phone_core::ContactSource::Macos,
                name: "Synthetic name".into(),
                organization: String::new(),
                phones: vec![],
                favorite: false,
            }],
        };
        assert!(publish(&state, &Ok(authorized.clone())).unwrap());
        assert!(!publish(&state, &Ok(authorized.clone())).unwrap());
        assert!(
            publish(
                &state,
                &Ok(ContactsSnapshot {
                    permission: ContactsPermission::Denied,
                    contacts: vec![],
                })
            )
            .unwrap()
        );
        assert!(
            state
                .cache
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .contacts
                .is_empty()
        );
        publish(&state, &Ok(authorized)).unwrap();
        assert!(publish(&state, &Err(storage_error())).unwrap());
        assert!(state.cache.read().unwrap().is_none());
        assert!(state.error.read().unwrap().is_some());
        publish(
            &state,
            &Ok(ContactsSnapshot {
                permission: ContactsPermission::Authorized,
                contacts: vec![],
            }),
        )
        .unwrap();
        assert!(state.error.read().unwrap().is_none());
    }

    #[test]
    fn favorites_round_trip_without_storing_contact_names_or_numbers() {
        let directory = Directory::new();
        let path = directory.0.join("contact-favorites.json");
        assert!(load_favorites(&path).unwrap().is_empty());
        write_favorite(&path, "source-id-a", true).unwrap();
        write_favorite(&path, "source-id-b", true).unwrap();
        write_favorite(&path, "source-id-a", true).unwrap();
        assert_eq!(load_favorites(&path).unwrap().len(), 2);
        write_favorite(&path, "source-id-a", false).unwrap();
        assert_eq!(
            load_favorites(&path).unwrap(),
            ["source-id-b".to_owned()].into()
        );
        let data = std::fs::read_to_string(path).unwrap();
        assert!(!data.contains("name"));
        assert!(!data.contains("number"));
    }

    #[test]
    fn corrupt_or_unsupported_preferences_are_never_overwritten() {
        let directory = Directory::new();
        std::fs::create_dir_all(&directory.0).unwrap();
        let path = directory.0.join("contact-favorites.json");
        for data in [
            "broken",
            r#"{"version":2,"favorites":[]}"#,
            r#"{"version":1,"favorites":[""]}"#,
        ] {
            std::fs::write(&path, data).unwrap();
            assert!(write_favorite(&path, "source-id", true).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), data);
        }
    }

    #[test]
    fn invalid_identifiers_are_rejected_before_creating_storage() {
        let directory = Directory::new();
        let path = directory.0.join("contact-favorites.json");
        for id in ["", "bad\nidentifier", &"a".repeat(1025)] {
            assert_eq!(
                write_favorite(&path, id, true).unwrap_err().code,
                phone_core::ErrorCode::InvalidInput
            );
        }
        assert!(!path.exists());
    }
}
