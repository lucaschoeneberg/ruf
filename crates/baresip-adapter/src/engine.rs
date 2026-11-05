use phone_core::{
    AccountConfig, AccountSnapshot, AudioMode, AudioTestKind, CallState, EngineSnapshot, ErrorCode,
    PhoneError, RegistrationState, SoundSettings,
};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

// Private bridge operations; values mirror native/bridge.c.
const OP_MUTE: i32 = 17;
const OP_HOLD: i32 = 18;
const OP_TRANSFER: i32 = 19;
const OP_CONSULT: i32 = 20;
const OP_SWITCH: i32 = 21;
const OP_CANCEL_CONSULT: i32 = 22;
const OP_COMPLETE_TRANSFER: i32 = 23;

#[derive(Default)]
pub(crate) struct EngineState {
    pub(crate) snapshot: EngineSnapshot,
    pub(crate) engine_error: Option<PhoneError>,
    #[cfg(feature = "native")]
    pub(crate) terminal_call_errors: BTreeMap<u64, PhoneError>,
    pub(crate) call_generation: u64,
    pub(crate) call_journal: crate::call_journal::CallJournal,
    next_account_id: u32,
    pub(crate) pending_accounts: BTreeMap<u32, Option<AccountSnapshot>>,
    pub(crate) account_results: BTreeMap<u32, Result<(), PhoneError>>,
    pub(crate) stopped: bool,
    pub(crate) sound_result: Option<Result<(), PhoneError>>,
    #[cfg(feature = "native")]
    pub(crate) remote_hold: bool,
    #[cfg(feature = "native")]
    pub(crate) local_ending: bool,
    #[cfg(feature = "native")]
    pub(crate) local_endings: std::collections::BTreeSet<u64>,
    #[cfg(feature = "native")]
    pub(crate) call_destinations: BTreeMap<u64, String>,
}

impl EngineState {
    pub(crate) fn snapshot_for_read(&self) -> EngineSnapshot {
        let mut snapshot = self.snapshot.clone();
        if snapshot.last_error.is_none() {
            snapshot.last_error = self.engine_error.clone();
        }
        snapshot.with_read_revision()
    }
}

