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

pub(crate) struct PhoneSession {
    engine: Option<Engine>,
    store: Option<AccountStore>,
    settings: SavedSettings,
    // Session identities map to stable, opaque credential keys in saved metadata.
    keys: BTreeMap<u32, String>,
    recording: Option<PathBuf>,
    storage_warning: Option<PhoneError>,
    credentials: Arc<dyn store::CredentialStore>,
    // Journaled secrets owned by saves currently waiting outside the session lock.
    credential_save: Option<String>,
    stopped: bool,
    call_observer: Option<baresip_adapter::CallObserver>,
}

impl Default for PhoneSession {
    fn default() -> Self {
        Self {
            engine: None,
            store: None,
            settings: SavedSettings::default(),
            keys: BTreeMap::new(),
            recording: None,
            storage_warning: None,
            credentials: Arc::new(store::SystemCredentials),
            credential_save: None,
            stopped: false,
            call_observer: None,
        }
    }
}

impl PhoneSession {
    /// Attach the nonblocking history sink before restoring accounts or registering.
    pub(crate) fn set_call_observer(&mut self, observer: Option<baresip_adapter::CallObserver>) {
        self.call_observer = observer.clone();
        if let Some(engine) = self.engine.as_mut() {
            engine.set_call_observer(observer);
        }
    }

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
        if self.stopped {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "The phone is shutting down.",
            ));
        }
        if self.store.is_some() {
            return Ok(());
        }
        // Each restoration read suppresses OS authorization UI independently.
        let store = AccountStore {
            path: directory.join("accounts.json"),
        };
        let settings = store.load()?;
        self.recording = Some(cache.join("received.wav"));
        if !settings.accounts.is_empty() {
            let mut engine = self.start_engine(settings.audio_mode, &settings.sound)?;
            let mut keys = BTreeMap::new();
            for saved in &settings.accounts {
                let password = self.credentials.read(&saved.key, false);
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
        engine.set_call_observer(self.call_observer.clone());
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
        // A failed metadata rollback can leave a newer on-disk account than the
        // current engine. Never delete a credential still referenced durably.
        let durable = match self
            .store
            .as_ref()
            .ok_or_else(store::storage_error)
            .and_then(AccountStore::load)
        {
            Ok(settings) => settings,
            Err(_) => {
                self.storage_warning = Some(store::storage_error());
                return;
            }
        };
        let mut next = self.settings.clone();
        next.pending_deletions.retain(|key| {
            self.credential_save.as_ref() == Some(key)
                || durable.accounts.iter().any(|account| account.key == *key)
                || self.credentials.delete(key).is_err()
        });
        let pending = next
            .pending_deletions
            .iter()
            .any(|key| self.credential_save.as_ref() != Some(key));
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

    fn validate_save(
        &self,
        id: Option<u32>,
        name: &str,
        account: &AccountConfig,
    ) -> Result<(), PhoneError> {
        if self.stopped {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "The phone is shutting down.",
            ));
        }
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
        Ok(())
    }

    fn prepare_save(
        &mut self,
        id: Option<u32>,
        name: String,
        account: AccountConfig,
    ) -> Result<PreparedSave, PhoneError> {
        let old_key = id
            .map(|id| {
                self.keys
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| PhoneError::new(ErrorCode::InvalidInput, "Unknown account."))
            })
            .transpose()?;
        if account.password.is_empty() && old_key.is_none() {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "New accounts require a password.",
            ));
        }
        // Validate non-secret fields before allowing a keychain prompt. The real
        // secret is validated again after a retained password has been read.
        let mut validation = account.clone();
        if validation.password.is_empty() {
            validation.password = "unavailable".into();
        }
        validation.validate()?;
        let name = name.trim().to_string();
        self.validate_save(id, &name, &account)?;
        let key = format!(
            "{:x}-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| store::storage_error())?
                .as_nanos(),
            std::process::id()
        );
        // Keep crash cleanup durable before the out-of-lock credential write.
        let mut journal = self.settings.clone();
        journal.pending_deletions.push(key.clone());
        self.write(&journal)?;
        self.settings = journal;
        self.credential_save = Some(key.clone());
        Ok(PreparedSave {
            id,
            name,
            account,
            old_key,
            key,
            credentials: Arc::clone(&self.credentials),
        })
    }

    fn commit_save(&mut self, saved: &PreparedSave) -> Result<EngineSnapshot, PhoneError> {
        let PreparedSave {
            id,
            name,
            account,
            old_key,
            key,
            ..
        } = saved;
        if id.is_some_and(|id| self.keys.get(&id) != old_key.as_ref()) {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "The account changed during credential authorization.",
            ));
        }
        account.validate()?;
        self.validate_save(*id, name, account)?;
        if self.engine.is_none() {
            self.engine = Some(self.start_engine(account.audio_mode, &self.settings.sound)?);
        }
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
        next.pending_deletions.retain(|pending| pending != key);
        if let Some(old_key) = old_key {
            next.pending_deletions.push(old_key.clone());
        }
        next.audio_mode = account.audio_mode;
        if next.selected_key.is_none() || next.selected_key.as_ref() == old_key.as_ref() {
            next.selected_key = Some(key.clone());
        }
        self.write(&next)?;
        let saved_id = match self
            .engine()?
            .save_account(*id, name, account)
            .and_then(|id| {
                self.engine()?.wait_account_update(id)?;
                Ok(id)
            }) {
            Ok(id) => id,
            Err(error) => {
                self.write(&self.settings)?;
                return Err(error);
            }
        };
        self.settings = next;
        self.keys.insert(saved_id, key.clone());
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
            let password = self.credentials.read(&key, false);
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
        self.stopped = true;
        if let Some(mut engine) = self.engine.take() {
            engine.shutdown()?;
        }
        Ok(())
    }
}

