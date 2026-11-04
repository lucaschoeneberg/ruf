//! SIP response classification must remain distinct from native operation errors.
use phone_core::{ErrorCode, PhoneError};

#[test]
fn sip_failures_preserve_status_and_have_actionable_classifications() {
    for (status, code) in [
        (302, ErrorCode::SipRedirect),
        (401, ErrorCode::SipAuthenticationFailed),
        (403, ErrorCode::SipForbidden),
        (404, ErrorCode::SipNotFound),
        (408, ErrorCode::SipTimeout),
        (480, ErrorCode::SipUnavailable),
        (486, ErrorCode::SipBusy),
        (488, ErrorCode::SipNotAcceptable),
        (503, ErrorCode::SipUnavailable),
        (603, ErrorCode::SipRejected),
        (604, ErrorCode::SipFailure),
    ] {
        let error = PhoneError::from_sip_status(status).unwrap();
        assert_eq!(error.code, code, "SIP {status}");
        assert_eq!(error.sip_status, Some(status));
    }
}

#[test]
fn successful_responses_and_normal_cancellation_are_not_call_errors() {
    for status in [0, 100, 180, 183, 200, 299, 487, 700, 999] {
        assert!(PhoneError::from_sip_status(status).is_none(), "{status}");
    }
    assert_eq!(
        PhoneError::new(ErrorCode::NativeFailure, "native error").sip_status,
        None
    );
}
