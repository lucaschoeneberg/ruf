//! Per-dialog storage behind the legacy selected-call projection.
use crate::engine::EngineState;
use phone_core::{CallSnapshot, CallState, TransferState};

impl EngineState {
    pub(crate) fn select_dialog(&mut self, id: &str) -> bool {
        let Some(call) = self
            .snapshot
            .calls
            .iter()
            .find(|call| call.id == id)
            .cloned()
        else {
            return false;
        };
        self.snapshot.call_id = Some(call.id);
        self.snapshot.call_account_id = Some(call.account_id);
        self.snapshot.call = call.state;
        self.snapshot.caller = call.caller;
        self.snapshot.caller_contact = None;
        self.snapshot.call_connected_at_ms = call.connected_at_ms;
        self.snapshot.call_events = call.events;
        self.snapshot.call_events_dropped = call.events_dropped;
        self.snapshot.last_error = call.last_error;
        self.call_generation = id.parse().unwrap_or_default();
        self.remote_hold = call.remote_held;
        self.local_ending = self.local_endings.contains(&self.call_generation);
        true
    }

    pub(crate) fn save_dialog(&mut self) {
        let Some(id) = self.snapshot.call_id.clone() else {
            return;
        };
        let snapshot = &mut self.snapshot;
        let position = snapshot.calls.iter().position(|call| call.id == id);
        let is_consultation = !snapshot.calls.is_empty();
        let call = if let Some(position) = position {
            &mut snapshot.calls[position]
        } else {
            snapshot.calls.push(CallSnapshot {
                security: Default::default(),
                id,
                account_id: snapshot.call_account_id.unwrap_or_default(),
                state: snapshot.call,
                destination: self.call_destinations.get(&self.call_generation).cloned(),
                caller: None,
                connected_at_ms: None,
                muted: false,
                local_held: false,
                remote_held: false,
                hold_pending: false,
                transfer: TransferState::Idle,
                consultation: is_consultation,
                events: Vec::new(),
                events_dropped: 0,
                last_error: None,
            });
            snapshot.calls.last_mut().expect("inserted dialog")
        };
        call.state = snapshot.call;
        call.account_id = snapshot.call_account_id.unwrap_or_default();
        call.caller = snapshot.caller.clone();
        call.connected_at_ms = snapshot.call_connected_at_ms;
        call.remote_held = self.remote_hold;
        call.events = snapshot.call_events.clone();
        call.events_dropped = snapshot.call_events_dropped;
        call.last_error = snapshot.last_error.clone();
    }

    pub(crate) fn finish_dialog_event(
        &mut self,
        generation: u64,
        previous: Option<String>,
        kind: i32,
    ) {
        if self.snapshot.call == CallState::Idle {
            self.local_endings.remove(&generation);
            self.call_destinations.remove(&generation);
            self.snapshot
                .calls
                .retain(|call| call.id != generation.to_string());
            if self.snapshot.calls.len() == 1 {
                self.snapshot.calls[0].consultation = false;
            }
        } else {
            self.save_dialog();
        }
        let selection = if matches!(kind, 5 | 7) {
            self.snapshot.call_id.clone()
        } else {
            previous
        };
        if let Some(id) = selection
            && self.select_dialog(&id)
        {
            return;
        }
        if let Some(id) = self.snapshot.calls.first().map(|call| call.id.clone()) {
            self.select_dialog(&id);
        }
    }
}