/// One owned native engine, up to eight SIP accounts, one original call and one consultation.
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
            .map(|s| s.snapshot_for_read())
            .map_err(|_| state_error())
    }

    /// Attach a nonblocking observer for ordered native call lifecycle events.
    /// The sink runs synchronously on the native event thread while state is locked;
    /// only enqueue work, without engine access, disk writes or UI operations.
    /// Passing `None` detaches the sink; prior observations are not replayed.
    pub fn set_call_observer(&mut self, observer: Option<crate::CallObserver>) {
        if let Ok(mut state) = self.snapshot.lock() {
            state.call_journal.observer = observer;
        }
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
    /// Start an explicit, idle-only native audio diagnostic on an opaque device ID.
    /// Microphone capture requires prior authorization where the OS supports querying it,
    /// otherwise the explicitly requested capture determines access. Stops after ten seconds;
    /// speaker output stops after two seconds. Incoming/outgoing calls preempt both.
    /// Returns an owner token for `stop_audio_test`; Busy rejects concurrent audio.
    /// Invalid device/token input returns InvalidInput, unlinked builds EngineUnavailable,
    /// and unavailable hardware/authorization returns NativeFailure without prompting.
    pub fn start_audio_test(
        &mut self,
        kind: AudioTestKind,
        device: &str,
    ) -> Result<String, PhoneError> {
        SoundSettings {
            input_device: device.into(),
            ..Default::default()
        }
        .validate()?;
        self.reserve_sound_operation()?;
        #[cfg(feature = "native")]
        {
            if kind == AudioTestKind::Microphone
                && self.audio == AudioMode::System
                && !matches!(
                    crate::microphone_permission(false)?,
                    phone_core::MicrophonePermission::Authorized
                        | phone_core::MicrophonePermission::Unsupported
                )
            {
                return Err(PhoneError::new(
                    ErrorCode::NativeFailure,
                    "Authorize microphone access before starting the test.",
                ));
            }
            let id = self.native.reserve_call_generation()?;
            self.snapshot
                .lock()
                .map_err(|_| state_error())?
                .snapshot
                .audio_test_error = None;
            self.native.audio_test_request(
                match kind {
                    AudioTestKind::Microphone => 12,
                    AudioTestKind::Speaker => 13,
                },
                id,
                device,
            )?;
            self.wait_sound_operation()?;
            Ok(id.to_string())
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = kind;
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "Native audio diagnostics are not linked.",
            ))
        }
    }
    /// Idempotently stop only the named test; stale owner tokens never stop a later
    /// diagnostic or an incoming-call ringtone. Does not require an idle call slot.
    pub fn stop_audio_test(&mut self, id: &str) -> Result<(), PhoneError> {
        let token = id
            .parse::<u64>()
            .ok()
            .filter(|value| *value != 0 && value.to_string() == id)
            .ok_or_else(|| PhoneError::new(ErrorCode::InvalidInput, "Invalid audio test token."))?;
        #[cfg(feature = "native")]
        {
            self.snapshot
                .lock()
                .map_err(|_| state_error())?
                .sound_result = None;
            self.native.audio_test_request(14, token, "")?;
            self.wait_sound_operation()
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = token;
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "Native audio diagnostics are not linked.",
            ))
        }
    }
    fn reserve_sound_operation(&self) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call != CallState::Idle
            || state.snapshot.audio_test.is_some()
            || state.snapshot.ringtone_playing
        {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "Stop the current call or audio preview before changing sound.",
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
        let enabled = state
            .snapshot
            .accounts
            .iter()
            .find(|a| a.id == id)
            .is_none_or(|a| a.enabled);
        self.request(
            1,
            id,
            Some(config),
            Some(if enabled { "1" } else { "0" }),
            0,
        )?;
        let previous = state.snapshot.accounts.iter().find(|a| a.id == id).cloned();
        state.pending_accounts.insert(id, previous);
        state.account_results.remove(&id);
        let account = AccountSnapshot {
            id,
            name: name.into(),
            registrar: config.registrar.clone(),
            username: config.username.clone(),
            registration: if enabled {
                RegistrationState::Registering
            } else {
                RegistrationState::Unconfigured
            },
            last_error: None,
            removing: false,
            enabled,
            enabled_changing: true,
            registered_at_ms: None,
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

    /// Activate/deactivate an existing identity without deleting it or changing selection.
    /// Enabling requires its validated saved config; disabling never needs a secret.
    /// Waits for native application, not SIP registration. Busy rejects pending account
    /// work or an occupied call slot; a losing native call race restores prior state.
    pub fn set_account_enabled(
        &mut self,
        id: u32,
        enabled: bool,
        config: Option<&AccountConfig>,
    ) -> Result<(), PhoneError> {
        {
            let mut state = self.snapshot.lock().map_err(|_| state_error())?;
            if state.snapshot.call != CallState::Idle || state.pending_accounts.contains_key(&id) {
                return Err(PhoneError::new(
                    ErrorCode::Busy,
                    "End the call or pending account update before changing activation.",
                ));
            }
            let previous = state
                .snapshot
                .accounts
                .iter()
                .find(|a| a.id == id && !a.removing)
                .cloned()
                .ok_or_else(|| {
                    PhoneError::new(ErrorCode::InvalidInput, "Unknown or removing account.")
                })?;
            if previous.enabled == enabled {
                return Ok(());
            }
            if enabled {
                let config = config.ok_or_else(|| {
                    PhoneError::new(
                        ErrorCode::InvalidInput,
                        "Enabling an account requires its saved configuration.",
                    )
                })?;
                config.validate()?;
                if config.audio_mode != self.audio
                    || config.registrar != previous.registrar
                    || config.username != previous.username
                {
                    return Err(PhoneError::new(
                        ErrorCode::InvalidInput,
                        "Activation cannot replace the saved SIP identity.",
                    ));
                }
            }
            self.request(if enabled { 16 } else { 15 }, id, config, Some("1"), 0)?;
            state.pending_accounts.insert(id, Some(previous));
            state.account_results.remove(&id);
            let account = state
                .snapshot
                .accounts
                .iter_mut()
                .find(|a| a.id == id)
                .ok_or_else(state_error)?;
            account.enabled = enabled;
            account.enabled_changing = true;
            account.registration = if enabled {
                RegistrationState::Registering
            } else {
                RegistrationState::Unconfigured
            };
            account.registered_at_ms = None;
            account.last_error = None;
            state.snapshot.sync_selected_registration();
        }
        self.wait_account_update(id)
    }

    /// Restore disabled, nonsecret saved metadata without creating a native UA.
    /// Config contains a caller-owned placeholder password; no credential is read.
    pub fn restore_disabled_account(
        &mut self,
        name: &str,
        config: &AccountConfig,
    ) -> Result<u32, PhoneError> {
        let id = self.restore_unavailable_account(
            name,
            config,
            PhoneError::new(ErrorCode::InvalidState, "Account disabled."),
        )?;
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let account = state
            .snapshot
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(state_error)?;
        account.enabled = false;
        account.registration = RegistrationState::Unconfigured;
        account.last_error = None;
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
            enabled: true,
            enabled_changing: false,
            registered_at_ms: None,
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
        if state.pending_accounts.contains_key(&id) {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "An account update is still pending.",
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
        // Reserve the native token before returning: an immediate hangup must
        // target this queued attempt even before its first native callback.
        #[cfg(feature = "native")]
        let generation = match self.native.reserve_call_generation() {
            Ok(generation) => generation,
            Err(error) => {
                state.snapshot = previous;
                return Err(error);
            }
        };
        #[cfg(not(feature = "native"))]
        let generation = state.call_generation;
        let previous_generation = state.call_generation;
        state.call_generation = generation;
        state.snapshot.call_id = Some(generation.to_string());
        if let Err(error) = self.request(
            2,
            state.snapshot.selected_account_id.unwrap_or(0),
            None,
            Some(destination),
            generation,
        ) {
            state.call_generation = previous_generation;
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
        let id = state.snapshot.call_id.clone();
        if let Some(call) = state
            .snapshot
            .calls
            .iter_mut()
            .find(|call| Some(&call.id) == id.as_ref())
        {
            call.state = CallState::Answering;
        }
        if let Err(error) = self.request(3, 0, None, None, state.call_generation) {
            state.snapshot.call = CallState::Incoming;
            if let Some(call) = state
                .snapshot
                .calls
                .iter_mut()
                .find(|call| Some(&call.id) == id.as_ref())
            {
                call.state = CallState::Incoming;
            }
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

    /// Reject only an unanswered incoming call matching the opaque snapshot token.
    /// Returns InvalidState for stale tokens, idle, outgoing, answering or active calls.
    /// The native queue repeats this guard before executing, so a late reject cannot end a call.
    pub fn reject_call(&mut self, call_id: &str) -> Result<(), PhoneError> {
        let state = self.snapshot.lock().map_err(|_| state_error())?;
        validate_call_token(&state.snapshot, Some(call_id))?;
        if state.snapshot.call != CallState::Incoming {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "No unanswered incoming call to reject.",
            ));
        }
        self.request(11, 0, None, None, state.call_generation)
    }

    fn hangup_current(&self, call_id: Option<&str>) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let generation = if let Some(id) = call_id {
            if !state.snapshot.calls.iter().any(|call| call.id == id)
                && state.snapshot.call_id.as_deref() != Some(id)
            {
                return Err(PhoneError::new(
                    ErrorCode::InvalidState,
                    "The call changed before this action was submitted.",
                ));
            }
            if state.snapshot.call_id.as_deref() == Some(id) {
                state.call_generation
            } else {
                id.parse().map_err(|_| state_error())?
            }
        } else {
            state.call_generation
        };
        if state.snapshot.call == CallState::Idle {
            return Err(PhoneError::new(ErrorCode::InvalidState, "No call to end."));
        }
        self.request(4, 0, None, None, generation)?;
        if let Some(call) = state
            .snapshot
            .calls
            .iter_mut()
            .find(|call| call.id == generation.to_string())
        {
            call.last_error = None;
        }
        if state.call_generation == generation {
            state.snapshot.last_error = None;
        }
        Ok(())
    }

    /// Mute/unmute outgoing audio for a live established dialog; confirmation arrives natively.
    pub fn set_call_muted(&mut self, call_id: &str, muted: bool) -> Result<(), PhoneError> {
        self.call_control(call_id, OP_MUTE, Some(if muted { "1" } else { "0" }))
    }
    /// Request local hold/resume. Confirmation requires an accepted SIP answer.
    pub fn set_call_held(&mut self, call_id: &str, held: bool) -> Result<(), PhoneError> {
        self.call_control(call_id, OP_HOLD, Some(if held { "1" } else { "0" }))
    }
    /// Send blind REFER to a validated numeric destination and await final NOTIFY.
    pub fn transfer_call(&mut self, call_id: &str, destination: &str) -> Result<(), PhoneError> {
        self.validate_control_destination(destination)?;
        self.call_control(call_id, OP_TRANSFER, Some(destination))
    }
    /// Hold the original before dialing one consultation on the same owning account.
    pub fn start_consultation(
        &mut self,
        call_id: &str,
        destination: &str,
    ) -> Result<(), PhoneError> {
        self.validate_control_destination(destination)?;
        self.call_control(call_id, OP_CONSULT, Some(destination))
    }
    /// Select a live conversation after confirmed hold of the previously audible dialog.
    pub fn switch_call(&mut self, call_id: &str) -> Result<(), PhoneError> {
        self.call_control(call_id, OP_SWITCH, None)
    }
    /// End the consultation and resume the surviving original.
    pub fn cancel_consultation(&mut self, call_id: &str) -> Result<(), PhoneError> {
        self.call_control(call_id, OP_CANCEL_CONSULT, None)
    }
    /// Send REFER/Replaces using the original and established consultation dialogs.
    pub fn complete_transfer(&mut self, call_id: &str) -> Result<(), PhoneError> {
        self.call_control(call_id, OP_COMPLETE_TRANSFER, None)
    }
    fn validate_control_destination(&self, destination: &str) -> Result<(), PhoneError> {
        let mut snapshot = EngineSnapshot {
            registration: RegistrationState::Registered,
            ..Default::default()
        };
        snapshot.reserve_dial(destination)
    }
    fn call_control(
        &self,
        call_id: &str,
        operation: i32,
        destination: Option<&str>,
    ) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let call = state
            .snapshot
            .calls
            .iter()
            .find(|call| call.id == call_id)
            .ok_or_else(|| {
                PhoneError::new(
                    ErrorCode::InvalidState,
                    "The call changed before this action was submitted.",
                )
            })?;
        if operation != OP_MUTE
            && state.snapshot.calls.iter().any(|call| {
                call.hold_pending || call.transfer == phone_core::TransferState::Pending
            })
        {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "A call control is still pending.",
            ));
        }
        if !matches!(call.state, CallState::Active | CallState::Held)
            && operation != OP_CANCEL_CONSULT
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "An established call is required.",
            ));
        }
        if operation == OP_TRANSFER && state.snapshot.calls.len() != 1 {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "Finish consultation before blind transfer.",
            ));
        }
        if matches!(operation, OP_CANCEL_CONSULT | OP_COMPLETE_TRANSFER)
            && (state.snapshot.calls.len() != 2 || !call.consultation)
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "The consultation changed before this action was submitted.",
            ));
        }
        if operation == OP_COMPLETE_TRANSFER
            && state
                .snapshot
                .calls
                .iter()
                .any(|call| !matches!(call.state, CallState::Active | CallState::Held))
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "Both dialogs must be established.",
            ));
        }
        if operation == OP_CONSULT && state.snapshot.calls.len() != 1 {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "Only one consultation is supported.",
            ));
        }
        let generation = call.id.parse().map_err(|_| state_error())?;
        self.request(operation, call.account_id, None, destination, generation)?;
        if let Some(call) = state
            .snapshot
            .calls
            .iter_mut()
            .find(|call| call.id == call_id)
        {
            call.last_error = None;
        }
        if state.snapshot.call_id.as_deref() == Some(call_id) {
            state.snapshot.last_error = None;
        }
        Ok(())
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
    fn history_records_native_connected_and_failed_calls_without_polling() {
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        use phone_core::{CallDirection, CallOutcome, ObservedCallKind};
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-history-observations.wav")),
        )
        .unwrap();
        let (sender, events) = std::sync::mpsc::channel();
        engine.set_call_observer(Some(Arc::new(move |event| {
            let _ = sender.send(event);
        })));
        engine
            .configure_account(&AccountConfig {
                registrar: "127.0.0.1:5070".into(),
                username: "1001".into(),
                password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
                audio_mode: AudioMode::TestTone,
            })
            .unwrap();
        wait(&engine, |state| {
            state.registration == RegistrationState::Registered
        });
        // Deliberately consume lifecycle messages without reading call snapshots.
        for (destination, expected) in [
            ("601", CallOutcome::Completed),
            ("999999", CallOutcome::Failed),
        ] {
            engine.dial(destination).unwrap();
            let started = events.recv_timeout(Duration::from_secs(10)).unwrap();
            assert!(
                matches!(&started.kind, ObservedCallKind::Started { direction: CallDirection::Outgoing, number: Some(number), account: Some(account), .. } if number == destination && account.username == "1001")
            );
            let next = events.recv_timeout(Duration::from_secs(10)).unwrap();
            let finished = if matches!(next.kind, ObservedCallKind::Connected) {
                assert_eq!(expected, CallOutcome::Completed);
                assert_eq!(next.observation_id, started.observation_id);
                events.recv_timeout(Duration::from_secs(10)).unwrap()
            } else {
                next
            };
            assert_eq!(finished.observation_id, started.observation_id);
            match finished.kind {
                ObservedCallKind::Finished {
                    outcome,
                    duration_ms,
                    sip_status,
                } => {
                    assert_eq!(outcome, expected);
                    assert_eq!(duration_ms.is_some(), expected == CallOutcome::Completed);
                    if expected == CallOutcome::Failed {
                        assert_eq!(sip_status, Some(404));
                    }
                }
                _ => panic!("Missing native end observation"),
            }
            // The callback sends while holding engine state; acquiring it waits
            // until its corresponding snapshot projection has completed.
            assert_eq!(engine.snapshot().unwrap().call, CallState::Idle);
        }
        // Queue a cancellation immediately, before waiting for a start snapshot.
        engine.dial("600").unwrap();
        engine.hangup().unwrap();
        loop {
            let event = events.recv_timeout(Duration::from_secs(10)).unwrap();
            if let ObservedCallKind::Finished { outcome, .. } = event.kind {
                assert!(matches!(
                    outcome,
                    CallOutcome::Cancelled | CallOutcome::Completed
                ));
                break;
            }
        }
        assert!(engine.snapshot().unwrap().last_error.is_none());
        engine.shutdown().unwrap();
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn nat_remote_hangup_without_server_contact_rewriting() {
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
    fn initial_registration_accepts_incoming_before_any_outgoing_call() {
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-startup-incoming.wav")),
        )
        .unwrap();
        engine
            .configure_account(&AccountConfig {
                registrar: "127.0.0.1:5070".into(),
                username: "1003".into(),
                password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
                audio_mode: AudioMode::TestTone,
            })
            .unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        assert_eq!(engine.snapshot().unwrap().call, CallState::Idle);
        // This account disables server Contact rewriting. No outgoing call is
        // allowed to prime signaling/media routes before the first inbound one.
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
        let incoming = engine.snapshot().unwrap();
        assert!(incoming.call_id.is_some());
        assert!(incoming.call_account_id.is_some());
        engine.answer().unwrap();
        wait(&engine, |s| s.call == CallState::Active);
        wait(&engine, |s| s.call == CallState::Idle);
        assert!(engine.snapshot().unwrap().last_error.is_none());
        engine.shutdown().unwrap();
    }

    #[test]
    #[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
    fn remote_hangup_releases_outgoing_and_incoming_calls() {
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner());
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
    fn engine_and_uninitialized_reads_share_the_same_revision_sequence() {
        let before_engine = EngineSnapshot::default().with_read_revision();
        let engine = incoming().snapshot().unwrap();
        let after_engine = EngineSnapshot::default().with_read_revision();
        assert!(before_engine.revision < engine.revision);
        assert!(engine.revision < after_engine.revision);
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

#[cfg(all(test, feature = "native"))]
mod audio_diagnostic_tests {
    use super::*;
    use phone_core::AudioTestKind;
    use std::time::{Duration, Instant};

    #[test]
    fn native_audio_tests_are_exclusive_bounded_and_owner_scoped() {
        let _guard = crate::NATIVE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let recording =
            std::env::temp_dir().join(format!("ruf-audio-diagnostics-{}.wav", std::process::id()));
        let mut engine = Engine::start(AudioMode::TestTone, Some(recording.clone())).unwrap();
        let first = engine
            .start_audio_test(AudioTestKind::Microphone, "")
            .unwrap();
        assert_eq!(
            engine
                .start_audio_test(AudioTestKind::Speaker, "")
                .unwrap_err()
                .code,
            ErrorCode::Busy
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine
            .snapshot()
            .unwrap()
            .audio_test
            .as_ref()
            .unwrap()
            .level
            == 0.0
        {
            assert!(
                Instant::now() < deadline,
                "synthetic ausine input must reach the meter"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        engine.stop_audio_test(&first).unwrap();
        assert!(engine.snapshot().unwrap().audio_test.is_none());
        let second = engine.start_audio_test(AudioTestKind::Speaker, "").unwrap();
        assert_ne!(first, second);
        engine.stop_audio_test(&first).unwrap();
        assert_eq!(engine.snapshot().unwrap().audio_test.unwrap().id, second);
        let deadline = Instant::now() + Duration::from_secs(4);
        while engine.snapshot().unwrap().audio_test.is_some() {
            assert!(
                Instant::now() < deadline,
                "speaker test must stop after two seconds"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        engine
            .start_audio_test(AudioTestKind::Microphone, "")
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(12);
        while engine.snapshot().unwrap().audio_test.is_some() {
            assert!(
                Instant::now() < deadline,
                "capture must stop after ten seconds"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        engine
            .start_audio_test(AudioTestKind::Microphone, "")
            .unwrap();
        engine.shutdown().unwrap();
        assert!(engine.snapshot().unwrap().audio_test.is_none());
        let _ = std::fs::remove_file(recording);
    }
}
