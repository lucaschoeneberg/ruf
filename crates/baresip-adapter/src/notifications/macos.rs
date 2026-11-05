//! Owns the process-lifetime callback boundary to UserNotifications.framework.

use super::{CallNotificationAction, NotificationPermission};
use phone_core::{ErrorCode, PhoneError};
use std::{
    ffi::{CString, c_char, c_int},
    sync::{Mutex, OnceLock, mpsc::Sender},
};

type ActionCallback = unsafe extern "C" fn(c_int, *const c_char);

unsafe extern "C" {
    fn sip_call_notifications_init(app_id: *const c_char, callback: ActionCallback) -> c_int;
    fn sip_call_notifications_permission(request: c_int, permission: *mut c_int) -> c_int;
    fn sip_call_notifications_show(
        token: *const c_char,
        title: *const c_char,
        body: *const c_char,
    ) -> c_int;
    fn sip_test_notification_show() -> c_int;
    fn sip_call_notifications_dismiss(token: *const c_char) -> c_int;
}

static SENDER: OnceLock<Mutex<Option<Sender<CallNotificationAction>>>> = OnceLock::new();
static INITIALIZED: OnceLock<Result<(), PhoneError>> = OnceLock::new();
const TOKEN_LIMIT: usize = 128;

pub(super) fn init(
    sender: Sender<CallNotificationAction>,
    app_id: &str,
    _register_activation: bool,
) -> Result<(), PhoneError> {
    if app_id.is_empty()
        || app_id.len() > 255
        || !app_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(native_error());
    }
    let app_id = CString::new(app_id).map_err(|_| native_error())?;
    INITIALIZED
        .get_or_init(|| {
            let _ = SENDER.set(Mutex::new(Some(sender)));
            // SAFETY: the native bridge copies the identifier and retains only the static callback.
            native_result(unsafe { sip_call_notifications_init(app_id.as_ptr(), receive) })
        })
        .clone()
}

pub(super) fn permission(request: bool) -> Result<NotificationPermission, PhoneError> {
    let mut permission = 0;
    // SAFETY: the native function synchronously writes to this live stack value.
    let result = unsafe { sip_call_notifications_permission(i32::from(request), &mut permission) };
    native_result(result)?;
    match permission {
        0 => Ok(NotificationPermission::NotDetermined),
        1 => Ok(NotificationPermission::Denied),
        2 => Ok(NotificationPermission::Authorized),
        3 => Ok(NotificationPermission::Unsupported),
        _ => Err(native_error()),
    }
}

pub(super) fn show(token: &str, title: &str, body: &str) -> Result<(), PhoneError> {
    let token = token_string(token).map_err(|()| native_error())?;
    let title = CString::new(title).map_err(|_| native_error())?;
    let body = CString::new(body).map_err(|_| native_error())?;
    // SAFETY: all buffers remain alive until the native bridge acknowledges the request.
    native_result(unsafe {
        sip_call_notifications_show(token.as_ptr(), title.as_ptr(), body.as_ptr())
    })
}

pub(super) fn show_test() -> Result<(), PhoneError> {
    // SAFETY: no borrowed buffers or callback context cross this synchronous call.
    native_result(unsafe { sip_test_notification_show() })
}

pub(super) fn dismiss(token: &str) -> Result<(), PhoneError> {
    let token = token_string(token).map_err(|()| native_error())?;
    // SAFETY: the native bridge copies the token before this synchronous function returns.
    native_result(unsafe { sip_call_notifications_dismiss(token.as_ptr()) })
}

fn native_result(result: c_int) -> Result<(), PhoneError> {
    if result == 0 {
        Ok(())
    } else {
        Err(native_error())
    }
}

fn native_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "The macOS call notification could not be processed. Check notification settings and use the installed application.",
    )
}

fn token_string(token: &str) -> Result<CString, ()> {
    if token.is_empty()
        || token.len() > TOKEN_LIMIT
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(());
    }
    CString::new(token).map_err(|_| ())
}

unsafe fn callback_token(token: *const c_char) -> Option<String> {
    if token.is_null() {
        return None;
    }
    let mut bytes = Vec::with_capacity(TOKEN_LIMIT);
    for offset in 0..=TOKEN_LIMIT {
        // SAFETY: the native bridge lends a live NUL-terminated buffer for this callback.
        // The loop stops at its terminator and never scans beyond the token limit.
        let byte = unsafe { *token.add(offset) }.cast_unsigned();
        if byte == 0 {
            let token = String::from_utf8(bytes).ok()?;
            token_string(&token).ok()?;
            return Some(token);
        }
        bytes.push(byte);
    }
    None
}

fn action_from_native(action: c_int, token: String) -> Option<CallNotificationAction> {
    if token.starts_with("org.sipsoftphone.settings-test.") {
        return None;
    }
    match action {
        1 => Some(CallNotificationAction::Answer(token)),
        2 => Some(CallNotificationAction::Reject(token)),
        3 => Some(CallNotificationAction::Open(token)),
        _ => None,
    }
}

unsafe extern "C" fn receive(action: c_int, token: *const c_char) {
    // An unwinding Rust callback must never cross the Objective-C ABI boundary.
    let _ = std::panic::catch_unwind(|| {
        // SAFETY: the native delegate guarantees a borrowed, bounded UTF-8 token.
        let Some(token) = (unsafe { callback_token(token) }) else {
            return;
        };
        let Some(action) = action_from_native(action, token) else {
            return;
        };
        if let Some(sender) = SENDER.get()
            && let Ok(sender) = sender.lock()
            && let Some(sender) = sender.as_ref()
        {
            let _ = sender.send(action);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_buttons_preserve_the_bound_call_and_body_click_only_opens() {
        assert!(
            matches!(action_from_native(1, "call-a".into()), Some(CallNotificationAction::Answer(token)) if token == "call-a")
        );
        assert!(
            matches!(action_from_native(2, "call-b".into()), Some(CallNotificationAction::Reject(token)) if token == "call-b")
        );
        assert!(
            matches!(action_from_native(3, "call-c".into()), Some(CallNotificationAction::Open(token)) if token == "call-c")
        );
        assert!(action_from_native(0, "call-a".into()).is_none());
        assert!(action_from_native(4, "call-a".into()).is_none());
    }

    #[test]
    fn settings_test_activation_never_enters_the_call_action_receiver() {
        for action in 1..=3 {
            assert!(
                action_from_native(action, "org.sipsoftphone.settings-test.synthetic".into())
                    .is_none()
            );
        }
    }

    #[test]
    fn native_tokens_are_bounded_opaque_identifiers() {
        for token in ["call-0.a_b", &"a".repeat(128)] {
            let token = token_string(token).unwrap();
            // SAFETY: this owned NUL-terminated token outlives the synchronous read.
            let copied = unsafe { callback_token(token.as_ptr()) }.unwrap();
            assert_eq!(copied.as_bytes(), token.as_bytes());
        }
        for token in [
            "",
            "call\0other",
            "name@example.com",
            "call/other",
            "ä",
            &"a".repeat(129),
        ] {
            assert!(token_string(token).is_err());
        }
        // SAFETY: null is explicitly rejected before any pointer is read.
        assert!(unsafe { callback_token(std::ptr::null()) }.is_none());
        for token in ["", "name@example.com", &"a".repeat(129)] {
            let token = CString::new(token).unwrap();
            // SAFETY: these synthetic buffers remain alive and NUL-terminated.
            assert!(unsafe { callback_token(token.as_ptr()) }.is_none());
        }
    }
}
