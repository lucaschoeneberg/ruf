//! Ordered observations of native calls, independent of UI snapshot polling.
use phone_core::ObservedCallEvent;
use std::sync::Arc;
#[cfg(any(feature = "native", test))]
use {
    phone_core::{AccountSnapshot, CallDirection, CallOutcome, ObservedCallKind},
    std::{
        collections::BTreeMap,
        panic::{AssertUnwindSafe, catch_unwind},
        sync::atomic::{AtomicU64, Ordering},
        time::{Instant, SystemTime, UNIX_EPOCH},
    },
};

/// A synchronous, nonblocking call observer. Implementations must only enqueue events;
/// they must not access the engine or perform disk/UI work from the native callback.
pub type CallObserver = Arc<dyn Fn(ObservedCallEvent) + Send + Sync>;

#[derive(Default)]
pub(crate) struct CallJournal {
    pub(crate) observer: Option<CallObserver>,
    #[cfg(any(feature = "native", test))]
    calls: BTreeMap<u64, OpenCall>,
}
#[cfg(any(feature = "native", test))]
struct OpenCall {
    id: u64,
    direction: CallDirection,
    connected: Option<Instant>,
    local_end: bool,
}
#[cfg(any(feature = "native", test))]
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(any(feature = "native", test))]
impl CallJournal {
    pub(crate) fn start(
        &mut self,
        token: u64,
        account: Option<AccountSnapshot>,
        direction: CallDirection,
        number: Option<String>,
        remote_name: Option<String>,
    ) {
        if self.calls.contains_key(&token) {
            return;
        }
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.calls.insert(
            token,
            OpenCall {
                id,
                direction,
                connected: None,
                local_end: false,
            },
        );
        self.emit(
            id,
            ObservedCallKind::Started {
                account,
                direction,
                number,
                remote_name,
            },
        );
    }
    pub(crate) fn connected(&mut self, token: u64) {
        let Some(call) = self.calls.get_mut(&token) else {
            return;
        };
        if call.connected.is_some() {
            return;
        }
        call.connected = Some(Instant::now());
        let id = call.id;
        self.emit(id, ObservedCallKind::Connected);
    }
    pub(crate) fn local_end(&mut self, token: u64) {
        if let Some(call) = self.calls.get_mut(&token) {
            call.local_end = true;
        }
    }
    pub(crate) fn finish(&mut self, token: u64, status: i32, failed: bool) {
        let Some(call) = self.calls.remove(&token) else {
            return;
        };
        let sip_status = (!failed && (100..=699).contains(&status)).then_some(status as u16);
        let outcome = if call.connected.is_some() {
            CallOutcome::Completed
        } else if failed {
            CallOutcome::Failed
        } else if call.local_end {
            match call.direction {
                CallDirection::Incoming => CallOutcome::Rejected,
                CallDirection::Outgoing => CallOutcome::Cancelled,
            }
        } else if call.direction == CallDirection::Incoming {
            CallOutcome::Missed
        } else if matches!(sip_status, Some(486 | 600)) {
            CallOutcome::Busy
        } else if matches!(sip_status, Some(603)) {
            CallOutcome::Rejected
        } else {
            CallOutcome::Failed
        };
        self.ended(call, outcome, sip_status);
    }
    pub(crate) fn busy(&mut self, token: u64) {
        if let Some(call) = self.calls.remove(&token) {
            self.ended(call, CallOutcome::Busy, Some(486));
        }
    }
    pub(crate) fn interrupt(&mut self) {
        for (_, call) in std::mem::take(&mut self.calls) {
            self.ended(call, CallOutcome::Interrupted, None);
        }
    }
    fn ended(&self, call: OpenCall, outcome: CallOutcome, sip_status: Option<u16>) {
        self.emit(
            call.id,
            ObservedCallKind::Finished {
                outcome,
                sip_status,
                duration_ms: call
                    .connected
                    .map(|connected| connected.elapsed().as_millis().min(u64::MAX as u128) as u64),
            },
        );
    }
    fn emit(&self, observation_id: u64, kind: ObservedCallKind) {
        if let Some(observer) = &self.observer {
            let at_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(u64::MAX as u128) as u64;
            // A broken history sink must never unwind through C or poison engine state.
            let _ = catch_unwind(AssertUnwindSafe(|| {
                observer(ObservedCallEvent {
                    observation_id,
                    at_ms,
                    kind,
                })
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use phone_core::{CallOutcome, ObservedCallKind};
    use std::sync::Mutex;

    fn journal() -> (CallJournal, Arc<Mutex<Vec<ObservedCallEvent>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        (
            CallJournal {
                observer: Some(Arc::new(move |event| sink.lock().unwrap().push(event))),
                ..Default::default()
            },
            events,
        )
    }
    fn start(journal: &mut CallJournal, token: u64, direction: CallDirection) {
        journal.start(token, None, direction, Some("123".into()), None);
    }
    fn terminal(events: &[ObservedCallEvent]) -> (CallOutcome, Option<u16>, Option<u64>) {
        match events.last().unwrap().kind {
            ObservedCallKind::Finished {
                outcome,
                sip_status,
                duration_ms,
            } => (outcome, sip_status, duration_ms),
            _ => panic!("Missing terminal observation"),
        }
    }
    #[test]
    fn fast_call_is_observed_without_a_snapshot_poll() {
        let (mut journal, events) = journal();
        start(&mut journal, 1, CallDirection::Incoming);
        journal.finish(1, 487, false);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert!(
            matches!(&events[0].kind, ObservedCallKind::Started { number: Some(number), .. } if number == "123")
        );
        assert_eq!(terminal(&events), (CallOutcome::Missed, Some(487), None));
        assert_eq!(events[0].observation_id, events[1].observation_id);
        assert!(events[0].at_ms > 1_700_000_000_000);
    }
    #[test]
    fn local_and_remote_endings_have_distinct_outcomes() {
        for (direction, local, status, expected) in [
            (CallDirection::Incoming, false, 487, CallOutcome::Missed),
            (CallDirection::Incoming, true, 486, CallOutcome::Rejected),
            (CallDirection::Outgoing, true, 487, CallOutcome::Cancelled),
            (CallDirection::Outgoing, false, 486, CallOutcome::Busy),
            (CallDirection::Outgoing, false, 600, CallOutcome::Busy),
            (CallDirection::Outgoing, false, 603, CallOutcome::Rejected),
            (CallDirection::Outgoing, false, 503, CallOutcome::Failed),
        ] {
            let (mut journal, events) = journal();
            start(&mut journal, 1, direction);
            if local {
                journal.local_end(1);
            }
            journal.finish(1, status, false);
            assert_eq!(
                terminal(&events.lock().unwrap()),
                (expected, Some(status as u16), None)
            );
        }
    }
    #[test]
    fn connected_duration_uses_monotonic_time_and_duplicate_events_are_ignored() {
        let (mut journal, events) = journal();
        start(&mut journal, 1, CallDirection::Outgoing);
        start(&mut journal, 1, CallDirection::Outgoing);
        journal.connected(1);
        std::thread::sleep(std::time::Duration::from_millis(3));
        journal.connected(1);
        journal.local_end(999); // Stale native action must not change the active result.
        journal.finish(1, 200, false);
        journal.finish(1, 200, false);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 3);
        let (outcome, code, duration) = terminal(&events);
        assert_eq!((outcome, code), (CallOutcome::Completed, Some(200)));
        assert!(duration.unwrap() >= 3);
    }
    #[test]
    fn busy_offer_is_separate_from_the_active_call() {
        let (mut journal, events) = journal();
        start(&mut journal, 1, CallDirection::Outgoing);
        journal.connected(1);
        start(&mut journal, 2, CallDirection::Incoming);
        journal.busy(2);
        journal.finish(1, 200, false);
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 5);
        assert_ne!(events[0].observation_id, events[2].observation_id);
        assert_eq!(terminal(&events[..4]), (CallOutcome::Busy, Some(486), None));
        assert_eq!(terminal(&events).0, CallOutcome::Completed);
    }
    #[test]
    fn native_errno_never_becomes_a_sip_status() {
        let (mut journal, events) = journal();
        start(&mut journal, 1, CallDirection::Outgoing);
        journal.finish(1, 500, true); // An errno may numerically overlap the SIP range.
        assert_eq!(
            terminal(&events.lock().unwrap()),
            (CallOutcome::Failed, None, None)
        );
    }
    #[test]
    fn ids_survive_engine_recreation_and_shutdown_interrupts_open_calls() {
        let (mut first, a) = journal();
        let (mut second, b) = journal();
        start(&mut first, 1, CallDirection::Incoming);
        start(&mut second, 1, CallDirection::Incoming);
        first.interrupt();
        assert_ne!(
            a.lock().unwrap()[0].observation_id,
            b.lock().unwrap()[0].observation_id
        );
        assert_eq!(
            terminal(&a.lock().unwrap()),
            (CallOutcome::Interrupted, None, None)
        );
    }
    #[test]
    fn withheld_identity_remains_unavailable_in_observations() {
        let (mut journal, events) = journal();
        journal.start(
            1,
            None,
            CallDirection::Incoming,
            None,
            Some("Unterdrückte Rufnummer".into()),
        );
        journal.finish(1, 487, false);
        let events = events.lock().unwrap();
        assert!(
            matches!(&events[0].kind, ObservedCallKind::Started { number: None, remote_name: Some(name), .. } if name == "Unterdrückte Rufnummer")
        );
    }
}
