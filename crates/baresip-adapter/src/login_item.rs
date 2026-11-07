//! ServiceManagement boundary; the desktop selects it only for App Store builds.
use phone_core::{ErrorCode, PhoneError};

unsafe extern "C" {
    fn sip_login_item_enabled(enabled: *mut i32) -> i32;
    fn sip_login_item_set_enabled(enabled: i32) -> i32;
    fn sip_login_item_launched_at_login() -> i32;
}

fn login_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "Login start is unavailable or requires approval. Check System Settings > General > Login Items.",
    )
}

fn decode_enabled(value: i32) -> Result<bool, PhoneError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(login_error()),
    }
}

/// Read this main app's ServiceManagement login state without changing it.
/// Only native `SMAppServiceStatusEnabled` returns true; unregistered,
/// approval-required and not-found states return false. A fresh app may report
/// not-found before its first registration. Unknown status and native failures
/// return `NativeFailure` without native error details.
/// Requires macOS 13+; this app supports macOS 14+. Does not open System Settings.
pub fn login_item_enabled() -> Result<bool, PhoneError> {
    let mut enabled = -1;
    // SAFETY: synchronously writes one integer into a valid exclusive pointer.
    if unsafe { sip_login_item_enabled(&mut enabled) } != 0 {
        return Err(login_error());
    }
    decode_enabled(enabled)
}

/// Register/unregister this signed main app with `SMAppService.mainApp`.
/// Idempotent for enabled/unregistered states (including not-found when disabling).
/// Not-found permits a first registration attempt; macOS validates the app.
/// Returns `NativeFailure` if macOS
/// rejects the operation, still requires approval, or does not report the desired
/// status. Does not open System Settings, re-register an approval-required item,
/// create a helper, or touch legacy LaunchAgent files. Call on a blocking worker.
pub fn set_login_item_enabled(enabled: bool) -> Result<(), PhoneError> {
    // SAFETY: a bounded boolean is passed by value; no handles cross the bridge.
    if unsafe { sip_login_item_set_enabled(i32::from(enabled)) } != 0 {
        return Err(login_error());
    }
    Ok(())
}

/// Inspect the current macOS open-application Apple event for login-item launch.
/// Call during initial main-thread application setup. Manual launches, worker
/// calls, absent events and native failures return false. No permissions or
/// registration are requested. SMAppService supplies this event instead of CLI args.
#[must_use]
pub fn launched_as_login_item() -> bool {
    // SAFETY: read-only, argument-free native operation returns a boolean integer.
    unsafe { sip_login_item_launched_at_login() == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_native_booleans_are_accepted() {
        assert!(!decode_enabled(0).unwrap());
        assert!(decode_enabled(1).unwrap());
        for value in [-1, 2, i32::MAX] {
            let error = decode_enabled(value).unwrap_err();
            assert_eq!(error.code, ErrorCode::NativeFailure);
            assert!(!error.message.contains("private-native-detail"));
        }
    }

    #[test]
    fn read_only_status_query_never_registers_a_login_item() {
        // A test runner is not an installed main app. Known disabled states,
        // including NotFound, decode to false; native exceptions still fail safely.
        if let Err(error) = login_item_enabled() {
            assert_eq!(error.code, ErrorCode::NativeFailure);
        }
        assert!(!launched_as_login_item());
    }
}
