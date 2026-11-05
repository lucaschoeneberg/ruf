//! Password-free desktop integration status; operating-system settings remain authoritative.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Fixed external destinations available from the application's About page.
/// URLs are selected by the desktop shell; callers cannot supply arbitrary URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AboutLink {
    /// Mardu GmbH's company website.
    Company,
    /// Mardu GmbH's legal notice.
    Imprint,
    /// Mardu GmbH's website privacy policy.
    Privacy,
}

/// Native operating-system notification authorization; reading it never prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NotificationPermission {
    /// No notification permission decision has been made; a foreground request may prompt.
    NotDetermined,
    /// The user or operating-system policy disabled notifications; use OS settings to change it.
    Denied,
    /// Notification delivery is authorized, subject to the user's display and Focus settings.
    Authorized,
    /// This platform or launch context cannot deliver native call notifications.
    Unsupported,
}

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
    /// Canonical native OS authorization; distinguishes the first request from prior denial.
    /// A read never opens a permission prompt, and denial does not prevent native ringing.
    pub notification_permission: NotificationPermission,
    /// Compatibility projection: true for authorized, false for notDetermined/denied,
    /// and null for unsupported. Use notificationPermission to choose recovery actions.
    pub notifications_granted: Option<bool>,
}
