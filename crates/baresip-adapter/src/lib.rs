//! Safe native telephone integration boundary. C handles remain private.

use phone_core::TelephonyStatus;

mod call_journal;
pub use call_journal::CallObserver;
mod engine;
pub use engine::Engine;
mod notifications;
pub use notifications::{
    CallNotificationAction, NotificationPermission, call_notification_permission,
    dismiss_call_notification, init_call_notifications, show_call_notification,
    show_test_notification,
};
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod contacts;
#[cfg(feature = "native")]
#[allow(unsafe_code)]
mod ffi;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod microphone;

/// Query microphone authorization without capturing audio or reading account metadata.
/// `false` never prompts. `true` may prompt only for an undecided decision and must
/// follow an explicit user action. Denied/restricted results require OS settings.
/// Native read/timeout failures return NativeFailure; other platforms return Unsupported.
/// Call on a blocking worker while the application main loop runs.
pub fn microphone_permission(
    request_access: bool,
) -> Result<phone_core::MicrophonePermission, phone_core::PhoneError> {
    #[cfg(target_os = "macos")]
    {
        microphone::permission(request_access)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request_access;
        Ok(phone_core::MicrophonePermission::Unsupported)
    }
}

/// Check system contacts authorization without enumerating any contact records.
///
/// On macOS, `request_access = false` never prompts. `true` may request permission
/// when undecided and must follow an explicit user action. Denial and restriction
/// return permission values; native authorization failures return `NativeFailure`
/// without native error details. Other platforms return `Unsupported`.
/// Call from a blocking worker, never a UI thread; this can wait for a user decision.
pub fn contacts_permission(
    request_access: bool,
) -> Result<phone_core::ContactsPermission, phone_core::PhoneError> {
    #[cfg(target_os = "macos")]
    {
        contacts::permission(request_access)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request_access;
        Ok(phone_core::ContactsPermission::Unsupported)
    }
}

/// Read system contacts, prompting only when `request_access` follows an explicit user action.
///
/// macOS contacts work independently of native SIP linkage. Other platforms return
/// `Unsupported` and an empty collection. Call from a blocking worker, never a UI thread.
pub fn read_contacts(
    request_access: bool,
) -> Result<phone_core::ContactsSnapshot, phone_core::PhoneError> {
    #[cfg(target_os = "macos")]
    {
        contacts::read(request_access)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = request_access;
        Ok(phone_core::ContactsSnapshot {
            permission: phone_core::ContactsPermission::Unsupported,
            contacts: Vec::new(),
        })
    }
}

/// Read the process-local macOS contact-store change counter without reading contacts.
/// The first call retains a native notification observer and store for this process.
/// Changes contain no contact data and never request authorization. Other platforms
/// return zero; focus/manual refresh remain necessary fallbacks for permission changes.
#[must_use]
pub fn contacts_revision() -> u64 {
    #[cfg(target_os = "macos")]
    {
        contacts::revision()
    }
    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

/// Copy the original selected number to the system clipboard without normalization.
/// Accepts 1–1024 UTF-8 bytes without control characters; invalid text returns
/// `InvalidInput`. macOS pasteboard failures return `NativeFailure`; unsupported
/// platforms return `EngineUnavailable`. No contact authorization is required.
/// On macOS, call from a blocking worker while the application's main loop runs;
/// pasteboard work is dispatched synchronously to the main thread when needed.
pub fn copy_phone_number(number: &str) -> Result<(), phone_core::PhoneError> {
    copy_plain_text(number)
}

fn validate_clipboard_text(text: &str) -> Result<(), phone_core::PhoneError> {
    if text.is_empty() || text.len() > 1024 || text.chars().any(char::is_control) {
        return Err(phone_core::PhoneError::new(
            phone_core::ErrorCode::InvalidInput,
            "Clipboard text must contain 1–1024 UTF-8 bytes without control characters.",
        ));
    }
    Ok(())
}

/// Copy bounded plain text using the existing native pasteboard bridge.
/// Accepts 1–1024 UTF-8 bytes without control characters. Invalid text returns
/// `InvalidInput`; native write failures return `NativeFailure`, unsupported
/// platforms return `EngineUnavailable`. Does not request Contacts authorization.
/// Call from a blocking worker while the application's main loop runs.
pub fn copy_plain_text(text: &str) -> Result<(), phone_core::PhoneError> {
    validate_clipboard_text(text)?;
    #[cfg(target_os = "macos")]
    {
        contacts::copy_text(text)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(phone_core::PhoneError::new(
            phone_core::ErrorCode::EngineUnavailable,
            "Native clipboard access is not available on this platform.",
        ))
    }
}

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

#[cfg(all(test, feature = "native"))]
mod sip_event_tests;

// libre owns process-global state; native engine tests must share its lease.
#[cfg(all(test, feature = "native"))]
pub(crate) static NATIVE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod clipboard_tests {
    use super::*;

    #[test]
    fn invalid_clipboard_values_are_rejected_before_native_access() {
        for number in ["", "123\n456", "123\0secret", "123\t456", &"1".repeat(1025)] {
            assert_eq!(
                copy_phone_number(number).unwrap_err().code,
                phone_core::ErrorCode::InvalidInput
            );
        }
    }
    #[test]
    fn generic_clipboard_rejects_invalid_payloads_before_native_access() {
        for text in [
            "",
            "report\nsecond line",
            "report\0suffix",
            &"x".repeat(1025),
        ] {
            assert_eq!(
                copy_plain_text(text).unwrap_err().code,
                phone_core::ErrorCode::InvalidInput
            );
        }
    }

    #[test]
    fn selected_number_is_validated_without_normalizing_its_text() {
        for number in ["+49 (30) 123-45", "123 ext. 7", "*123#", &"1".repeat(1024)] {
            assert!(validate_clipboard_text(number).is_ok());
        }
        assert!(validate_clipboard_text(&"é".repeat(513)).is_err());
    }
}
