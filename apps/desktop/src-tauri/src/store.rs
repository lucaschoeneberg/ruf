//! Versioned, password-free metadata and OS credential storage. No webview access.
use phone_core::{AudioMode, ErrorCode, PhoneError, SoundSettings};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::Manager;

/// Use this bundle's own application-data directory without importing old SIP data.
pub(crate) fn data_directory(app: &tauri::AppHandle) -> Result<PathBuf, PhoneError> {
    app.path().app_data_dir().map_err(|_| storage_error())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SavedAccount {
    pub key: String,
    pub name: String,
    pub registrar: String,
    pub username: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SavedSettings {
    pub version: u32,
    pub accounts: Vec<SavedAccount>,
    pub selected_key: Option<String>,
    pub audio_mode: AudioMode,
    #[serde(default)]
    pub pending_deletions: Vec<String>,
    #[serde(default)]
    pub sound: SoundSettings,
    #[serde(default)]
    pub desktop: DesktopPreferences,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DesktopPreferences {
    pub notifications_enabled: bool,
    pub autostart_configured: bool,
}
impl Default for DesktopPreferences {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            autostart_configured: false,
        }
    }
}
impl Default for SavedSettings {
    fn default() -> Self {
        Self {
            version: 1,
            accounts: Vec::new(),
            selected_key: None,
            audio_mode: AudioMode::System,
            pending_deletions: Vec::new(),
            sound: SoundSettings::default(),
            desktop: DesktopPreferences::default(),
        }
    }
}

fn valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 80 && key.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

pub(crate) struct AccountStore {
    pub path: PathBuf,
}
impl AccountStore {
    pub fn load(&self) -> Result<SavedSettings, PhoneError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SavedSettings::default());
            }
            Err(_) => return Err(storage_error()),
        };
        let settings: SavedSettings =
            serde_json::from_slice(&bytes).map_err(|_| storage_error())?;
        settings.sound.validate().map_err(|_| storage_error())?;
        if settings.version != 1 || settings.accounts.len() > 8 {
            return Err(storage_error());
        }
        if settings
            .pending_deletions
            .iter()
            .any(|key| !valid_key(key) || settings.accounts.iter().any(|a| a.key == *key))
        {
            return Err(storage_error());
        }
        for (index, account) in settings.accounts.iter().enumerate() {
            // Keys become credential identifiers only; never filenames or arbitrary services.
            if !valid_key(&account.key)
                || settings.accounts[..index].iter().any(|a| {
                    a.key == account.key
                        || (a.registrar.eq_ignore_ascii_case(&account.registrar)
                            && a.username == account.username)
                })
            {
                return Err(storage_error());
            }
        }
        Ok(settings)
    }
    pub fn write(&self, settings: &SavedSettings) -> Result<(), PhoneError> {
        let parent = self.path.parent().ok_or_else(storage_error)?;
        fs::create_dir_all(parent).map_err(|_| storage_error())?;
        let bytes = serde_json::to_vec_pretty(settings).map_err(|_| storage_error())?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, bytes).map_err(|_| storage_error())?;
        // Same-directory replacement is atomic on macOS and Windows.
        fs::rename(&temporary, &self.path).map_err(|_| storage_error())
    }
}

/// Internal credential boundary; system calls may display OS authorization UI.
pub(crate) trait CredentialStore: Send + Sync {
    fn read(&self, key: &str, interactive: bool) -> Result<String, PhoneError>;
    fn write(&self, key: &str, password: &str) -> Result<(), PhoneError>;
    fn delete(&self, key: &str) -> Result<(), PhoneError>;
}

pub(crate) struct SystemCredentials;
impl CredentialStore for SystemCredentials {
    fn read(&self, key: &str, interactive: bool) -> Result<String, PhoneError> {
        credential_operation(interactive, || read_password(key))
    }
    fn write(&self, key: &str, password: &str) -> Result<(), PhoneError> {
        credential_operation(true, || write_password(key, password))
    }
    fn delete(&self, key: &str) -> Result<(), PhoneError> {
        credential_operation(false, || delete_password(key))
    }
}

