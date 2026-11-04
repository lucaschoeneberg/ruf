//! Native telephone integration boundary. No C library is linked in this scaffold.

use phone_core::TelephonyStatus;

/// Report actual engine availability; this scaffold cannot register or place calls.
#[must_use]
pub const fn status() -> TelephonyStatus {
    TelephonyStatus::NotLinked
}
