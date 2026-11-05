//! Whitelisted diagnostic summaries; never serialize personal snapshots for support.
use crate::{ErrorCode, MicrophonePermission, NotificationPermission, TelephonyStatus};
use serde::Serialize;
use ts_rs::TS;

/// Anonymous local health report. Unknown values remain null rather than fabricated success.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsReport {
    /// Application version.
    pub version: String,
    /// Static operating-system family, not a hostname or user identifier.
    pub platform: String,
    /// Compiled native linkage only.
    pub telephony: TelephonyStatus,
    /// Configured accounts, or null if account state could not be read.
    pub accounts_total: Option<u32>,
    /// Accounts with native registration confirmation, or null when unavailable.
    pub accounts_registered: Option<u32>,
    /// Whether native state reports a non-idle call; no peer or call token is included.
    pub call_active: Option<bool>,
    /// Native notification authorization; null if querying failed.
    pub notification_permission: Option<NotificationPermission>,
    /// Microphone authorization metadata only; null if reading failed. Never captures audio.
    pub microphone_permission: Option<MicrophonePermission>,
    /// Both input and output inventory contain endpoints; null if inventory failed.
    /// This does not establish microphone capture, playback or physical audibility.
    pub audio_devices_available: Option<bool>,
    /// A reported credentialAccessRequired, credentialMissing or credentialFailure,
    /// or null when none is reported. No credential probe is made; null is not proof of access.
    pub credential_error: Option<ErrorCode>,
    /// Whether local history reports an operational error.
    pub history_available: bool,
    /// Safe classification only; no native text, addresses or personal records.
    pub last_error: Option<ErrorCode>,
}
