use phone_core::{AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, PhoneError};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct EngineState {
    pub(crate) snapshot: EngineSnapshot,
    pub(crate) call_generation: u64,
}

/// One owned native engine, one ephemeral account and one call slot.
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
            let snapshot = Arc::new(Mutex::new(EngineState::default()));
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

    /// Replace the in-memory account and request registration; success means queued.
    pub fn configure_account(&mut self, config: &AccountConfig) -> Result<(), PhoneError> {
        config.validate()?;
        if config.audio_mode != self.audio {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Restart the engine to change the audio backend.",
            ));
        }
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let previous = state.snapshot.clone();
        state.snapshot.reserve_registration()?;
        if let Err(error) = self.request(1, Some(config), None, 0) {
            state.snapshot = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Reserve the single call slot and queue a numeric extension/number.
    pub fn dial(&mut self, destination: &str) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let previous = state.snapshot.clone();
        state.snapshot.reserve_dial(destination)?;
        if let Err(error) = self.request(2, None, Some(destination), 0) {
            state.snapshot = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Accept the currently offered incoming call.
    pub fn answer(&mut self) -> Result<(), PhoneError> {
        let state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call != CallState::Incoming {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "No incoming call to answer.",
            ));
        }
        self.request(3, None, None, state.call_generation)
    }

    /// Reject or end the current call; completion arrives as a native event.
    pub fn hangup(&mut self) -> Result<(), PhoneError> {
        let state = self.snapshot.lock().map_err(|_| state_error())?;
        if state.snapshot.call == CallState::Idle {
            return Err(PhoneError::new(ErrorCode::InvalidState, "No call to end."));
        }
        self.request(4, None, None, state.call_generation)
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
        account: Option<&AccountConfig>,
        destination: Option<&str>,
        call_generation: u64,
    ) -> Result<(), PhoneError> {
        #[cfg(feature = "native")]
        {
            self.native
                .request(operation, account, destination, call_generation)
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = (operation, account, destination, call_generation);
            Err(PhoneError::new(
                ErrorCode::EngineUnavailable,
                "Native support is not linked.",
            ))
        }
    }
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
                .request(operation, None, None, generation_a)
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
