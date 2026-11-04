//! Password-free desktop integration status; operating-system settings remain authoritative.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Actual login-start and notification settings returned by the desktop shell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopSettings {
    /// Whether the operating system currently starts this installed app at login.
    pub autostart_enabled: bool,
    /// False for development builds and apps outside supported installation locations.
    pub autostart_available: bool,
    /// User preference for a single system notification per incoming call.
    pub notifications_enabled: bool,
    /// Actual OS permission when queryable; null on desktop where the plugin cannot query it.
    /// An unknown or denied permission never prevents native ringing or answering.
    pub notifications_granted: Option<bool>,
}
