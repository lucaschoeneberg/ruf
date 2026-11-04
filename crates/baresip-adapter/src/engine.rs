use phone_core::{
    AccountConfig, AccountSnapshot, AudioMode, CallState, EngineSnapshot, ErrorCode, PhoneError,
    RegistrationState, SoundSettings,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct EngineState {
    pub(crate) snapshot: EngineSnapshot,
    pub(crate) call_generation: u64,
    next_account_id: u32,
    pub(crate) pending_accounts: BTreeMap<u32, Option<AccountSnapshot>>,
    pub(crate) account_results: BTreeMap<u32, Result<(), PhoneError>>,
    pub(crate) stopped: bool,
    pub(crate) sound_result: Option<Result<(), PhoneError>>,
}

/// One owned native engine, up to eight simultaneous SIP accounts and one call slot.
pub struct Engine {
    snapshot: Arc<Mutex<EngineState>>,
    audio: AudioMode,
    #[cfg(feature = "native")]
    native: crate::ffi::NativeEngine,
}

impl Engine {
    /// Start the native event loop; a test-tone player requires a WAV output path.
    pub fn start(audio: AudioMode, recording: Option<PathBuf>) -> Result<Self, PhoneError> {
        #[cfg(feature = "native")]
        {
            let snapshot = Arc::new(Mutex::new(EngineState {
                snapshot: EngineSnapshot {
                    audio_mode: Some(audio),
                    ..Default::default()
                },
                ..Default::default()
            }));
            let native = crate::ffi::NativeEngine::start(Arc::clone(&snapshot), audio, recording)?;
            Ok(Self {
                snapshot,
                audio,
                native,
            })
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = (audio, recording);
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "Rebuild with the native feature and a matching native SDK.",
            ))
        }
    }

    /// Read owned, password-free state; no native pointers escape this method.
    pub fn snapshot(&self) -> Result<EngineSnapshot, PhoneError> {
        self.snapshot
            .lock()
            .map(|s| s.snapshot.clone())
            .map_err(|_| state_error())
    }

    /// Validate and apply device routing for subsequent calls while idle.
    pub fn set_sound_settings(&mut self, settings: &SoundSettings) -> Result<(), PhoneError> {
        self.apply_sound(settings, false)
    }
    /// Restore saved routing even if a device is temporarily unplugged; never silently replace it.
    pub fn restore_sound_settings(&mut self, settings: &SoundSettings) -> Result<(), PhoneError> {
        self.apply_sound(settings, true)
    }
    fn apply_sound(&mut self, settings: &SoundSettings, restore: bool) -> Result<(), PhoneError> {
        settings.validate()?;
        self.reserve_sound_operation()?;
        #[cfg(feature = "native")]
        self.native
            .sound_request(if restore { 10 } else { 7 }, settings)?;
        #[cfg(not(feature = "native"))]
        {
            let _ = restore;
            Err(state_error())
        }
        #[cfg(feature = "native")]
        {
            self.wait_sound_operation()?;
            self.snapshot
                .lock()
                .map_err(|_| state_error())?
                .snapshot
                .sound_settings = settings.clone();
            Ok(())
        }
    }
    /// Play the built-in two-second sample on the chosen output; does not save routing.
    pub fn preview_ringtone(&mut self, device: &str) -> Result<(), PhoneError> {
        SoundSettings {
            ring_device: device.into(),
            ..Default::default()
        }
        .validate()?;
        self.reserve_sound_operation()?;
        #[cfg(feature = "native")]
        {
            self.native.preview_request(8, device)?;
            self.wait_sound_operation()
        }
        #[cfg(not(feature = "native"))]
        {
            Err(state_error())
        }
    }
    /// Stop a preview without stopping an incoming-call ringtone.
    pub fn stop_ringtone_preview(&mut self) -> Result<(), PhoneError> {
        self.snapshot
            .lock()
            .map_err(|_| state_error())?
            .sound_result = None;
        #[cfg(feature = "native")]
        {
            self.native.preview_request(9, "")?;
            self.wait_sound_operation()
        }
        #[cfg(not(feature = "native"))]
        {
            Err(state_error())
        }
    }
    fn reserve_sound_operation(&self) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the call before changing or previewing sound.",
            ));
        }
        state.sound_result = None;
        Ok(())
    }
    #[cfg(feature = "native")]
    fn wait_sound_operation(&self) -> Result<(), PhoneError> {
        loop {
            {
                let mut state = self.snapshot.lock().map_err(|_| state_error())?;
                if let Some(result) = state.sound_result.take() {
                    return result;
                }
                if state.stopped {
                    return Err(state_error());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Compatibility API: replace the selected account, or add the first account.
    pub fn configure_account(&mut self, config: &AccountConfig) -> Result<(), PhoneError> {
        let id = self.snapshot()?.selected_account_id;
        self.save_account(id, &config.username, config).map(|_| ())
    }

    /// Queue account creation/replacement; returns its session identity, not registration success.
    pub fn save_account(
        &mut self,
        id: Option<u32>,
        name: &str,
        config: &AccountConfig,
    ) -> Result<u32, PhoneError> {
        config.validate()?;
        if config.audio_mode != self.audio {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Restart the engine to change the audio backend.",
            ));
        }
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Account name must contain 1–64 characters without control characters.",
            ));
        }
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before changing accounts.",
            ));
        }
        if let Some(id) = id {
            if !state
                .snapshot
                .accounts
                .iter()
                .any(|a| a.id == id && !a.removing)
            {
                return Err(PhoneError::new(
                    ErrorCode::InvalidInput,
                    "Unknown or removing account.",
                ));
            }
        } else if state.snapshot.accounts.len() >= 8 {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "At most eight accounts are supported.",
            ));
        }
        if state.snapshot.accounts.iter().any(|a| {
            Some(a.id) != id
                && a.registrar.eq_ignore_ascii_case(&config.registrar)
                && a.username == config.username
        }) {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "This SIP identity already exists.",
            ));
        }
        let id = match id {
            Some(id) => id,
            None => {
                state.next_account_id = state
                    .next_account_id
                    .checked_add(1)
                    .ok_or_else(state_error)?;
                state.next_account_id
            }
        };
        if state.pending_accounts.contains_key(&id) {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "An account update is still pending.",
            ));
        }
        // Hold the state lock until the entry exists: queued callbacks cannot race insertion.
        self.request(1, id, Some(config), None, 0)?;
        let previous = state.snapshot.accounts.iter().find(|a| a.id == id).cloned();
        state.pending_accounts.insert(id, previous);
        state.account_results.remove(&id);
        let account = AccountSnapshot {
            id,
            name: name.into(),
            registrar: config.registrar.clone(),
            username: config.username.clone(),
            registration: RegistrationState::Registering,
            last_error: None,
            removing: false,
        };
        if let Some(existing) = state.snapshot.accounts.iter_mut().find(|a| a.id == id) {
            *existing = account;
        } else {
            state.snapshot.accounts.push(account);
        }
        if state.snapshot.selected_account_id.is_none() {
            state.snapshot.selected_account_id = Some(id);
        }
        state.snapshot.last_error = None;
        state.snapshot.sync_selected_registration();
        Ok(id)
    }

    /// Wait for the native queue to apply or reject a configuration, not for SIP registration.
    /// Native shutdown rejects pending operations; a delayed operation is never abandoned.
    pub fn wait_account_update(&self, id: u32) -> Result<(), PhoneError> {
        loop {
            {
                let mut state = self.snapshot.lock().map_err(|_| state_error())?;
                if let Some(result) = state.account_results.remove(&id) {
                    return result;
                }
                if state.stopped {
                    return Err(state_error());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Wait for native deletion or rejection, so durable storage cannot race a live UA.
    pub fn wait_account_removed(&self, id: u32) -> Result<(), PhoneError> {
        loop {
            {
                let state = self.snapshot.lock().map_err(|_| state_error())?;
                match state.snapshot.accounts.iter().find(|a| a.id == id) {
                    None => return Ok(()),
                    Some(account) if !account.removing => {
                        return Err(PhoneError::new(
                            ErrorCode::Busy,
                            "An incoming call prevented account removal.",
                        ));
                    }
                    _ => {}
                }
                if state.stopped {
                    return Err(state_error());
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Restore non-secret metadata when a saved password could not be read.
    /// The account remains visibly failed and can be repaired by a normal save.
    pub fn restore_unavailable_account(
        &mut self,
        name: &str,
        config: &AccountConfig,
        error: PhoneError,
    ) -> Result<u32, PhoneError> {
        config.validate()?;
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.accounts.len() >= 8
            || name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Invalid saved account metadata.",
            ));
        }
        state.next_account_id = state
            .next_account_id
            .checked_add(1)
            .ok_or_else(state_error)?;
        let id = state.next_account_id;
        state.snapshot.accounts.push(AccountSnapshot {
            id,
            name: name.into(),
            registrar: config.registrar.clone(),
            username: config.username.clone(),
            registration: RegistrationState::Failed,
            last_error: Some(error),
            removing: false,
        });
        if state.snapshot.selected_account_id.is_none() {
            state.snapshot.selected_account_id = Some(id);
        }
        state.snapshot.sync_selected_registration();
        Ok(id)
    }

    /// Change the outgoing account without disconnecting the other registered accounts.
    pub fn select_account(&mut self, id: u32) -> Result<(), PhoneError> {
        self.snapshot
            .lock()
            .map_err(|_| state_error())?
            .snapshot
            .select_account(id)
    }

    /// Queue removal while idle; the native confirmation removes its public metadata.
    pub fn remove_account(&mut self, id: u32) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before removing accounts.",
            ));
        }
        let account = state
            .snapshot
            .accounts
            .iter_mut()
            .find(|a| a.id == id && !a.removing)
            .ok_or_else(|| {
                PhoneError::new(ErrorCode::InvalidInput, "Unknown or removing account.")
            })?;
        self.request(6, id, None, None, 0)?;
        account.removing = true;
        state.snapshot.sync_selected_registration();
        Ok(())
    }

    /// Reserve the single call slot and queue a numeric extension/number.
    pub fn dial(&mut self, destination: &str) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let previous = state.snapshot.clone();
        state.snapshot.reserve_dial(destination)?;
        if let Err(error) = self.request(
            2,
            state.snapshot.selected_account_id.unwrap_or(0),
            None,
            Some(destination),
            0,
        ) {
            state.snapshot = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Accept the currently offered incoming call and reserve its answer atomically.
    pub fn answer(&mut self) -> Result<(), PhoneError> {
        self.answer_current(None)
    }

    /// Answer only the call matching the snapshot's opaque token.
    /// Returns InvalidState for stale tokens or calls already being answered.
    pub fn answer_call(&mut self, call_id: &str) -> Result<(), PhoneError> {
        self.answer_current(Some(call_id))
    }

    fn answer_current(&self, call_id: Option<&str>) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        validate_call_token(&state.snapshot, call_id)?;
        if state.snapshot.call != CallState::Incoming {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "No incoming call to answer.",
            ));
        }
        state.snapshot.call = CallState::Answering;
        if let Err(error) = self.request(3, 0, None, None, state.call_generation) {
            state.snapshot.call = CallState::Incoming;
            return Err(error);
        }
        Ok(())
    }

    /// Reject or end the current call; completion arrives as a native event.
    pub fn hangup(&mut self) -> Result<(), PhoneError> {
        self.hangup_current(None)
    }

    /// End only the call matching the snapshot's opaque token.
    /// Returns InvalidState if the token is stale or the slot is idle.
    pub fn hangup_call(&mut self, call_id: &str) -> Result<(), PhoneError> {
        self.hangup_current(Some(call_id))
    }

    fn hangup_current(&self, call_id: Option<&str>) -> Result<(), PhoneError> {
        let state = self.snapshot.lock().map_err(|_| state_error())?;
        validate_call_token(&state.snapshot, call_id)?;
        if state.snapshot.call == CallState::Idle {
            return Err(PhoneError::new(ErrorCode::InvalidState, "No call to end."));
        }
        self.request(4, 0, None, None, state.call_generation)
    }

    /// Stop the native event loop and join its thread before releasing resources.
    pub fn shutdown(&mut self) -> Result<(), PhoneError> {
        #[cfg(feature = "native")]
        {
            self.native.shutdown()
        }
        #[cfg(not(feature = "native"))]
        {
            Ok(())
        }
    }

    fn request(
        &self,
        operation: i32,
        account_id: u32,
        account: Option<&AccountConfig>,
        destination: Option<&str>,
        call_generation: u64,
    ) -> Result<(), PhoneError> {
        #[cfg(feature = "native")]
        {
            self.native
                .request(operation, account_id, account, destination, call_generation)
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = (operation, account_id, account, destination, call_generation);
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "Native support is not linked.",
            ))
        }
    }
}

