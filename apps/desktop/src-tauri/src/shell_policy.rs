//! Pure desktop lifecycle decisions shared by native handlers and tests.
use phone_core::{CallState, EngineSnapshot, PhoneError};

#[derive(Default)]
pub(crate) struct ObservedState {
    last_success: Option<(EngineSnapshot, bool)>,
}

impl ObservedState {
    pub(crate) fn update(
        &mut self,
        result: Result<(EngineSnapshot, bool), PhoneError>,
    ) -> (EngineSnapshot, bool) {
        match result {
            Ok(value) => {
                self.last_success = Some(value.clone());
                value
            }
            Err(error) => {
                let (mut snapshot, enabled) = self.last_success.clone().unwrap_or_default();
                snapshot.last_error = Some(error);
                (snapshot, enabled)
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct PendingSnapshot(Option<EngineSnapshot>);

impl PendingSnapshot {
    /// Return whether the observer needs to schedule a main-thread delivery.
    pub(crate) fn replace(&mut self, snapshot: EngineSnapshot) -> bool {
        let needs_delivery = self.0.is_none();
        self.0 = Some(snapshot);
        needs_delivery
    }

    pub(crate) fn take(&mut self) -> Option<EngineSnapshot> {
        self.0.take()
    }
}

pub(crate) fn start_hidden(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "--background" | "--notification-activation"))
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
    fn failed_observation_preserves_the_live_call_and_notification_preference() {
        let mut observed = ObservedState::default();
        observed.update(Ok((
            EngineSnapshot {
                call: CallState::Incoming,
                call_id: Some("synthetic-call".into()),
                ..Default::default()
            },
            true,
        )));
        let (snapshot, enabled) = observed.update(Err(PhoneError::new(
            phone_core::ErrorCode::NativeFailure,
            "Synthetic read failure.",
        )));
        assert_eq!(snapshot.call, CallState::Incoming);
        assert_eq!(snapshot.call_id.as_deref(), Some("synthetic-call"));
        assert!(snapshot.last_error.is_some());
        assert!(enabled);
        let (snapshot, enabled) = observed.update(Ok((EngineSnapshot::default(), false)));
        assert_eq!(snapshot.call, CallState::Idle);
        assert!(snapshot.last_error.is_none());
        assert!(!enabled);
    }

    #[test]
    fn a_stalled_main_thread_only_publishes_the_latest_snapshot() {
        let mut pending = PendingSnapshot::default();
        assert!(pending.replace(EngineSnapshot {
            revision: 17,
            call: CallState::Incoming,
            call_id: Some("ended-before-delivery".into()),
            ..Default::default()
        }));
        assert!(!pending.replace(EngineSnapshot {
            revision: 18,
            ..Default::default()
        }));
        let delivered = pending.take().unwrap();
        assert_eq!(delivered.call, CallState::Idle);
        assert_eq!(delivered.revision, 18);
        assert!(pending.take().is_none());
        assert!(pending.replace(EngineSnapshot::default()));
    }

    #[test]
    fn login_start_is_silent_but_normal_open_is_visible() {
        assert!(start_hidden(&["Ruf".into(), "--background".into()]));
        assert!(start_hidden(&[
            "Ruf".into(),
            "--notification-activation".into()
        ]));
        assert!(!start_hidden(&["Ruf".into()]));
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