// macOS authorization policy is process-global, so never change it while another
// credential operation owns an interactive prompt. Session-locked callers fail
// promptly instead of waiting for that prompt.
static CREDENTIAL_ACCESS: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn credential_operation<T>(
    interactive: bool,
    operation: impl FnOnce() -> Result<T, PhoneError>,
) -> Result<T, PhoneError> {
    let _access = if interactive {
        CREDENTIAL_ACCESS.lock().map_err(|_| storage_error())?
    } else {
        CREDENTIAL_ACCESS.try_lock().map_err(|_| storage_error())?
    };
    #[cfg(target_os = "macos")]
    let _ui = if !interactive
        && security_framework::os::macos::keychain::SecKeychain::user_interaction_allowed()
            .map_err(|_| storage_error())?
    {
        Some(
            security_framework::os::macos::keychain::SecKeychain::disable_user_interaction()
                .map_err(|_| storage_error())?,
        )
    } else {
        None
    };
    operation()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn entry(key: &str) -> Result<keyring::Entry, PhoneError> {
    keyring::Entry::new("org.rufapp.desktop", key).map_err(|_| storage_error())
}
fn read_password(key: &str) -> Result<String, PhoneError> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        entry(key)?.get_password().map_err(|_| storage_error())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = key;
        Err(storage_error())
    }
}
fn write_password(key: &str, password: &str) -> Result<(), PhoneError> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        entry(key)?
            .set_password(password)
            .map_err(|_| storage_error())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (key, password);
        Err(storage_error())
    }
}
fn delete_password(key: &str) -> Result<(), PhoneError> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(storage_error()),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = key;
        Err(storage_error())
    }
}
pub(crate) fn storage_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::StorageFailure,
        "Account storage failed. Check access to the application data folder and system credential store.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noninteractive_credential_access_defers_during_authorization() {
        use std::{sync::mpsc, time::Duration};
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let authorization = std::thread::spawn(move || {
            credential_operation(true, || {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let (result_tx, result_rx) = mpsc::channel();
        let background = std::thread::spawn(move || {
            result_tx
                .send(credential_operation(false, || Ok(())))
                .unwrap();
        });
        let result = result_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        authorization.join().unwrap().unwrap();
        background.join().unwrap();
        assert!(
            result.is_ok(),
            "background credentials waited for foreground authorization"
        );
        assert_eq!(result.unwrap().unwrap_err().code, ErrorCode::StorageFailure);
    }

    #[test]
    fn metadata_roundtrip_and_replacement_contain_no_password() {
        let directory = std::env::temp_dir().join(format!("sip-store-{}", std::process::id()));
        let store = AccountStore {
            path: directory.join("accounts.json"),
        };
        let mut settings = SavedSettings::default();
        settings.accounts.push(SavedAccount {
            key: "abcd-1234".into(),
            name: "Büro".into(),
            registrar: "pbx.test".into(),
            username: "1000".into(),
        });
        store.write(&settings).unwrap();
        settings.accounts[0].name = "Privat".into();
        store.write(&settings).unwrap();
        let bytes = fs::read_to_string(&store.path).unwrap();
        assert!(!bytes.contains("password"));
        assert_eq!(store.load().unwrap().accounts[0].name, "Privat");
        // Existing v1 metadata predates sound preferences and must keep loading.
        let mut legacy = serde_json::to_value(&settings).unwrap();
        legacy.as_object_mut().unwrap().remove("sound");
        legacy.as_object_mut().unwrap().remove("desktop");
        fs::write(&store.path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        assert_eq!(store.load().unwrap().sound, SoundSettings::default());
        assert!(store.load().unwrap().desktop.notifications_enabled);
        assert!(!store.load().unwrap().desktop.autostart_configured);
        fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    #[ignore = "Writes only a unique synthetic entry to the actual OS credential store"]
    fn system_keychain_roundtrip_and_delete() {
        let key = format!(
            "test-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        write_password(&key, "synthetic-keychain-test").unwrap();
        assert_eq!(read_password(&key).unwrap(), "synthetic-keychain-test");
        delete_password(&key).unwrap();
        assert!(read_password(&key).is_err());
    }
}
