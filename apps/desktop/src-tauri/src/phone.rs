use crate::store::{self, AccountStore, SavedAccount, SavedSettings};
use baresip_adapter::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, PhoneError, SoundSettings,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct PhoneSession {
    engine: Option<Engine>,
    store: Option<AccountStore>,
    settings: SavedSettings,
    // Session identities map to stable, opaque credential keys in saved metadata.
    keys: BTreeMap<u32, String>,
    recording: Option<PathBuf>,
    storage_warning: Option<PhoneError>,
}

impl PhoneSession {
    pub fn desktop_preferences(&self) -> crate::store::DesktopPreferences {
        self.settings.desktop.clone()
    }
    pub fn save_desktop_preferences(
        &mut self,
        preferences: crate::store::DesktopPreferences,
    ) -> Result<(), PhoneError> {
        let mut next = self.settings.clone();
        next.desktop = preferences;
        self.write(&next)?;
        self.settings = next;
        Ok(())
    }
    pub fn load(&mut self, directory: PathBuf, cache: PathBuf) -> Result<(), PhoneError> {
        if self.store.is_some() {
            return Ok(());
        }
        // Startup restoration is always noninteractive, even if a webview wins
        // the race with the background observer. Explicit account saves may ask.
        #[cfg(target_os = "macos")]
        let _keychain_ui =
            security_framework::os::macos::keychain::SecKeychain::disable_user_interaction()
                .map_err(|_| store::storage_error())?;
        let store = AccountStore {
            path: directory.join("accounts.json"),
        };
        let settings = store.load()?;
        self.recording = Some(cache.join("received.wav"));
        if !settings.accounts.is_empty() {
            let mut engine = self.start_engine(settings.audio_mode, &settings.sound)?;
            let mut keys = BTreeMap::new();
            for saved in &settings.accounts {
                let password = store::read_password(&saved.key);
                let config = AccountConfig {
                    registrar: saved.registrar.clone(),
                    username: saved.username.clone(),
                    password: password
                        .as_ref()
                        .cloned()
                        .unwrap_or_else(|_| "unavailable".into()),
                    audio_mode: settings.audio_mode,
                };
                let id = match password {
                    Ok(_) => engine.save_account(None, &saved.name, &config)?,
                    Err(error) => {
                        engine.restore_unavailable_account(&saved.name, &config, error)?
                    }
                };
                keys.insert(id, saved.key.clone());
            }
            if let Some(id) = keys
                .iter()
                .find_map(|(id, key)| (Some(key) == settings.selected_key.as_ref()).then_some(*id))
            {
                engine.select_account(id)?;
            }
            self.engine = Some(engine);
            self.keys = keys;
        }
        self.settings = settings;
        self.store = Some(store);
        self.cleanup_credentials();
        Ok(())
    }
    fn start_engine(&self, mode: AudioMode, sound: &SoundSettings) -> Result<Engine, PhoneError> {
        let recording = if mode == AudioMode::TestTone {
            let path = self.recording.clone().ok_or_else(store::storage_error)?;
            std::fs::create_dir_all(path.parent().ok_or_else(store::storage_error)?)
                .map_err(|_| store::storage_error())?;
            Some(path)
        } else {
            None
        };
        let mut engine = Engine::start(mode, recording)?;
        engine.restore_sound_settings(sound)?;
        Ok(engine)
    }
    pub fn snapshot(&self) -> Result<EngineSnapshot, PhoneError> {
        let mut snapshot = self
            .engine
            .as_ref()
            .map_or_else(|| Ok(EngineSnapshot::default()), Engine::snapshot)?;
        if self.engine.is_none() {
            snapshot.sound_settings = self.settings.sound.clone();
        }
        if self.storage_warning.is_some() {
            snapshot.last_error = self.storage_warning.clone();
        }
        Ok(snapshot)
    }
    fn cleanup_credentials(&mut self) {
        let mut next = self.settings.clone();
        next.pending_deletions
            .retain(|key| store::delete_password(key).is_err());
        let pending = !next.pending_deletions.is_empty();
        if next.pending_deletions != self.settings.pending_deletions {
            if self.write(&next).is_err() {
                self.storage_warning = Some(store::storage_error());
                return;
            }
            self.settings = next;
        }
        self.storage_warning = pending.then(|| PhoneError::new(ErrorCode::CredentialCleanupPending, "Obsolete passwords are queued for deletion; retry after unlocking the system credential store."));
    }
    pub fn engine(&mut self) -> Result<&mut Engine, PhoneError> {
        self.engine.as_mut().ok_or_else(|| {
            PhoneError::new(
                ErrorCode::InvalidState,
                "Configure an account before using the engine.",
            )
        })
    }
    fn write(&self, settings: &SavedSettings) -> Result<(), PhoneError> {
        self.store
            .as_ref()
            .ok_or_else(store::storage_error)?
            .write(settings)
    }