enum SaveTarget {
    Id(Option<u32>),
    Selected,
}

struct PreparedSave {
    id: Option<u32>,
    name: String,
    account: AccountConfig,
    old_key: Option<String>,
    key: String,
    credentials: Arc<dyn store::CredentialStore>,
}
impl PreparedSave {
    fn persist_credentials(&mut self) -> Result<(), PhoneError> {
        if self.account.password.is_empty() {
            self.account.password = self.credentials.read(
                self.old_key.as_deref().ok_or_else(store::storage_error)?,
                true,
            )?;
        }
        self.account.validate()?;
        self.credentials.write(&self.key, &self.account.password)
    }
}

#[derive(Clone, Default)]
pub(crate) struct PhoneState(Arc<Mutex<PhoneSession>>, Arc<Mutex<()>>);
impl PhoneState {
    /// Save an account without holding phone state while OS credentials may ask for authorization.
    pub async fn save_account(
        &self,
        directory: PathBuf,
        cache: PathBuf,
        id: Option<u32>,
        name: String,
        account: AccountConfig,
    ) -> Result<EngineSnapshot, PhoneError> {
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            state.save_account_blocking(directory, cache, SaveTarget::Id(id), name, account)
        })
        .await
        .map_err(|_| lock_error())?
    }

    /// Compatibility save: resolve the outgoing account during locked preparation.
    pub async fn configure_account(
        &self,
        directory: PathBuf,
        cache: PathBuf,
        account: AccountConfig,
    ) -> Result<EngineSnapshot, PhoneError> {
        let state = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            state.save_account_blocking(
                directory,
                cache,
                SaveTarget::Selected,
                account.username.clone(),
                account,
            )
        })
        .await
        .map_err(|_| lock_error())?
    }

    fn save_account_blocking(
        &self,
        directory: PathBuf,
        cache: PathBuf,
        target: SaveTarget,
        name: String,
        account: AccountConfig,
    ) -> Result<EngineSnapshot, PhoneError> {
        // Serialize foreground saves without blocking observation or call commands.
        let _save = self.1.lock().map_err(|_| lock_error())?;
        let mut prepared = self.with(|session| {
            session.load(directory, cache)?;
            let id = match target {
                SaveTarget::Id(id) => id,
                SaveTarget::Selected => session.snapshot()?.selected_account_id,
            };
            session.prepare_save(id, name, account)
        })?;
        let credential_result = prepared.persist_credentials();
        self.with(|session| {
            let result = credential_result.and_then(|()| session.commit_save(&prepared));
            session.credential_save = None;
            session.cleanup_credentials();
            result.and_then(|_| session.snapshot())
        })
    }
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
    fn save(
        session: &mut PhoneSession,
        id: Option<u32>,
        name: String,
        account: AccountConfig,
    ) -> Result<EngineSnapshot, PhoneError> {
        let mut prepared = session.prepare_save(id, name, account)?;
        let result = prepared
            .persist_credentials()
            .and_then(|()| session.commit_save(&prepared));
        session.credential_save = None;
        session.cleanup_credentials();
        result.and_then(|_| session.snapshot())
    }
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
            save(
                &mut session,
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
        save(
            &mut session,
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
        assert!(session.credentials.read(&old_key, false).is_err());
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
        session
            .credentials
            .write(&interrupted_key, "synthetic-orphan")
            .unwrap();
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
        assert!(restored.credentials.read(&interrupted_key, false).is_err());
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

#[cfg(test)]
mod credential_tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    struct BlockedCredentials {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
        fail_write: bool,
        fail_delete: bool,
        values: Mutex<BTreeMap<String, String>>,
    }
    impl store::CredentialStore for BlockedCredentials {
        fn read(&self, key: &str, _interactive: bool) -> Result<String, PhoneError> {
            self.values
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(store::storage_error)
        }
        fn write(&self, key: &str, password: &str) -> Result<(), PhoneError> {
            self.entered.send(()).unwrap();
            self.release.lock().unwrap().recv().unwrap();
            if self.fail_write {
                return Err(store::storage_error());
            }
            self.values
                .lock()
                .unwrap()
                .insert(key.into(), password.into());
            Ok(())
        }
        fn delete(&self, key: &str) -> Result<(), PhoneError> {
            if self.fail_delete {
                return Err(store::storage_error());
            }
            self.values.lock().unwrap().remove(key);
            Ok(())
        }
    }

    #[test]
    fn credential_authorization_does_not_block_phone_state() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let state = PhoneState::default();
        state
            .with(|session| {
                session.credentials = Arc::new(BlockedCredentials {
                    entered: entered_tx,
                    release: Mutex::new(release_rx),
                    fail_write: true,
                    fail_delete: false,
                    values: Mutex::new(BTreeMap::new()),
                });
                Ok(())
            })
            .unwrap();
        let root =
            std::env::temp_dir().join(format!("sip-blocked-credentials-{}", std::process::id()));
        let saving = state.clone();
        let directory = root.clone();
        let save = std::thread::spawn(move || {
            saving.save_account_blocking(
                directory.clone(),
                directory,
                SaveTarget::Id(None),
                "Synthetic account".into(),
                AccountConfig {
                    registrar: "pbx.test".into(),
                    username: "1000".into(),
                    password: "synthetic".into(),
                    audio_mode: AudioMode::System,
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let reading = state.clone();
        let (progress_tx, progress_rx) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let result = reading.with(|session| {
                let snapshot = session.snapshot()?;
                session.settings.desktop.notifications_enabled = false;
                session.cleanup_credentials();
                Ok((snapshot, session.settings.pending_deletions.len()))
            });
            progress_tx.send(result).unwrap();
        });
        let progress = progress_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        assert!(save.join().unwrap().is_err());
        reader.join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert!(
            progress.is_ok(),
            "phone state was locked by OS credential authorization"
        );
        let (snapshot, journaled) = progress.unwrap().unwrap();
        assert_eq!(snapshot.call, CallState::Idle);
        assert_eq!(
            journaled, 1,
            "cleanup deleted a credential transaction still in flight"
        );
    }

    #[test]
    fn cleanup_keeps_a_credential_referenced_by_failed_metadata_rollback() {
        let (entered, _) = mpsc::channel();
        let (_, release) = mpsc::channel();
        let key = "abcd-5678";
        let credentials = Arc::new(BlockedCredentials {
            entered,
            release: Mutex::new(release),
            fail_write: false,
            fail_delete: false,
            values: Mutex::new(BTreeMap::from([(key.into(), "synthetic".into())])),
        });
        let root =
            std::env::temp_dir().join(format!("sip-rollback-credentials-{}", std::process::id()));
        let store = AccountStore {
            path: root.join("accounts.json"),
        };
        store
            .write(&SavedSettings {
                accounts: vec![SavedAccount {
                    key: key.into(),
                    name: "Durable replacement".into(),
                    registrar: "pbx.test".into(),
                    username: "1000".into(),
                }],
                ..Default::default()
            })
            .unwrap();
        let mut session = PhoneSession {
            store: Some(store),
            credentials: credentials.clone(),
            settings: SavedSettings {
                pending_deletions: vec![key.into()],
                ..Default::default()
            },
            ..Default::default()
        };
        session.cleanup_credentials();
        assert_eq!(session.settings.pending_deletions, [key]);
        assert!(credentials.values.lock().unwrap().contains_key(key));
        assert_eq!(
            session.store.as_ref().unwrap().load().unwrap().accounts[0].key,
            key
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shutdown_prevents_queued_startup_restoration() {
        let state = PhoneState::default();
        state.shutdown().unwrap();
        let root =
            std::env::temp_dir().join(format!("sip-stopped-credentials-{}", std::process::id()));
        let result = state.with(|session| session.load(root.clone(), root.clone()));
        assert_eq!(result.unwrap_err().code, ErrorCode::InvalidState);
        assert!(!root.exists());
    }

    #[test]
    fn shutdown_during_authorization_rejects_commit_and_keeps_cleanup_durable() {
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let credentials = Arc::new(BlockedCredentials {
            entered: entered_tx,
            release: Mutex::new(release_rx),
            fail_write: false,
            fail_delete: true,
            values: Mutex::new(BTreeMap::new()),
        });
        let state = PhoneState::default();
        state
            .with(|session| {
                session.credentials = credentials.clone();
                Ok(())
            })
            .unwrap();
        let root =
            std::env::temp_dir().join(format!("sip-stop-credentials-{}", std::process::id()));
        let directory = root.clone();
        let saving = state.clone();
        let save = std::thread::spawn(move || {
            saving.save_account_blocking(
                directory.clone(),
                directory,
                SaveTarget::Id(None),
                "Synthetic account".into(),
                AccountConfig {
                    registrar: "pbx.test".into(),
                    username: "1000".into(),
                    password: "synthetic".into(),
                    audio_mode: AudioMode::System,
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        // Shutdown is a real phone operation and must remain possible while the
        // foreground credential service is waiting for authorization.
        let stopping = state.clone();
        let (stopped_tx, stopped_rx) = mpsc::channel();
        let stop = std::thread::spawn(move || stopped_tx.send(stopping.shutdown()).unwrap());
        let stopped = stopped_rx.recv_timeout(Duration::from_secs(2));
        release_tx.send(()).unwrap();
        stop.join().unwrap();
        assert!(
            stopped.is_ok(),
            "shutdown waited for OS credential authorization"
        );
        stopped.unwrap().unwrap();
        assert_eq!(
            save.join().unwrap().unwrap_err().code,
            ErrorCode::InvalidState
        );
        let metadata = AccountStore {
            path: root.join("accounts.json"),
        }
        .load()
        .unwrap();
        assert!(metadata.accounts.is_empty());
        assert_eq!(metadata.pending_deletions.len(), 1);
        assert!(
            credentials
                .values
                .lock()
                .unwrap()
                .contains_key(&metadata.pending_deletions[0])
        );
        assert!(state.with(|session| Ok(session.engine.is_none())).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(feature = "native")]
    #[test]
    #[ignore = "Requires the isolated Asterisk lab; uses only in-memory credentials and test-tone audio"]
    fn incoming_during_authorization_preserves_account_and_cleanup_journal() {
        fn wait(state: &PhoneState, predicate: impl Fn(&EngineSnapshot) -> bool) -> EngineSnapshot {
            let deadline = std::time::Instant::now() + Duration::from_secs(15);
            loop {
                let snapshot = state.with(|session| session.snapshot()).unwrap();
                if predicate(&snapshot) {
                    return snapshot;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "native state did not arrive"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let password = "sip-lab-only-not-a-secret";
        let old_key = "abcd-1234";
        let credentials = Arc::new(BlockedCredentials {
            entered: entered_tx,
            release: Mutex::new(release_rx),
            fail_write: false,
            fail_delete: true,
            values: Mutex::new(BTreeMap::from([(old_key.into(), password.into())])),
        });
        let root =
            std::env::temp_dir().join(format!("sip-incoming-credentials-{}", std::process::id()));
        let store = AccountStore {
            path: root.join("accounts.json"),
        };
        store
            .write(&SavedSettings {
                accounts: vec![SavedAccount {
                    key: old_key.into(),
                    name: "Original".into(),
                    registrar: "127.0.0.1:5070".into(),
                    username: "1001".into(),
                }],
                selected_key: Some(old_key.into()),
                audio_mode: AudioMode::TestTone,
                ..Default::default()
            })
            .unwrap();
        let state = PhoneState::default();
        state
            .with(|session| {
                session.credentials = credentials.clone();
                session.load(root.clone(), root.clone())
            })
            .unwrap();
        let id = wait(&state, |s| {
            s.registration == phone_core::RegistrationState::Registered
        })
        .selected_account_id
        .unwrap();
        let saving = state.clone();
        let directory = root.clone();
        let save = std::thread::spawn(move || {
            saving.save_account_blocking(
                directory.clone(),
                directory,
                SaveTarget::Id(Some(id)),
                "Replacement".into(),
                AccountConfig {
                    registrar: "127.0.0.1:5070".into(),
                    username: "1001".into(),
                    password: password.into(),
                    audio_mode: AudioMode::TestTone,
                },
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/interop/asterisk/compose.yml");
        let originated = std::process::Command::new("docker")
            .args(["compose", "-f"])
            .arg(lab)
            .args([
                "exec",
                "-T",
                "asterisk",
                "asterisk",
                "-rx",
                "channel originate PJSIP/1001 application Wait 5",
            ])
            .output()
            .unwrap();
        assert!(originated.status.success());
        let incoming = wait(&state, |s| s.call == CallState::Incoming);
        release_tx.send(()).unwrap();
        assert_eq!(save.join().unwrap().unwrap_err().code, ErrorCode::Busy);
        let metadata = store.load().unwrap();
        assert_eq!(metadata.accounts[0].key, old_key);
        assert_eq!(metadata.accounts[0].name, "Original");
        assert_eq!(metadata.pending_deletions.len(), 1);
        assert!(
            credentials
                .values
                .lock()
                .unwrap()
                .contains_key(&metadata.pending_deletions[0])
        );
        state
            .with(|session| {
                assert_eq!(session.keys[&id], old_key);
                assert_eq!(session.snapshot()?.accounts[0].name, "Original");
                session
                    .engine()?
                    .hangup_call(incoming.call_id.as_deref().unwrap())
            })
            .unwrap();
        wait(&state, |s| s.call == CallState::Idle);
        state.shutdown().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
