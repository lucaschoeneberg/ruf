use phone_core::{AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, PhoneError};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// One owned native engine, one ephemeral account and one call slot.
pub struct Engine {
    snapshot: Arc<Mutex<EngineSnapshot>>,
    audio: AudioMode,
    #[cfg(feature = "native")]
    native: crate::ffi::NativeEngine,
}

impl Engine {
    /// Start the native event loop; a test-tone player requires a WAV output path.
    pub fn start(audio: AudioMode, recording: Option<PathBuf>) -> Result<Self, PhoneError> {
        #[cfg(feature = "native")]
        {
            let snapshot = Arc::new(Mutex::new(EngineSnapshot::default()));
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
            .map(|s| s.clone())
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
        let previous = state.clone();
        state.reserve_registration()?;
        if let Err(error) = self.request(1, Some(config), None) {
            *state = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Reserve the single call slot and queue a numeric extension/number.
    pub fn dial(&mut self, destination: &str) -> Result<(), PhoneError> {
        let mut state = self.snapshot.lock().map_err(|_| state_error())?;
        let previous = state.clone();
        state.reserve_dial(destination)?;
        if let Err(error) = self.request(2, None, Some(destination)) {
            *state = previous;
            return Err(error);
        }
        Ok(())
    }

    /// Accept the currently offered incoming call.
    pub fn answer(&mut self) -> Result<(), PhoneError> {
        if self.snapshot()?.call != CallState::Incoming {
            return Err(PhoneError::new(
                ErrorCode::InvalidState,
                "No incoming call to answer.",
            ));
        }
        self.request(3, None, None)
    }

    /// Reject or end the current call; completion arrives as a native event.
    pub fn hangup(&mut self) -> Result<(), PhoneError> {
        if self.snapshot()?.call == CallState::Idle {
            return Err(PhoneError::new(ErrorCode::InvalidState, "No call to end."));
        }
        self.request(4, None, None)
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
    ) -> Result<(), PhoneError> {
        #[cfg(feature = "native")]
        {
            self.native.request(operation, account, destination)
        }
        #[cfg(not(feature = "native"))]
        {
            let _ = (operation, account, destination);
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