    pub fn save(
        &mut self,
        id: Option<u32>,
        name: String,
        mut account: AccountConfig,
    ) -> Result<EngineSnapshot, PhoneError> {
        let old_key = id
            .map(|id| {
                self.keys
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| PhoneError::new(ErrorCode::InvalidInput, "Unknown account."))
            })
            .transpose()?;
        if account.password.is_empty() {
            account.password = store::read_password(old_key.as_deref().ok_or_else(|| {
                PhoneError::new(ErrorCode::InvalidInput, "New accounts require a password.")
            })?)?;
        }
        account.validate()?;
        let snapshot = self.snapshot()?;
        if snapshot.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before changing accounts.",
            ));
        }
        if snapshot
            .audio_mode
            .is_some_and(|mode| mode != account.audio_mode)
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Restart the app to change its audio backend.",
            ));
        }
        let name = name.trim().to_string();
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
            || (id.is_none() && snapshot.accounts.len() >= 8)
            || snapshot.accounts.iter().any(|a| {
                Some(a.id) != id
                    && a.registrar.eq_ignore_ascii_case(&account.registrar)
                    && a.username == account.username
            })
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Invalid name, duplicate identity or account limit reached.",
            ));
        }
        if self.engine.is_none() {
            self.engine = Some(self.start_engine(account.audio_mode, &self.settings.sound)?);
        }
        // Immutable credential versions keep the old saved account usable until metadata commits.
        let key = format!(
            "{:x}-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| store::storage_error())?
                .as_nanos(),
            std::process::id()
        );
        // Journal the key before touching the keychain, so interrupted/failed saves
        // always leave a durable reference for secret cleanup on the next launch.
        let mut journal = self.settings.clone();
        journal.pending_deletions.push(key.clone());
        self.write(&journal)?;
        self.settings = journal;
        store::write_password(&key, &account.password)?;
        let mut next = self.settings.clone();
        let saved = SavedAccount {
            key: key.clone(),
            name: name.clone(),
            registrar: account.registrar.clone(),
            username: account.username.clone(),
        };
        if let Some(existing) = next
            .accounts
            .iter_mut()
            .find(|a| Some(&a.key) == old_key.as_ref())
        {
            *existing = saved;
        } else {
            next.accounts.push(saved);
        }
        next.pending_deletions.retain(|pending| pending != &key);
        if let Some(old_key) = &old_key {
            next.pending_deletions.push(old_key.clone());
        }
        next.audio_mode = account.audio_mode;
        if next.selected_key.is_none() || next.selected_key == old_key {
            next.selected_key = Some(key.clone());
        }
        if let Err(error) = self.write(&next) {
            self.cleanup_credentials();
            return Err(error);
        }
        let saved_id = match self
            .engine()?
            .save_account(id, &name, &account)
            .and_then(|id| {
                self.engine()?.wait_account_update(id)?;
                Ok(id)
            }) {
            Ok(id) => id,
            Err(error) => {
                self.write(&self.settings)?;
                self.cleanup_credentials();
                return Err(error);
            }
        };
        self.settings = next;
        self.keys.insert(saved_id, key);
        self.cleanup_credentials();
        self.snapshot()
    }

    pub fn save_sound(&mut self, sound: SoundSettings) -> Result<EngineSnapshot, PhoneError> {
        sound.validate()?;
        if self.snapshot()?.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the call before changing sound.",
            ));
        }
        let devices = baresip_adapter::audio_devices()?;
        for (id, available) in [
            (&sound.input_device, &devices.inputs),
            (&sound.output_device, &devices.outputs),
            (&sound.ring_device, &devices.outputs),
        ] {
            if !id.is_empty()
                && id != "default"
                && !available
                    .iter()
                    .any(|device| device.id == *id && device.selectable)
            {
                return Err(PhoneError::new(
                    ErrorCode::InvalidInput,
                    "Selected audio device is unavailable; refresh the device list.",
                ));
            }
        }
        let mut next = self.settings.clone();
        next.sound = sound.clone();
        self.write(&next)?;
        if let Some(engine) = self.engine.as_mut()
            && let Err(error) = engine.set_sound_settings(&sound)
        {
            self.write(&self.settings)?;
            return Err(error);
        }
        self.settings = next;
        self.snapshot()
    }
    pub fn preview_ring(&mut self, device: String) -> Result<EngineSnapshot, PhoneError> {
        if self.engine.is_none() {
            self.engine = Some(self.start_engine(self.settings.audio_mode, &self.settings.sound)?);
        }
        self.engine()?.preview_ringtone(&device)?;
        self.snapshot()
    }
    pub fn stop_preview(&mut self) -> Result<EngineSnapshot, PhoneError> {
        if let Some(engine) = self.engine.as_mut() {
            engine.stop_ringtone_preview()?;
        }
        self.snapshot()
    }

    pub fn select(&mut self, id: u32) -> Result<EngineSnapshot, PhoneError> {
        let key = self
            .keys
            .get(&id)
            .ok_or_else(|| PhoneError::new(ErrorCode::InvalidInput, "Unknown account."))?
            .clone();
        let previous = self.snapshot()?.selected_account_id;
        self.engine()?.select_account(id)?;
        let mut next = self.settings.clone();
        next.selected_key = Some(key);
        if let Err(error) = self.write(&next) {
            if let Some(previous) = previous {
                self.engine()?.select_account(previous)?;
            }
            return Err(error);
        }
        self.settings = next;
        self.snapshot()
    }
    pub fn remove(&mut self, id: u32) -> Result<EngineSnapshot, PhoneError> {
        if self.snapshot()?.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before removing accounts.",
            ));
        }
        let key = self
            .keys
            .get(&id)
            .ok_or_else(|| PhoneError::new(ErrorCode::InvalidInput, "Unknown account."))?
            .clone();
        let mut next = self.settings.clone();
        next.accounts.retain(|a| a.key != key);
        if next.selected_key.as_ref() == Some(&key) {
            next.selected_key = next.accounts.first().map(|a| a.key.clone());
        }
        self.engine()?.remove_account(id)?;
        self.engine()?.wait_account_removed(id)?;
        // Journal deletion with the metadata commit. Failed keychain access leaves
        // a retryable key, rather than silently retaining an unreachable password.
        next.pending_deletions.push(key.clone());
        if let Err(error) = self.write(&next) {
            let saved = self
                .settings
                .accounts
                .iter()
                .find(|a| a.key == key)
                .cloned()
                .ok_or_else(store::storage_error)?;
            let password = store::read_password(&key);
            let config = AccountConfig {
                registrar: saved.registrar,
                username: saved.username,
                password: password
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|_| "unavailable".into()),
                audio_mode: self.settings.audio_mode,
            };
            let restore = match password {
                Ok(_) => self
                    .engine()?
                    .save_account(None, &saved.name, &config)
                    .and_then(|id| {
                        self.engine()?.wait_account_update(id)?;
                        Ok(id)
                    }),
                Err(error) => Err(error),
            };
            let restored_id = match restore {
                Ok(id) => id,
                Err(error) => {
                    self.engine()?
                        .restore_unavailable_account(&saved.name, &config, error)?
                }
            };
            self.keys.remove(&id);
            self.keys.insert(restored_id, key);
            return Err(error);
        }
        self.settings = next;
        self.keys.remove(&id);
        self.cleanup_credentials();
        self.snapshot()
    }

    fn shutdown(&mut self) -> Result<(), PhoneError> {
        if let Some(mut engine) = self.engine.take() {
            engine.shutdown()?;
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(crate) struct PhoneState(Arc<Mutex<PhoneSession>>);
impl PhoneState {
    pub(crate) fn with<T>(
        &self,
        operation: impl FnOnce(&mut PhoneSession) -> Result<T, PhoneError>,
    ) -> Result<T, PhoneError> {
        let mut session = self.0.lock().map_err(|_| lock_error())?;
        operation(&mut session)
    }
    pub async fn execute(
        &self,
        operation: impl FnOnce(&mut PhoneSession) -> Result<EngineSnapshot, PhoneError> + Send + 'static,
    ) -> Result<EngineSnapshot, PhoneError> {
        let session = Arc::clone(&self.0);
        tauri::async_runtime::spawn_blocking(move || {
            let mut session = session.lock().map_err(|_| lock_error())?;
            operation(&mut session)
        })
        .await
        .map_err(|_| lock_error())?
    }
    pub fn shutdown(&self) -> Result<(), PhoneError> {
        self.0.lock().map_err(|_| lock_error())?.shutdown()
    }
}
fn lock_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The desktop engine state is unavailable.",
    )
}

