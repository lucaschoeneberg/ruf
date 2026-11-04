//! Pure desktop lifecycle decisions shared by native handlers and tests.
use phone_core::CallState;

pub(crate) fn start_hidden(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--background")
}

pub(crate) fn can_answer(call: CallState) -> bool {
    call == CallState::Incoming
}

pub(crate) fn can_hangup(call: CallState) -> bool {
    call != CallState::Idle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_start_is_silent_but_normal_open_is_visible() {
        assert!(start_hidden(&["SIP".into(), "--background".into()]));
        assert!(!start_hidden(&["SIP".into()]));
        assert!(!start_hidden(&["--background=false".into()]));
    }

    #[test]
    fn tray_controls_follow_native_call_state() {
        assert!(can_answer(CallState::Incoming));
        assert!(!can_answer(CallState::Active));
        assert!(!can_answer(CallState::Idle));
        assert!(can_hangup(CallState::Incoming));
        assert!(can_hangup(CallState::Active));
        assert!(!can_hangup(CallState::Idle));
    }
}
