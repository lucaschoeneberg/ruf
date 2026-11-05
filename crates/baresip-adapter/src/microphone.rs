//! Platform authorization metadata; independent of engine startup and account storage.
use phone_core::{ErrorCode, MicrophonePermission, PhoneError};
unsafe extern "C" {
    fn sip_microphone_permission(request_access: i32, permission: *mut i32) -> i32;
}
fn decode_permission(value: i32) -> Result<MicrophonePermission, PhoneError> {
    match value {
        0 => Ok(MicrophonePermission::NotDetermined),
        1 => Ok(MicrophonePermission::Restricted),
        2 => Ok(MicrophonePermission::Denied),
        3 => Ok(MicrophonePermission::Authorized),
        _ => Err(authorization_error()),
    }
}
fn authorization_error() -> PhoneError {
    PhoneError::new(
        ErrorCode::NativeFailure,
        "Microphone authorization could not be read.",
    )
}
pub(crate) fn permission(request_access: bool) -> Result<MicrophonePermission, PhoneError> {
    let mut value = -1;
    // SAFETY: native code synchronously writes one integer; it never captures audio.
    if unsafe { sip_microphone_permission(i32::from(request_access), &mut value) } != 0 {
        return Err(authorization_error());
    }
    decode_permission(value)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_permission_values_are_explicit_and_unknown_values_fail() {
        assert_eq!(
            decode_permission(0).unwrap(),
            MicrophonePermission::NotDetermined
        );
        assert_eq!(
            decode_permission(1).unwrap(),
            MicrophonePermission::Restricted
        );
        assert_eq!(decode_permission(2).unwrap(), MicrophonePermission::Denied);
        assert_eq!(
            decode_permission(3).unwrap(),
            MicrophonePermission::Authorized
        );
        assert_eq!(
            decode_permission(-1).unwrap_err().code,
            ErrorCode::NativeFailure
        );
        assert_eq!(
            decode_permission(4).unwrap_err().code,
            ErrorCode::NativeFailure
        );
    }
    #[test]
    fn read_only_authorization_works_without_an_engine_or_metadata() {
        assert!(permission(false).is_ok());
    }
}