#[cfg(all(test, feature = "native"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Uses native OS device metadata, without microphone capture or keychain writes"]
    fn first_sound_save_rejects_missing_device_without_starting_engine() {
        let root = std::env::temp_dir().join(format!("sip-sound-settings-{}", std::process::id()));
        let mut session = PhoneSession::default();
        session.load(root.clone(), root.clone()).unwrap();
        let invalid = SoundSettings {
            input_device: "missing-test-device".into(),
            ..Default::default()
        };
        assert!(session.save_sound(invalid).is_err());
        assert_eq!(
            session.snapshot().unwrap().sound_settings,
            SoundSettings::default()
        );
        assert!(session.engine.is_none());
        assert!(!root.join("accounts.json").exists());
    }

    #[test]
    #[ignore = "Requires Asterisk lab and writes unique synthetic OS keychain entries"]
    fn saved_accounts_survive_session_restart_without_password_readback() {
        let root = std::env::temp_dir().join(format!("sip-session-{}", std::process::id()));
        let mut session = PhoneSession::default();
        session.load(root.clone(), root.clone()).unwrap();
        session
            .save_sound(SoundSettings {
                ringtone_enabled: false,
                ..Default::default()
            })
            .unwrap();
        for username in ["1001", "1002"] {
            session
                .save(
                    None,
                    username.into(),
                    AccountConfig {
                        registrar: "127.0.0.1:5070".into(),
                        username: username.into(),
                        password: "sip-lab-only-not-a-secret".into(),
                        audio_mode: AudioMode::TestTone,
                    },
                )
                .unwrap();
        }
        // Replacing a saved password keeps the same account identity and deletes
        // the obsolete key only after durable commit and native acceptance.
        let first = session.snapshot().unwrap().accounts[0].id;
        let old_key = session.keys[&first].clone();
        session
            .save(
                Some(first),
                "Renamed".into(),
                AccountConfig {
                    registrar: "127.0.0.1:5070".into(),
                    username: "1001".into(),
                    password: "".into(),
                    audio_mode: AudioMode::TestTone,
                },
            )
            .unwrap();
        assert!(store::read_password(&old_key).is_err());
        assert_eq!(session.snapshot().unwrap().accounts[0].name, "Renamed");
        let second = session.snapshot().unwrap().accounts[1].id;
        session.select(second).unwrap();
        // Simulate interruption after a new secret was journaled and written.
        let interrupted_key = format!("abcd-{:x}", std::process::id());
        session
            .settings
            .pending_deletions
            .push(interrupted_key.clone());
        session.write(&session.settings).unwrap();
        store::write_password(&interrupted_key, "synthetic-orphan").unwrap();
        let metadata = std::fs::read_to_string(root.join("accounts.json")).unwrap();
        assert!(!metadata.contains("sip-lab-only-not-a-secret"));
        assert!(!metadata.contains("password"));
        session.shutdown().unwrap();
        drop(session);
        let mut restored = PhoneSession::default();
        restored.load(root.clone(), root.clone()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while !restored
            .snapshot()
            .unwrap()
            .accounts
            .iter()
            .all(|a| a.registration == phone_core::RegistrationState::Registered)
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(store::read_password(&interrupted_key).is_err());
        assert!(restored.settings.pending_deletions.is_empty());
        let state = restored.snapshot().unwrap();
        assert_eq!(state.accounts.len(), 2);
        assert!(!state.sound_settings.ringtone_enabled);
        assert_eq!(
            state
                .accounts
                .iter()
                .find(|a| Some(a.id) == state.selected_account_id)
                .unwrap()
                .username,
            "1002"
        );
        for account in state.accounts {
            restored.remove(account.id).unwrap();
        }
        assert!(
            restored
                .store
                .as_ref()
                .unwrap()
                .load()
                .unwrap()
                .accounts
                .is_empty()
        );
        restored.shutdown().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
