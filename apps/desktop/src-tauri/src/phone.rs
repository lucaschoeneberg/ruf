use baresip_adapter::Engine;
use phone_core::{EngineSnapshot, ErrorCode, PhoneError};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct PhoneState(Arc<Mutex<Option<Engine>>>);

impl PhoneState {
    pub(crate) async fn execute(
        &self,
        operation: impl FnOnce(&mut Option<Engine>) -> Result<EngineSnapshot, PhoneError>
        + Send
        + 'static,
    ) -> Result<EngineSnapshot, PhoneError> {
        let engines = Arc::clone(&self.0);
        tauri::async_runtime::spawn_blocking(move || {
            let mut engine = engines.lock().map_err(|_| lock_error())?;
            operation(&mut engine)
        })
        .await
        .map_err(|_| {
            PhoneError::new(
                ErrorCode::NativeFailure,
                "The native operation could not complete.",
            )
        })?
    }

    pub(crate) fn shutdown(&self) -> Result<(), PhoneError> {
        let mut slot = self.0.lock().map_err(|_| lock_error())?;
        if let Some(mut engine) = slot.take() {
            engine.shutdown()?;
        }
        Ok(())
    }
}

pub(crate) fn active_engine(slot: &mut Option<Engine>) -> Result<&mut Engine, PhoneError> {
    slot.as_mut().ok_or_else(|| {
        PhoneError::new(
            ErrorCode::InvalidState,
            "Configure an account before using the engine.",
        )
    })
}
fn lock_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The desktop engine state is unavailable.",
    )
}
