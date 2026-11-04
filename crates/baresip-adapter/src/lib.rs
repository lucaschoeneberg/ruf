//! Safe native telephone integration boundary. C handles remain private.

use phone_core::TelephonyStatus;

mod engine;
pub use engine::Engine;
#[cfg(feature = "native")]
#[allow(unsafe_code)]
mod ffi;

/// Report linkage only; account registration is returned by the engine snapshot.
#[must_use]
pub const fn status() -> TelephonyStatus {
    if cfg!(feature = "native") {
        TelephonyStatus::Linked
    } else {
        TelephonyStatus::NotLinked
    }
}

/// Enumerate current native input/output devices without starting SIP or opening a microphone.
pub fn audio_devices() -> Result<phone_core::AudioDevices, phone_core::PhoneError> {
    #[cfg(feature = "native")]
    {
        ffi::audio_devices()
    }
    #[cfg(not(feature = "native"))]
    {
        Err(phone_core::PhoneError::new(
            phone_core::ErrorCode::EngineUnavailable,
            "Native audio inventory is not linked.",
        ))
    }
}

#[cfg(all(test, feature = "native"))]
mod nat_tests;