fn validate_call_token(snapshot: &EngineSnapshot, call_id: Option<&str>) -> Result<(), PhoneError> {
    if call_id.is_some_and(|token| snapshot.call_id.as_deref() != Some(token)) {
        return Err(PhoneError::new(
            ErrorCode::InvalidState,
            "The call changed before this action was submitted.",
        ));
    }
    Ok(())
}

fn state_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The engine state lock is unavailable.",
    )
}

#[cfg(all(test, feature = "native"))]
mod interop_tests {
    use super::*;
    use phone_core::RegistrationState;
    use std::{
        thread,
        time::{Duration, Instant},
    };

    fn wait(engine: &Engine, predicate: impl Fn(&EngineSnapshot) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let state = engine.snapshot().unwrap();
            if predicate(&state) {
                return;
            }
            assert!(Instant::now() < deadline, "Timed out: {state:?}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn nat_remote_hangup_without_server_contact_rewriting() {
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-nat-hangup.wav")),
        )
        .unwrap();
        let account = AccountConfig {
            registrar: "127.0.0.1:5070".into(),
            username: "1003".into(),
            password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
            audio_mode: AudioMode::TestTone,
        };
        engine.configure_account(&account).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        engine.dial("601").unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        assert_eq!(engine.snapshot().unwrap().call_account_id, None);
        let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/interop/asterisk/compose.yml");
        assert!(
            std::process::Command::new("docker")
                .args(["compose", "-f"])
                .arg(lab)
                .args([
                    "exec",
                    "-T",
                    "asterisk",
                    "asterisk",
                    "-rx",
                    "channel originate PJSIP/1003 application Wait 1"
                ])
                .status()
                .unwrap()
                .success()
        );
        wait(&engine, |s| s.call == CallState::Incoming);
        engine.answer().unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        engine.dial("601").unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        let state = engine.snapshot().unwrap();
        assert_eq!(state.call_account_id, None);
        assert!(state.last_error.is_none(), "{state:?}");
        engine.shutdown().unwrap();
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn remote_hangup_releases_outgoing_and_incoming_calls() {
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-remote-hangup.wav")),
        )
        .unwrap();
        let account = AccountConfig {
            registrar: "127.0.0.1:5070".into(),
            username: "1001".into(),
            password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
            audio_mode: AudioMode::TestTone,
        };
        engine.configure_account(&account).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        // Repeating proves the native call slot is released, not just the UI state.
        for _ in 0..2 {
            engine.dial("601").unwrap();
            wait(&engine, |s| s.call == CallState::Active);
            wait(&engine, |s| s.call == CallState::Idle);
            let state = engine.snapshot().unwrap();
            assert_eq!(state.call_account_id, None);
            assert!(!state.ringtone_playing);
            assert!(state.last_error.is_none(), "{state:?}");
        }
        let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/interop/asterisk/compose.yml");
        assert!(
            std::process::Command::new("docker")
                .args(["compose", "-f"])
                .arg(lab)
                .args([
                    "exec",
                    "-T",
                    "asterisk",
                    "asterisk",
                    "-rx",
                    "channel originate PJSIP/1001 application Wait 1",
                ])
                .status()
                .unwrap()
                .success()
        );
        wait(&engine, |s| s.call == CallState::Incoming);
        engine.answer().unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        let state = engine.snapshot().unwrap();
        assert_eq!(state.call_account_id, None);
        assert!(!state.ringtone_playing);
        assert!(state.last_error.is_none(), "{state:?}");
        // The incoming call must also release the native call slot.
        engine.dial("601").unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        assert_eq!(engine.snapshot().unwrap().call_account_id, None);
        // No local hangup has been issued anywhere in this test.
        engine.shutdown().unwrap();
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn multiple_accounts_register_switch_call_and_remove_independently() {
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-multiple-accounts.wav")),
        )
        .unwrap();
        for username in ["1001", "1002"] {
            let account = AccountConfig {
                registrar: "127.0.0.1:5070".into(),
                username: username.into(),
                password: std::env::var("SIP_TEST_PASSWORD").unwrap(),
                audio_mode: AudioMode::TestTone,
            };
            engine.save_account(None, username, &account).unwrap();
        }
        wait(&engine, |s| {
            s.accounts.len() == 2
                && s.accounts
                    .iter()
                    .all(|a| a.registration == RegistrationState::Registered)
        });
        let ids: Vec<_> = engine
            .snapshot()
            .unwrap()
            .accounts
            .iter()
            .map(|a| a.id)
            .collect();
        for id in &ids {
            engine.select_account(*id).unwrap();
            engine.dial("600").unwrap();
            wait(&engine, |s| s.call == CallState::Active);
            assert_eq!(engine.snapshot().unwrap().call_account_id, Some(*id));
            assert_eq!(
                engine
                    .set_sound_settings(&SoundSettings::default())
                    .unwrap_err()
                    .code,
                ErrorCode::Busy
            );
            assert_eq!(
                engine.preview_ringtone("").unwrap_err().code,
                ErrorCode::Busy
            );
            // Model an idle Rust reservation racing a native incoming call.
            // The native call slot must reject it and restore the old identity.
            if *id == ids[0] {
                let before = engine.snapshot().unwrap().accounts[0].clone();
                engine.snapshot.lock().unwrap().snapshot.call = CallState::Idle;
                let replacement = AccountConfig {
                    registrar: "127.0.0.1:5070".into(),
                    username: "9999".into(),
                    password: "not-used".into(),
                    audio_mode: AudioMode::TestTone,
                };
                engine
                    .save_account(Some(*id), "Must not replace", &replacement)
                    .unwrap();
                assert!(engine.wait_account_update(*id).is_err());
                let after = engine.snapshot().unwrap().accounts[0].clone();
                assert_eq!(
                    (after.id, after.name, after.username),
                    (before.id, before.name, before.username)
                );
                engine.snapshot.lock().unwrap().snapshot.call = CallState::Active;
            }

            assert_eq!(
                engine.remove_account(*id).unwrap_err().code,
                ErrorCode::Busy
            );
            assert_eq!(
                engine.select_account(ids[0]).unwrap_err().code,
                ErrorCode::Busy
            );
            engine.hangup().unwrap();
            wait(&engine, |s| s.call == CallState::Idle);
        }
        // An inbound call on account B must not change outgoing selection A.
        engine.select_account(ids[0]).unwrap();
        let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/interop/asterisk/compose.yml");
        assert!(
            std::process::Command::new("docker")
                .args(["compose", "-f"])
                .arg(lab)
                .args([
                    "exec",
                    "-T",
                    "asterisk",
                    "asterisk",
                    "-rx",
                    "channel originate PJSIP/1002 application Wait 5"
                ])
                .status()
                .unwrap()
                .success()
        );
        wait(&engine, |s| s.call == CallState::Incoming);
        assert_eq!(engine.snapshot().unwrap().call_account_id, Some(ids[1]));
        assert_eq!(engine.snapshot().unwrap().selected_account_id, Some(ids[0]));
        engine.answer().unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        engine.hangup().unwrap();
        wait(&engine, |s| s.call == CallState::Idle);
        engine.remove_account(ids[1]).unwrap();
        wait(&engine, |s| s.accounts.len() == 1);
        let state = engine.snapshot().unwrap();
        assert_eq!(state.selected_account_id, Some(ids[0]));
        assert_eq!(
            state.accounts[0].registration,
            RegistrationState::Registered
        );
        assert_eq!(state.audio_mode, Some(AudioMode::TestTone));
        engine.shutdown().unwrap();
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn stale_call_actions_preserve_a_replacement_call() {
        let account = AccountConfig {
            registrar: "127.0.0.1:5070".into(),
            username: "1000".into(),
            password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
            audio_mode: AudioMode::TestTone,
        };
        let path = std::env::temp_dir().join("sip-call-generation.wav");
        let mut engine = Engine::start(AudioMode::TestTone, Some(path)).unwrap();
        engine.configure_account(&account).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        engine.dial("600").unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        // Capture the identity accepted for call A; its delayed action is replayed
        // only after native processing has established replacement call B.
        let generation_a = engine.snapshot.lock().unwrap().call_generation;
        engine.hangup().unwrap();
        wait(&engine, |s| s.call == CallState::Idle);
        engine.dial("600").unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        assert_ne!(
            generation_a,
            engine.snapshot.lock().unwrap().call_generation
        );
        for operation in [4, 3] {
            engine.snapshot.lock().unwrap().snapshot.last_error = None;
            engine
                .native
                .request(operation, 0, None, None, generation_a)
                .unwrap();
            wait(&engine, |s| {
                s.call == CallState::Idle || s.last_error.is_some()
            });
            let state = engine.snapshot().unwrap();
            assert_eq!(
                state.call,
                CallState::Active,
                "A stale action must not change call B"
            );
            assert_eq!(state.last_error.unwrap().code, ErrorCode::InvalidState);
        }
        engine.hangup().unwrap();
        wait(&engine, |s| s.call == CallState::Idle);
        engine.shutdown().unwrap();
    }
}

#[cfg(all(test, not(feature = "native")))]
mod action_tests {
    use super::*;
    fn incoming() -> Engine {
        Engine {
            snapshot: Arc::new(Mutex::new(EngineState {
                snapshot: EngineSnapshot {
                    call: CallState::Incoming,
                    call_id: Some("current".into()),
                    ..Default::default()
                },
                call_generation: 9,
                ..Default::default()
            })),
            audio: AudioMode::TestTone,
        }
    }
    #[test]
    fn stale_frontend_tokens_are_rejected_before_native_queue() {
        let mut engine = incoming();
        assert_eq!(
            engine.answer_call("old").unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(
            engine.hangup_call("old").unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(engine.snapshot().unwrap().call, CallState::Incoming);
    }
    #[test]
    fn rejected_answer_queue_restores_incoming_reservation() {
        let mut engine = incoming();
        assert_eq!(
            engine.answer_call("current").unwrap_err().code,
            ErrorCode::EngineUnavailable
        );
        let snapshot = engine.snapshot().unwrap();
        assert_eq!(snapshot.call, CallState::Incoming);
        assert_eq!(snapshot.call_id.as_deref(), Some("current"));
    }
    #[test]
    fn answering_rejects_a_second_answer() {
        let mut engine = incoming();
        engine.snapshot.lock().unwrap().snapshot.call = CallState::Answering;
        assert_eq!(
            engine.answer_call("current").unwrap_err().code,
            ErrorCode::InvalidState
        );
    }
}
