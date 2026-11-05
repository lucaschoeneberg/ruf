//! Platform-independent application contracts. No native handles or UI dependencies.

use serde::Serialize;
use ts_rs::TS;

mod audio_diagnostics;
pub use audio_diagnostics::*;
mod phone;
pub use phone::*;
mod diagnostics;
pub use diagnostics::*;
mod desktop;
pub use desktop::*;
mod contacts;
pub use contacts::*;
mod history;
pub use history::*;

/// Native telephone engine availability. This is not an account registration state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TelephonyStatus {
    /// The native baresip library is not linked into this application yet.
    NotLinked,
    /// Native baresip is linked; registration is reported separately.
    Linked,
}

/// Read-only startup information returned by the desktop IPC API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Application version, aligned with the desktop bundle and root package.
    pub version: String,
    /// Whether the application's native telephone engine is available.
    pub telephony: TelephonyStatus,
}
