//! Native call banners. SIP state and action authorization remain in the desktop service.
use phone_core::{ErrorCode, PhoneError};
use std::sync::mpsc::Sender;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod macos;
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows;
#[cfg(any(windows, test))]
mod windows_payload;

/// An explicit native notification response, bound to an opaque banner token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallNotificationAction {
    /// Accept only the still-offered call belonging to this token.
    Answer(String),
    /// Reject only the still-offered call belonging to this token.
    Reject(String),
    /// Show the associated phone; tapping a banner never implicitly answers.
    Open(String),
}

/// Current operating-system notification authorization; querying never prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationPermission {
    /// No user decision has been made yet.
    NotDetermined,
    /// Notifications are disabled by the user or an OS policy.
    Denied,
    /// The OS authorizes delivery, subject to Focus/display preferences.
    Authorized,
    /// This platform or launch context cannot deliver native call banners.
    Unsupported,
}

/// Initialize the per-process native action receiver. No authorization prompt is shown.
/// `register_activation` permits Windows installed-release activation registration;
/// development builds must pass false. Call once during desktop setup.
pub fn init_call_notifications(
    sender: Sender<CallNotificationAction>,
    app_id: &str,
    register_activation: bool,
) -> Result<(), PhoneError> {
    #[cfg(target_os = "macos")]
    {
        macos::init(sender, app_id, register_activation)
    }
    #[cfg(windows)]
    {
        windows::init(sender, app_id, register_activation)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (sender, app_id, register_activation);
        Ok(())
    }
}

/// Query authorization, optionally requesting it after an explicit foreground user action.
/// May wait for the OS response; call from a blocking worker, never the UI thread.
pub fn call_notification_permission(request: bool) -> Result<NotificationPermission, PhoneError> {
    #[cfg(target_os = "macos")]
    {
        macos::permission(request)
    }
    #[cfg(windows)]
    {
        windows::permission(request)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = request;
        Ok(NotificationPermission::Unsupported)
    }
}

/// Deliver a silent call banner with Answer/Reject actions. Never requests permission.
/// Required `answer`/`reject` are localized action labels (1–128 UTF-8 bytes).
/// Title/body are required plain text bounded to 512/1024 bytes. Invalid inputs return `InvalidInput`.
/// The token is opaque and must contain no caller/account information. Call off the UI thread.
pub fn show_call_notification(
    token: &str,
    title: &str,
    body: &str,
    answer: &str,
    reject: &str,
) -> Result<(), PhoneError> {
    validate_token(token)?;
    for (text, bound) in [(title, 512), (body, 1024), (answer, 128), (reject, 128)] {
        if text.is_empty()
            || text.len() > bound
            || text.chars().any(|c| {
                c.is_control() || matches!(c, '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err(input_error());
        }
    }
    #[cfg(target_os = "macos")]
    {
        macos::show(token, title, body, answer, reject)
    }
    #[cfg(windows)]
    {
        windows::show(token, title, body, answer, reject)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "Native call notifications are unavailable on this platform.",
        ))
    }
}

/// Deliver one anonymous, silent settings-test notification without call actions.
/// Required localized title/body must be plain text bounded to 512/1024 UTF-8 bytes; invalid text returns `InvalidInput`.
/// Never requests authorization; the native service rechecks it before submission.
/// Call on a blocking worker. Success means OS acceptance, not guaranteed display.
/// macOS settings and acceptance each have a five-second deadline; late callbacks
/// remain safe and never modify live-call delivery. Windows uses its existing COM
/// worker and a separate notification group, without touching the live-call map.
/// Returns `NativeFailure` on denial, timeout or service failure and
/// `EngineUnavailable` on unsupported platforms or Windows launch contexts.
pub fn show_test_notification(title: &str, body: &str) -> Result<(), PhoneError> {
    for (text, bound) in [(title, 512), (body, 1024)] {
        if text.is_empty()
            || text.len() > bound
            || text.chars().any(|c| {
                c.is_control() || matches!(c, '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            })
        {
            return Err(input_error());
        }
    }
    #[cfg(target_os = "macos")]
    {
        macos::show_test(title, body)
    }
    #[cfg(windows)]
    {
        windows::show_test(title, body)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err(PhoneError::new(
            ErrorCode::EngineUnavailable,
            "Native test notifications are unavailable on this platform.",
        ))
    }
}

/// Remove both pending and delivered notifications for this token. Call off the UI thread.
pub fn dismiss_call_notification(token: &str) -> Result<(), PhoneError> {
    validate_token(token)?;
    #[cfg(target_os = "macos")]
    {
        macos::dismiss(token)
    }
    #[cfg(windows)]
    {
        windows::dismiss(token)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Ok(())
    }
}

fn validate_token(token: &str) -> Result<(), PhoneError> {
    if token.is_empty()
        || token.len() > 128
        || !token
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    {
        return Err(input_error());
    }
    Ok(())
}
fn input_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::InvalidInput,
        "Invalid call notification token or text.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translated_notification_labels_and_test_content_retain_input_bounds() {
        for label in [
            "".into(),
            "x".repeat(129),
            "Answer\nnow".into(),
            "Answer\u{202e}".into(),
        ] {
            assert_eq!(
                show_call_notification("opaque", "Title", "Body", &label, "Reject")
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidInput
            );
            assert_eq!(
                show_call_notification("opaque", "Title", "Body", "Answer", &label)
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidInput
            );
        }
        for title in ["".into(), "x".repeat(513), "Title\u{0}".into()] {
            assert_eq!(
                show_test_notification(&title, "Anonymous body")
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidInput
            );
        }
        assert_eq!(
            show_test_notification("Test", &"x".repeat(1025))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn notification_tokens_are_bounded_and_contain_no_transport_data() {
        assert!(validate_token("session-0123.call-9").is_ok());
        for token in [
            "",
            "sip:alice@example.test",
            "old\nnew",
            "a/b",
            &"x".repeat(129),
        ] {
            assert_eq!(
                validate_token(token).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }
}
