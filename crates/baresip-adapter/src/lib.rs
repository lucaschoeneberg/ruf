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
