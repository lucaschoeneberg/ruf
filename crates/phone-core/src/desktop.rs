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

#[cfg(test)]
mod preferences_tests {
    use super::*;

    #[test]
    fn preferences_default_preserves_behavior_and_retention_is_bounded() {
        let defaults = AppPreferences::default();
        assert!(defaults.start_hidden);
        assert!(defaults.show_caller_in_notifications);
        assert!(defaults.contacts.favorites_first);
        assert!(defaults.contacts.show_organization);
        assert!(defaults.history.recording_enabled);
        assert_eq!(defaults.history.retention_days, None);
        for days in [30, 90, 365] {
            assert!(
                HistoryPreferences {
                    retention_days: Some(days),
                    ..Default::default()
                }
                .validate()
                .is_ok()
            );
        }
        for days in [0, 1, 29, 31, 366, u16::MAX] {
            assert_eq!(
                HistoryPreferences {
                    retention_days: Some(days),
                    ..Default::default()
                }
                .validate()
                .unwrap_err()
                .code,
                crate::ErrorCode::InvalidInput
            );
        }
    }
}

/// Local display, login-launch and call-history preferences. Contains no credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppPreferences {
    /// Hide the phone window on login launches only; manual launches remain visible.
    pub start_hidden: bool,
    /// Include caller details in native notifications; false uses anonymous text.
    pub show_caller_in_notifications: bool,
    /// Contact list presentation; does not change contact authorization or records.
    pub contacts: ContactsPreferences,
    /// Local call-history collection and retention.
    pub history: HistoryPreferences,
}
impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            start_hidden: true,
            show_caller_in_notifications: true,
            contacts: ContactsPreferences::default(),
            history: HistoryPreferences::default(),
        }
    }
}

/// Presentation preferences applied to existing contacts and favorites.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContactsPreferences {
    /// Sort favorite contacts before other contacts.
    pub favorites_first: bool,
    /// Show organization text alongside contact names.
    pub show_organization: bool,
    /// Match German national 0… numbers with +49…/0049… for identity display only.
    /// Defaults to true; never rewrites the number used for dialing.
    #[serde(default = "german_matching_default")]
    pub match_german_numbers: bool,
}
fn german_matching_default() -> bool {
    true
}
impl Default for ContactsPreferences {
    fn default() -> Self {
        Self {
            favorites_first: true,
            show_organization: true,
            match_german_numbers: true,
        }
    }
}

/// Collection of call metadata only; this does not enable audio recording.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryPreferences {
    /// Record future calls. Changing this requires an idle phone and preserves existing rows.
    pub recording_enabled: bool,
    /// Completed-call retention by start timestamp: null (unlimited), 30, 90 or 365 days.
    /// In-progress records are always protected. Shortening requires explicit confirmation.
    pub retention_days: Option<u16>,
}
impl Default for HistoryPreferences {
    fn default() -> Self {
        Self {
            recording_enabled: true,
            retention_days: None,
        }
    }
}
impl HistoryPreferences {
    /// Reject unsupported retention intervals with `InvalidInput` before saving or deleting.
    pub fn validate(&self) -> Result<(), crate::PhoneError> {
        if self
            .retention_days
            .is_some_and(|days| !matches!(days, 30 | 90 | 365))
        {
            return Err(crate::PhoneError::new(
                crate::ErrorCode::InvalidInput,
                "History retention must be unlimited, 30, 90 or 365 days.",
            ));
        }
        Ok(())
    }
}
