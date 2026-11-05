//! Real native SIP transactions against a synthetic loopback registrar/peer.
//! Run: `cargo test -p baresip-adapter --features native sip_event_tests`.
use crate::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallEventKind, CallState, EngineSnapshot, ErrorCode,
    RegistrationState,
};
use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(5);

fn header<'a>(packet: &'a str, name: &str) -> &'a str {
    packet
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .expect("synthetic SIP header")
        .trim()
}

fn uri(value: &str) -> &str {
    value.split_once('<').unwrap().1.split_once('>').unwrap().0
}

fn sdp(version: u32, direction: &str) -> String {
    format!(
        "v=0\r\no=events 1 {version} IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\na={direction}\r\n"
    )
}

struct Lab {
    engine: Engine,
    peer: UdpSocket,
    local: SocketAddr,
    target: String,
    incoming_sequence: u32,
}

impl Lab {
    fn new(label: &str) -> Self {
        Self::registration_response(label, "200 OK")
    }

    fn registration_response(label: &str, status: &str) -> Self {
        let (lab, registration) = Self::pending_registration(label);
        lab.reply(&registration, status, "", "");
        let expected = if status.starts_with("200 ") {
            RegistrationState::Registered
        } else {
            RegistrationState::Failed
        };
        lab.wait(|s| s.registration == expected);
        lab
    }

    fn pending_registration(label: &str) -> (Self, String) {
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let endpoint = peer.local_addr().unwrap();
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join(format!("sip-events-{label}.wav"))),
        )
        .unwrap();
        engine
            .configure_account(&AccountConfig {
                registrar: endpoint.to_string(),
                username: "events-test".into(),
                password: "synthetic-only".into(),
                audio_mode: AudioMode::TestTone,
            })
            .unwrap();
        let until = Instant::now() + TIMEOUT;
        let (registration, local) = loop {
            if let Some(packet) = Self::read(&peer) {
                assert!(packet.0.starts_with("REGISTER "));
                break packet;
            }
            assert!(Instant::now() < until, "native REGISTER timed out");
        };
        let target = uri(header(&registration, "Contact:")).to_owned();
        let lab = Self {
            engine,
            peer,
            local,
            target,
            incoming_sequence: 0,
        };
        (lab, registration)
    }

    fn read(socket: &UdpSocket) -> Option<(String, SocketAddr)> {
        let mut bytes = [0; 8192];
        match socket.recv_from(&mut bytes) {
            Ok((len, source)) => Some((String::from_utf8(bytes[..len].to_vec()).unwrap(), source)),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                None
            }
            Err(error) => panic!("synthetic SIP receive failed: {error}"),
        }
    }

    fn send(&self, packet: &str) {
        self.peer.send_to(packet.as_bytes(), self.local).unwrap();
    }

    fn reply(&self, request: &str, status: &str, extra: &str, body: &str) {
        let to = header(request, "To:");
        let to_tag = if to.contains(";tag=") {
            ""
        } else {
            ";tag=peer"
        };
        let contact = if request.starts_with("REGISTER ") {
            header(request, "Contact:").to_owned()
        } else {
            format!("<sip:peer@{}>", self.peer.local_addr().unwrap())
        };
        let content_type = if body.is_empty() {
            ""
        } else {
            "Content-Type: application/sdp\r\n"
        };
        self.send(&format!(
            "SIP/2.0 {status}\r\nVia: {}\r\nFrom: {}\r\nTo: {to}{to_tag}\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: {contact}\r\n{extra}{content_type}Content-Length: {}\r\n\r\n{body}",
            header(request, "Via:"),
            header(request, "From:"),
            header(request, "Call-ID:"),
            header(request, "CSeq:"),
            body.len(),
        ));
    }

    fn receive(&self, description: &str, predicate: impl Fn(&str) -> bool) -> String {
        self.receive_for(TIMEOUT, description, predicate)
    }

    fn receive_for(
        &self,
        timeout: Duration,
        description: &str,
        predicate: impl Fn(&str) -> bool,
    ) -> String {
        let until = Instant::now() + timeout;
        loop {
            if let Some((packet, _)) = Self::read(&self.peer) {
                if predicate(&packet) {
                    return packet;
                }
                self.service(&packet);
            }
            assert!(Instant::now() < until, "{description} timed out");
        }
    }

    fn service(&self, packet: &str) {
        if packet.starts_with("INVITE ") {
            // Native media/NAT updates are real re-INVITEs and need an answer.
            self.reply(packet, "200 OK", "", &sdp(20, "sendrecv"));
        } else if packet.starts_with("REGISTER ") || packet.starts_with("BYE ") {
            self.reply(packet, "200 OK", "", "");
        }
    }

    fn settle(&self) {
        let until = Instant::now() + Duration::from_millis(150);
        while Instant::now() < until {
            if let Some((packet, _)) = Self::read(&self.peer) {
                self.service(&packet);
            }
        }
    }

    fn wait(&self, predicate: impl Fn(&EngineSnapshot) -> bool) -> EngineSnapshot {
        let until = Instant::now() + TIMEOUT;
        loop {
            let state = self.engine.snapshot().unwrap();
            if predicate(&state) {
                return state;
            }
            assert!(
                Instant::now() < until,
                "native event state timed out: call={:?}, calls={:?}, error={:?}",
                state.call,
                state.calls,
                state.last_error
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn offer(&mut self) -> String {
        self.incoming_sequence += 1;
        let sequence = self.incoming_sequence;
        let endpoint = self.peer.local_addr().unwrap();
        let body = sdp(1, "sendrecv");
        self.send(&format!(
            "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-initial-{sequence};rport\r\nMax-Forwards: 70\r\nFrom: \"Event Caller\" <sip:12345@lab>;tag=caller\r\nTo: <{}>\r\nCall-ID: synthetic-events-incoming-{sequence}\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{endpoint}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{body}",
            self.target, self.target, body.len(),
        ));
        self.wait(|s| s.call == CallState::Incoming);
        self.receive("incoming ringing", |p| p.starts_with("SIP/2.0 180"))
    }

    fn incoming(&mut self) -> String {
        self.offer();
        self.engine.answer().unwrap();
        let response = self.response(200, "1 INVITE");
        self.request(&response, "ACK", 1, "", "");
        self.wait(|s| s.call == CallState::Active);
        self.settle();
        response
    }

    fn request(&self, dialog: &str, method: &str, sequence: u32, extra: &str, body: &str) {
        let endpoint = self.peer.local_addr().unwrap();
        let dialog_id = header(dialog, "Call-ID:");
        self.send(&format!(
            "{method} {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-{method}-{sequence}-{dialog_id};rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {sequence} {method}\r\nContact: <sip:caller@{endpoint}>\r\n{extra}Content-Length: {}\r\n\r\n{body}",
            uri(header(dialog, "Contact:")),
            header(dialog, "From:"),
            header(dialog, "To:"),
            header(dialog, "Call-ID:"),
            body.len(),
        ));
    }

    fn response(&self, status: u16, cseq: &str) -> String {
        let response = self.receive("SIP response", |p| {
            p.starts_with("SIP/2.0 ") && header(p, "CSeq:") == cseq
        });
        let actual = response.split_whitespace().nth(1).unwrap();
        assert_eq!(
            actual,
            status.to_string(),
            "unexpected synthetic {cseq} response status"
        );
        response
    }

    fn outgoing(&mut self) -> String {
        self.engine.dial("4321").unwrap();
        self.receive("outgoing INVITE", |p| p.starts_with("INVITE "))
    }

    fn end_incoming(&self, dialog: &str, sequence: u32) {
        self.request(dialog, "BYE", sequence, "", "");
        self.response(200, &format!("{sequence} BYE"));
        self.wait(|s| s.call == CallState::Idle);
    }
}

#[test]
fn incoming_hold_resume_preserves_caller_and_initial_connection_time() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("hold");
    let dialog = lab.incoming();
    let connected = lab.engine.snapshot().unwrap();
    assert!(connected.call_connected_at_ms.is_some());
    assert_eq!(
        connected.caller.as_ref().unwrap().number.as_deref(),
        Some("12345")
    );
    for (sequence, direction, expected) in [
        (2, "sendonly", CallState::Held),
        (3, "sendrecv", CallState::Active),
    ] {
        lab.request(
            &dialog,
            "INVITE",
            sequence,
            "Content-Type: application/sdp\r\n",
            &sdp(sequence, direction),
        );
        let response = lab.response(200, &format!("{sequence} INVITE"));
        lab.request(&response, "ACK", sequence, "", "");
        let state = lab.wait(|s| s.call == expected);
        assert_eq!(state.caller, connected.caller);
        assert_eq!(state.call_id, connected.call_id);
        assert_eq!(state.call_connected_at_ms, connected.call_connected_at_ms);
        assert!(state.revision > connected.revision);
    }
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(
        state
            .call_events
            .iter()
            .map(|e| &e.event)
            .collect::<Vec<_>>(),
        vec![&CallEventKind::Held, &CallEventKind::Resumed]
    );
    lab.end_incoming(&dialog, 4);
    lab.engine.shutdown().unwrap();
}

#[test]
fn info_dtmf_start_end_burst_survives_snapshot_polling_and_call_end() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("dtmf");
    let dialog = lab.incoming();
    let token = lab.engine.snapshot().unwrap().call_id.unwrap();
    // Do not poll engine snapshots between INFO requests: both short signals
    // must remain observable after their native duration timers have expired.
    for (sequence, digit) in [(2, "5"), (3, "#"), (4, "a")] {
        lab.request(
            &dialog,
            "INFO",
            sequence,
            "Content-Type: application/dtmf-relay\r\n",
            &format!("Signal={digit}\r\nDuration=10\r\n"),
        );
        lab.response(200, &format!("{sequence} INFO"));
        thread::sleep(Duration::from_millis(40));
    }
    let state = lab.wait(|s| s.call_events.len() >= 6);
    let expected = ["5", "#", "A"]
        .into_iter()
        .flat_map(|digit| {
            [
                CallEventKind::DtmfStart {
                    digit: digit.into(),
                },
                CallEventKind::DtmfEnd,
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(
        state
            .call_events
            .iter()
            .map(|e| e.event.clone())
            .collect::<Vec<_>>(),
        expected
    );
    assert!(state.call_events.iter().all(|event| event.call_id == token));
    assert!(
        state
            .call_events
            .windows(2)
            .all(|events| events[1].sequence == events[0].sequence.wrapping_add(1))
    );
    assert_eq!(state.call_events_dropped, 0);
    lab.end_incoming(&dialog, 5);
    assert_eq!(
        lab.engine.snapshot().unwrap().call_events,
        state.call_events
    );
    lab.engine.shutdown().unwrap();
}

#[test]
fn outgoing_provisional_sdp_reports_early_media_before_establishment() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("progress");
    let invite = lab.outgoing();
    lab.reply(&invite, "183 Session Progress", "", &sdp(1, "sendrecv"));
    let early = lab.wait(|s| s.call == CallState::EarlyMedia);
    assert!(early.call_connected_at_ms.is_none());
    assert!(
        early
            .call_events
            .iter()
            .any(|event| event.event == CallEventKind::Progress)
    );
    lab.reply(&invite, "200 OK", "", &sdp(2, "sendrecv"));
    lab.receive("outgoing ACK", |p| p.starts_with("ACK "));
    let active = lab.wait(|s| s.call == CallState::Active);
    assert!(active.call_connected_at_ms.is_some());
    assert_eq!(active.call_id, early.call_id);
    lab.engine.hangup().unwrap();
    let bye = lab.receive("outgoing BYE", |p| p.starts_with("BYE "));
    lab.reply(&bye, "200 OK", "", "");
    lab.wait(|s| s.call == CallState::Idle);
    lab.engine.shutdown().unwrap();
}

#[test]
fn invalid_provisional_sdp_reports_a_native_failure_before_establishment() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("invalid-sdp");
    let invite = lab.outgoing();
    // baresip closes the session with a local decoder error, without a final
    // SIP response. Losing that error makes a failed attempt look like idle.
    lab.reply(&invite, "183 Session Progress", "", "v=invalid\r\n");
    let state = lab.wait(|s| s.call == CallState::Idle);
    let error = state
        .last_error
        .expect("native setup failure must be visible");
    assert_eq!(error.code, ErrorCode::NativeFailure);
    assert_eq!(error.sip_status, None);
    assert!(state.call_id.is_none());
    assert!(state.call_connected_at_ms.is_none());
    lab.engine.shutdown().unwrap();
}

#[test]
fn outgoing_final_responses_preserve_sip_status_and_classify_errors() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("errors");
    for (status, expected) in [
        (404, ErrorCode::SipNotFound),
        (480, ErrorCode::SipUnavailable),
        (486, ErrorCode::SipBusy),
        (503, ErrorCode::SipUnavailable),
    ] {
        let invite = lab.outgoing();
        lab.reply(&invite, &format!("{status} Synthetic Failure"), "", "");
        lab.receive("failed INVITE ACK", |p| p.starts_with("ACK "));
        let state = lab.wait(|s| s.call == CallState::Idle);
        let error = state
            .last_error
            .expect("final response must have a safe error");
        assert_eq!(error.code, expected, "SIP {status}");
        assert_eq!(error.sip_status, Some(status));
        assert!(state.call_id.is_none());
        assert!(state.call_connected_at_ms.is_none());
    }
    lab.engine.shutdown().unwrap();
}

#[test]
fn redirect_is_retained_without_dialing_its_contact() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("redirect");
    let redirect = UdpSocket::bind("127.0.0.1:0").unwrap();
    redirect
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let invite = lab.outgoing();
    // Reply uses the synthetic redirect endpoint as the only Contact.
    let response = format!(
        "SIP/2.0 302 Moved Temporarily\r\nVia: {}\r\nFrom: {}\r\nTo: {};tag=redirect\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:redirect@{}>\r\nContent-Length: 0\r\n\r\n",
        header(&invite, "Via:"),
        header(&invite, "From:"),
        header(&invite, "To:"),
        header(&invite, "Call-ID:"),
        header(&invite, "CSeq:"),
        redirect.local_addr().unwrap(),
    );
    lab.send(&response);
    lab.receive("redirect ACK", |p| p.starts_with("ACK "));
    let state = lab.wait(|s| s.call == CallState::Idle);
    let error = state.last_error.unwrap();
    assert_eq!(error.code, ErrorCode::SipRedirect);
    assert_eq!(error.sip_status, Some(302));
    assert!(
        state
            .call_events
            .iter()
            .any(|event| event.event == CallEventKind::Redirected { sip_status: 302 })
    );
    assert!(
        Lab::read(&redirect).is_none(),
        "redirect target received an unsolicited request"
    );
    lab.engine.shutdown().unwrap();
}

#[test]
fn out_of_dialog_refer_without_subscription_is_rejected_and_keeps_existing_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("refer");
    let dialog = lab.incoming();
    let before = lab.engine.snapshot().unwrap();
    let endpoint = lab.peer.local_addr().unwrap();
    lab.send(&format!(
        "REFER {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-refer;rport\r\nMax-Forwards: 70\r\nFrom: <sip:referrer@lab>;tag=referrer\r\nTo: <{}>\r\nCall-ID: synthetic-events-outside-refer\r\nCSeq: 1 REFER\r\nContact: <sip:referrer@{endpoint}>\r\nRefer-To: <sip:transfer@{endpoint}>\r\nRefer-Sub: false\r\nContent-Length: 0\r\n\r\n",
        lab.target, lab.target,
    ));
    lab.response(405, "1 REFER");
    let after = lab.engine.snapshot().unwrap();
    assert_eq!(after.call, CallState::Active);
    assert_eq!(after.call_id, before.call_id);
    assert_eq!(after.caller, before.caller);
    assert_eq!(after.call_connected_at_ms, before.call_connected_at_ms);
    lab.end_incoming(&dialog, 2);
    lab.engine.shutdown().unwrap();
}

#[test]
fn terminal_registration_failures_preserve_sip_status_and_error_classification() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for (status, expected) in [
        (403, ErrorCode::SipForbidden),
        (503, ErrorCode::SipUnavailable),
    ] {
        let mut lab = Lab::registration_response(
            "registration-errors",
            &format!("{status} Synthetic Failure"),
        );
        let state = lab.engine.snapshot().unwrap();
        let account = state.accounts.first().unwrap();
        assert_eq!(account.registration, RegistrationState::Failed);
        let error = account
            .last_error
            .as_ref()
            .expect("failed registration must have a safe error");
        assert_eq!(error.code, expected);
        assert_eq!(error.sip_status, Some(status));
        lab.engine.shutdown().unwrap();
    }
}

#[test]
fn retiring_accounts_finish_pending_register_before_unregistering() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for replace in [false, true] {
        let (mut lab, pending) = Lab::pending_registration("retire-pending");
        let id = lab.engine.snapshot().unwrap().selected_account_id.unwrap();
        lab.engine.wait_account_update(id).unwrap();
        if replace {
            lab.engine
                .save_account(
                    Some(id),
                    "Replacement",
                    &AccountConfig {
                        registrar: lab.peer.local_addr().unwrap().to_string(),
                        username: "replacement-test".into(),
                        password: "synthetic-only".into(),
                        audio_mode: AudioMode::TestTone,
                    },
                )
                .unwrap();
            lab.engine.wait_account_update(id).unwrap();
        } else {
            lab.engine.remove_account(id).unwrap();
            lab.engine.wait_account_removed(id).unwrap();
        }

        // Another UA may register, but the retired UA must keep its one request
        // until completion. Overwriting that request's owner leaves a dangling
        // callback after the unregister response frees the registration.
        let until = Instant::now() + Duration::from_millis(150);
        while Instant::now() < until {
            if let Some((packet, _)) = Lab::read(&lab.peer) {
                assert!(
                    header(&packet, "Call-ID:") != header(&pending, "Call-ID:")
                        || header(&packet, "CSeq:") == header(&pending, "CSeq:"),
                    "retired account sent overlapping REGISTER/unregister"
                );
                if header(&packet, "Call-ID:") != header(&pending, "Call-ID:") {
                    lab.reply(&packet, "200 OK", "", "");
                }
            }
        }

        lab.reply(&pending, "200 OK", "", "");
        let unregister = lab.receive("retired account unregister", |packet| {
            packet.starts_with("REGISTER ")
                && header(packet, "Call-ID:") == header(&pending, "Call-ID:")
                && header(packet, "Contact:").contains("expires=0")
        });
        lab.reply(&unregister, "200 OK", "", "");
        // Duplicate final responses must not revive the retired account.
        lab.reply(&pending, "200 OK", "", "");
        lab.settle();
        lab.wait(|s| {
            if replace {
                s.accounts.len() == 1
                    && s.accounts[0].username == "replacement-test"
                    && s.registration == RegistrationState::Registered
            } else {
                s.accounts.is_empty() && s.registration == RegistrationState::Unconfigured
            }
        });
        lab.engine.shutdown().unwrap();
    }
}

#[test]
fn retired_register_timeout_preserves_replacement_registration() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let (mut lab, pending) = Lab::pending_registration("retire-timeout");
    let id = lab.engine.snapshot().unwrap().selected_account_id.unwrap();
    lab.engine.wait_account_update(id).unwrap();
    lab.engine
        .save_account(
            Some(id),
            "Replacement",
            &AccountConfig {
                registrar: lab.peer.local_addr().unwrap().to_string(),
                username: "events-test".into(),
                password: "replacement-synthetic-only".into(),
                audio_mode: AudioMode::TestTone,
            },
        )
        .unwrap();
    lab.engine.wait_account_update(id).unwrap();

    // libre's UDP transaction timeout is 64 * T1 = 32 seconds. Keep the
    // original unanswered while servicing the replacement's real requests.
    let until = Instant::now() + Duration::from_secs(33);
    while Instant::now() < until {
        if let Some((packet, _)) = Lab::read(&lab.peer) {
            if header(&packet, "Call-ID:") == header(&pending, "Call-ID:") {
                assert_eq!(header(&packet, "CSeq:"), header(&pending, "CSeq:"));
            } else {
                lab.service(&packet);
            }
        }
    }
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.accounts.len(), 1);
    assert_eq!(state.accounts[0].id, id);
    assert_eq!(state.accounts[0].name, "Replacement");
    assert_eq!(state.registration, RegistrationState::Registered);
    assert!(state.accounts[0].last_error.is_none());
    lab.engine.shutdown().unwrap();
}

#[test]
fn incoming_answer_without_ack_reports_failure_and_releases_slot_for_next_offer() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("missing-ack");
    lab.offer();
    lab.engine.answer().unwrap();
    lab.response(200, "1 INVITE");
    assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Answering);
    // The real pinned UDP 2xx retransmission deadline is 64 * T1 = 32 seconds.
    // Keep registrations and the timeout's BYE serviced, but deliberately omit ACK.
    let until = Instant::now() + Duration::from_secs(35);
    let state = loop {
        if let Some((packet, _)) = Lab::read(&lab.peer) {
            lab.service(&packet);
        }
        let state = lab.engine.snapshot().unwrap();
        if state.call == CallState::Idle {
            break state;
        }
        assert!(
            Instant::now() < until,
            "unacknowledged answer retained its slot"
        );
    };
    let error = state
        .last_error
        .expect("failed incoming answer must be visible");
    assert_eq!(error.code, ErrorCode::NativeFailure);
    assert_eq!(error.sip_status, None);
    assert!(state.call_id.is_none());
    assert!(state.call_connected_at_ms.is_none());
    let next = lab.incoming();
    assert!(lab.engine.snapshot().unwrap().last_error.is_none());
    lab.end_incoming(&next, 2);
    lab.engine.shutdown().unwrap();
}

#[test]
fn local_hangup_while_answering_releases_slot_without_setup_failure() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("answer-hangup");
    for _ in 0..3 {
        lab.offer();
        lab.engine.answer().unwrap();
        let response = lab.response(200, "1 INVITE");
        assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Answering);
        lab.engine.hangup().unwrap();
        lab.wait(|s| s.call == CallState::Idle);
        lab.request(&response, "ACK", 1, "", "");
        lab.settle();
        let state = lab.engine.snapshot().unwrap();
        assert_eq!(state.call, CallState::Idle);
        assert!(
            state.last_error.is_none(),
            "local hangup reported a setup failure"
        );
    }
    let next = lab.incoming();
    lab.end_incoming(&next, 2);
    lab.engine.shutdown().unwrap();
}

#[test]
fn remote_cancellation_and_local_rejection_leave_subsequent_incoming_calls_usable() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("cancel-reject-repeat");
    for reject in [false, true, false, true] {
        let ringing = lab.offer();
        if reject {
            let token = lab.engine.snapshot().unwrap().call_id.unwrap();
            lab.engine.reject_call(&token).unwrap();
            lab.response(486, "1 INVITE");
        } else {
            let endpoint = lab.peer.local_addr().unwrap();
            let sequence = lab.incoming_sequence;
            lab.send(&format!(
                "CANCEL {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-initial-{sequence};rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: <{}>\r\nCall-ID: {}\r\nCSeq: 1 CANCEL\r\nContent-Length: 0\r\n\r\n",
                lab.target, header(&ringing, "From:"), lab.target, header(&ringing, "Call-ID:"),
            ));
            lab.response(200, "1 CANCEL");
            lab.response(487, "1 INVITE");
        }
        lab.wait(|s| s.call == CallState::Idle);
        assert!(lab.engine.snapshot().unwrap().last_error.is_none());
    }
    let next = lab.incoming();
    lab.end_incoming(&next, 2);
    lab.engine.shutdown().unwrap();
}

#[test]
fn simultaneous_answer_and_hangup_release_the_slot_and_preserve_future_incoming_calls() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("simultaneous-actions");
    for _ in 0..12 {
        lab.offer();
        let token = lab.engine.snapshot().unwrap().call_id.unwrap();
        let (answer, hangup) = {
            // Desktop actions share an exclusive engine/session lock. Release
            // both callers together, without waiting for any native response.
            let engine = std::sync::Mutex::new(&mut lab.engine);
            let barrier = std::sync::Barrier::new(3);
            thread::scope(|scope| {
                let answer = scope.spawn(|| {
                    barrier.wait();
                    engine.lock().unwrap().answer_call(&token)
                });
                let hangup = scope.spawn(|| {
                    barrier.wait();
                    engine.lock().unwrap().hangup_call(&token)
                });
                barrier.wait();
                (answer.join().unwrap(), hangup.join().unwrap())
            })
        };
        if let Err(error) = answer {
            assert_eq!(error.code, ErrorCode::InvalidState);
        }
        hangup.unwrap();
        lab.wait(|s| s.call == CallState::Idle);
        // Complete the winning action's real SIP transaction, then observe any
        // queued losing action rather than immediately clearing it with a call.
        let until = Instant::now() + Duration::from_millis(150);
        while Instant::now() < until {
            if let Some((packet, _)) = Lab::read(&lab.peer) {
                if packet.starts_with("SIP/2.0 ") && header(&packet, "CSeq:") == "1 INVITE" {
                    if packet.starts_with("SIP/2.0 200 ") {
                        lab.request(&packet, "ACK", 1, "", "");
                    } else {
                        lab.send(&format!(
                            "ACK {} SIP/2.0\r\nVia: {}\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 1 ACK\r\nContent-Length: 0\r\n\r\n",
                            lab.target, header(&packet, "Via:"), header(&packet, "From:"),
                            header(&packet, "To:"), header(&packet, "Call-ID:"),
                        ));
                    }
                } else {
                    lab.service(&packet);
                }
            }
        }
        let idle = lab.engine.snapshot().unwrap();
        assert_eq!(idle.call, CallState::Idle);
        assert!(idle.call_id.is_none());
        assert!(
            idle.last_error
                .as_ref()
                .is_none_or(|error| error.code == ErrorCode::InvalidState),
            "competing local actions produced a native failure: {:?}",
            idle.last_error,
        );
        let next = lab.incoming();
        let current = lab.engine.snapshot().unwrap().call_id.unwrap();
        assert_ne!(current, token);
        assert_eq!(
            lab.engine.answer_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(
            lab.engine.hangup_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Active);
        lab.end_incoming(&next, 2);
    }
    lab.engine.shutdown().unwrap();
}

#[test]
fn incoming_call_preempts_each_native_audio_test_and_stale_stop_preserves_call() {
    use phone_core::AudioTestKind;
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    for (kind, label) in [
        (AudioTestKind::Microphone, "audio-preempt-mic"),
        (AudioTestKind::Speaker, "audio-preempt-speaker"),
    ] {
        let mut lab = Lab::new(label);
        let token = lab.engine.start_audio_test(kind, "").unwrap();
        assert!(lab.engine.snapshot().unwrap().audio_test.is_some());
        lab.offer();
        let state = lab.engine.snapshot().unwrap();
        assert!(
            state.audio_test.is_none(),
            "real incoming offer must release test resources first"
        );
        assert!(state.audio_test_error.is_none());
        assert_eq!(
            lab.engine.start_audio_test(kind, "").unwrap_err().code,
            ErrorCode::Busy
        );
        lab.engine.stop_audio_test(&token).unwrap();
        assert_eq!(lab.engine.snapshot().unwrap().call, CallState::Incoming);
        lab.engine.hangup().unwrap();
        lab.wait(|state| state.call == CallState::Idle);
        lab.engine.shutdown().unwrap();
    }
}

#[test]
fn disabled_account_retains_selection_and_requires_ack_before_native_registration() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut lab = Lab::new("account-activation");
    let id = lab.engine.snapshot().unwrap().selected_account_id.unwrap();
    assert!(
        lab.engine.snapshot().unwrap().accounts[0]
            .registered_at_ms
            .is_some()
    );
    lab.engine.set_account_enabled(id, false, None).unwrap();
    let disabled = lab.engine.snapshot().unwrap();
    assert_eq!(disabled.selected_account_id, Some(id));
    assert!(!disabled.accounts[0].enabled);
    assert!(disabled.accounts[0].registered_at_ms.is_none());
    assert_eq!(
        lab.engine.dial("4321").unwrap_err().code,
        ErrorCode::NotRegistered
    );
    lab.settle();
    let config = AccountConfig {
        registrar: lab.peer.local_addr().unwrap().to_string(),
        username: "events-test".into(),
        password: "synthetic-only".into(),
        audio_mode: AudioMode::TestTone,
    };
    let updated_id = lab
        .engine
        .save_account(Some(id), "Edited disabled account", &config)
        .unwrap();
    lab.engine.wait_account_update(updated_id).unwrap();
    assert!(
        !lab.engine.snapshot().unwrap().accounts[0].enabled,
        "editing a disabled account must not register it"
    );
    assert_eq!(lab.engine.snapshot().unwrap().selected_account_id, Some(id));
    lab.engine
        .set_account_enabled(id, true, Some(&config))
        .unwrap();
    let registration = lab.receive("enabled registration", |packet| {
        packet.starts_with("REGISTER ")
    });
    lab.reply(&registration, "200 OK", "", "");
    lab.wait(|state| state.registration == RegistrationState::Registered);
    lab.offer();
    assert_eq!(
        lab.engine
            .set_account_enabled(id, false, None)
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    assert!(lab.engine.snapshot().unwrap().accounts[0].enabled);
    lab.engine.hangup().unwrap();
    lab.wait(|state| state.call == CallState::Idle);
    lab.engine.shutdown().unwrap();
}

#[test]
fn controls_hold_is_confirmed_and_refusal_rolls_back() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("local-controls");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
    let pending = lab.engine.snapshot().unwrap();
    assert!(!pending.calls[0].local_held);
    assert!(!pending.calls[0].remote_held);
    assert!(pending.calls[0].hold_pending);
    assert_eq!(
        lab.engine.transfer_call(&id, "123").unwrap_err().code,
        ErrorCode::Busy
    );
    lab.reply(&offer, "488 Not Acceptable Here", "", "");
    lab.wait(|s| !s.calls[0].hold_pending);
    assert!(!lab.engine.snapshot().unwrap().calls[0].local_held);
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold retry", |p| p.starts_with("INVITE "));
    lab.reply(&offer, "200 OK", "", &sdp(30, "recvonly"));
    let held = lab.wait(|s| s.calls[0].local_held);
    assert!(!held.calls[0].remote_held);
    assert_eq!(
        lab.engine
            .set_call_muted("old-generation", false)
            .unwrap_err()
            .code,
        ErrorCode::InvalidState
    );
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_consultation_is_two_dialogs_and_cancel_returns_to_original() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("consultation");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&id, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation invite", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(
        &invite,
        "200 OK",
        "Supported: replaces\r\n",
        &sdp(1, "sendrecv"),
    );
    lab.wait(|s| s.calls.len() == 2 && s.call == CallState::Active);
    let consultation = lab.engine.snapshot().unwrap().call_id.unwrap();
    assert_ne!(id, consultation);
    assert_eq!(
        lab.engine
            .start_consultation(&consultation, "456")
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
    assert_eq!(
        lab.engine.complete_transfer(&id).unwrap_err().code,
        ErrorCode::InvalidState
    );
    assert_eq!(
        lab.engine.cancel_consultation(&id).unwrap_err().code,
        ErrorCode::InvalidState
    );
    lab.engine.cancel_consultation(&consultation).unwrap();
    lab.settle();
    let original = lab.wait(|s| {
        s.calls.len() == 1 && s.call_id.as_deref() == Some(&id) && !s.calls[0].hold_pending
    });
    assert!(!original.calls[0].local_held);
    lab.engine.start_consultation(&id, "456").unwrap();
    let second_hold = lab.receive("second original hold", |p| p.starts_with("INVITE "));
    lab.reply(&second_hold, "200 OK", "", &sdp(32, "recvonly"));
    let second_invite = lab.receive("replacement consultation INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(&second_invite, "200 OK", "", &sdp(1, "sendrecv"));
    let replacement = lab
        .wait(|s| s.calls.len() == 2 && s.call == CallState::Active)
        .call_id
        .unwrap();
    assert_ne!(replacement, consultation);
    assert_eq!(
        lab.engine
            .cancel_consultation(&consultation)
            .unwrap_err()
            .code,
        ErrorCode::InvalidState
    );
    assert_eq!(
        lab.engine
            .complete_transfer(&consultation)
            .unwrap_err()
            .code,
        ErrorCode::InvalidState
    );
    assert_eq!(lab.engine.snapshot().unwrap().calls.len(), 2);
    lab.engine.cancel_consultation(&replacement).unwrap();
    lab.settle();
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_glare_rolls_back_and_local_remote_hold_are_independent() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("control-glare");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
    lab.reply(&offer, "491 Request Pending", "", "");
    let state = lab.wait(|s| !s.calls[0].hold_pending && s.calls[0].last_error.is_some());
    assert!(!state.calls[0].local_held);
    assert_eq!(
        state.calls[0].last_error.as_ref().unwrap().sip_status,
        Some(491)
    );
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold offer after glare", |p| p.starts_with("INVITE "));
    lab.reply(&offer, "200 OK", "", &sdp(30, "recvonly"));
    lab.wait(|s| s.calls[0].local_held);
    lab.request(
        &dialog,
        "INVITE",
        2,
        "Content-Type: application/sdp\r\n",
        &sdp(31, "sendonly"),
    );
    let response = lab.response(200, "2 INVITE");
    lab.request(&response, "ACK", 2, "", "");
    let state = lab.wait(|s| s.calls[0].remote_held);
    assert!(state.calls[0].local_held);
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_transfer_waits_for_final_notify_and_failure_keeps_original() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("blind-transfer");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: active;expires=60\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 2, &event, "SIP/2.0 100 Trying\r\n");
    lab.response(200, "2 NOTIFY");
    assert_eq!(
        lab.engine.snapshot().unwrap().calls[0].transfer,
        phone_core::TransferState::Pending
    );
    lab.request(&dialog, "NOTIFY", 3, &event, "SIP/2.0 486 Busy Here\r\n");
    lab.response(200, "3 NOTIFY");
    let state = lab.wait(|s| s.calls[0].transfer == phone_core::TransferState::Failed);
    assert_eq!(state.call_id.as_deref(), Some(id.as_str()));
    assert_eq!(state.call, CallState::Active);
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_transfer_timeout_ignores_late_success_and_original_remains_usable() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("transfer-timeout");
    let dialog = lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.transfer_call(&id, "123").unwrap();
    let refer = lab.receive("REFER", |p| p.starts_with("REFER "));
    lab.reply(&refer, "202 Accepted", "", "");
    let until = Instant::now() + Duration::from_secs(16);
    while Instant::now() < until {
        lab.settle();
    }
    assert_eq!(
        lab.engine.snapshot().unwrap().calls[0].transfer,
        phone_core::TransferState::Failed
    );
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&dialog, "NOTIFY", 2, &event, "SIP/2.0 200 OK\r\n");
    lab.settle();
    assert_eq!(
        lab.engine.snapshot().unwrap().call_id.as_deref(),
        Some(id.as_str())
    );
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_attended_transfer_sends_replaces_and_only_final_notify_closes_both() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("attended-transfer");
    let original_dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&original, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation invite", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(
        &invite,
        "200 OK",
        "Supported: replaces\r\n",
        &sdp(1, "sendrecv"),
    );
    let state = lab.wait(|s| s.calls.len() == 2 && s.call == CallState::Active);
    let consultation = state.call_id.unwrap();
    assert_eq!(state.calls[1].destination.as_deref(), Some("123"));
    lab.engine.switch_call(&original).unwrap();
    let consultation_hold = lab.receive("consultation hold", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&invite, "Call-ID:")
    });
    lab.reply(&consultation_hold, "200 OK", "", &sdp(2, "recvonly"));
    let original_resume = lab.receive("original resume", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&hold, "Call-ID:")
    });
    lab.reply(&original_resume, "200 OK", "", &sdp(31, "sendrecv"));
    lab.wait(|s| s.call_id.as_deref() == Some(original.as_str()) && !s.calls[0].hold_pending);
    lab.engine.complete_transfer(&consultation).unwrap();
    let refer = lab.receive("attended REFER", |p| p.starts_with("REFER "));
    assert!(header(&refer, "Refer-To:").contains("Replaces="));
    assert!(header(&refer, "Refer-To:").contains(header(&invite, "Call-ID:")));
    assert_eq!(header(&refer, "Call-ID:"), header(&hold, "Call-ID:"));
    lab.reply(&refer, "202 Accepted", "", "");
    assert_eq!(lab.engine.snapshot().unwrap().calls.len(), 2);
    let event = format!(
        "Event: refer;id={}\r\nSubscription-State: terminated;reason=noresource\r\nContent-Type: message/sipfrag\r\n",
        header(&refer, "CSeq:").split_whitespace().next().unwrap()
    );
    lab.request(&original_dialog, "NOTIFY", 2, &event, "SIP/2.0 200 OK\r\n");
    lab.response(200, "2 NOTIFY");
    lab.settle();
    let completed = lab.wait(|s| s.call == CallState::Idle && s.calls.is_empty());
    assert!(
        completed
            .call_events
            .iter()
            .any(|event| matches!(event.event, CallEventKind::TransferSucceeded))
    );
    assert_eq!(
        lab.engine.set_call_muted(&original, true).unwrap_err().code,
        ErrorCode::InvalidState
    );
}

#[test]
fn controls_hold_deadline_acks_and_reconciles_late_answer() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("hold-deadline");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold without response", |p| p.starts_with("INVITE "));
    let until = Instant::now() + Duration::from_secs(11);
    while Instant::now() < until {
        let _ = Lab::read(&lab.peer);
    }
    let state = lab.engine.snapshot().unwrap();
    assert!(state.calls[0].hold_pending); // Local rollback is not proof of remote rollback.
    assert!(!state.calls[0].local_held);
    assert_eq!(state.call, CallState::Active);
    assert!(state.calls[0].last_error.is_some());
    lab.engine.set_call_muted(&id, true).unwrap(); // Mute remains available during recovery.
    lab.wait(|s| s.calls[0].muted);
    lab.reply(&offer, "200 OK", "", &sdp(30, "recvonly"));
    let ack_cseq = header(&offer, "CSeq:").replace("INVITE", "ACK");
    let ack = lab.receive("ACK of late hold answer", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == ack_cseq
    });
    let recovery = lab.receive("compensating re-INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&offer, "CSeq:")
    });
    assert!(recovery.contains("a=sendrecv"));
    assert!(lab.engine.snapshot().unwrap().calls[0].hold_pending);
    lab.reply(&recovery, "200 OK", "", &sdp(31, "sendrecv"));
    lab.wait(|s| !s.calls[0].hold_pending && !s.calls[0].local_held);
    lab.reply(&offer, "200 OK", "", &sdp(30, "recvonly"));
    let repeated_ack = lab.receive("cached ACK for repeated late answer", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == ack_cseq
    });
    assert_eq!(ack, repeated_ack);
    lab.engine.set_call_held(&id, true).unwrap();
    let retry = lab.receive("usable next re-INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&recovery, "CSeq:")
    });
    lab.reply(&retry, "200 OK", "", &sdp(32, "recvonly"));
    lab.wait(|s| s.calls[0].local_held);
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_remote_original_end_keeps_ringing_consultation_usable() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("consultation-cleanup");
    let original_dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&original, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation invite", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(&invite, "180 Ringing", "", "");
    let state = lab.wait(|s| s.calls.len() == 2 && s.call == CallState::Ringing);
    let consultation = state.call_id.unwrap();
    lab.request(&original_dialog, "BYE", 2, "", "");
    lab.response(200, "2 BYE");
    let state = lab.wait(|s| s.calls.len() == 1);
    assert_eq!(state.call_id.as_deref(), Some(consultation.as_str()));
    assert!(!state.calls[0].consultation);
    lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
    lab.wait(|s| s.call == CallState::Active);
    lab.engine.set_call_muted(&consultation, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    lab.engine.hangup_call(&consultation).unwrap();
    lab.settle();
}

#[test]
fn controls_mute_suppresses_native_test_tone_and_unmute_restores_it() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("mute-rtp");
    let rtp = UdpSocket::bind("127.0.0.1:0").unwrap();
    rtp.set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let invite = lab.outgoing();
    let answer = sdp(1, "sendrecv").replace(
        "audio 9000",
        &format!("audio {}", rtp.local_addr().unwrap().port()),
    );
    lab.reply(&invite, "200 OK", "", &answer);
    let state = lab.wait(|s| s.call == CallState::Active);
    let id = state.call_id.unwrap();
    let samples = |socket: &UdpSocket| {
        let until = Instant::now() + Duration::from_millis(180);
        let mut payload = Vec::new();
        while Instant::now() < until {
            let mut buffer = [0; 2048];
            let len = socket.recv(&mut buffer).unwrap();
            if len > 12 && buffer[1] & 127 == 0 {
                payload.extend_from_slice(&buffer[12..len]);
            }
        }
        payload
    };
    let audible = samples(&rtp);
    assert!(audible.iter().any(|byte| ![0xff, 0x7f].contains(byte)));
    lab.engine.set_call_muted(&id, true).unwrap();
    lab.wait(|s| s.calls[0].muted);
    let _ = samples(&rtp); // Drain queued frames and the current native 20 ms frame.
    let muted = samples(&rtp);
    assert!(!muted.is_empty());
    assert!(muted.iter().all(|byte| [0xff, 0x7f].contains(byte)));
    lab.engine.set_call_muted(&id, false).unwrap();
    lab.wait(|s| !s.calls[0].muted);
    let _ = samples(&rtp);
    let resumed = samples(&rtp);
    assert!(resumed.iter().any(|byte| ![0xff, 0x7f].contains(byte)));
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_cancel_consultation_while_ringing_resumes_original() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("cancel-ringing");
    lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&original, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation invite", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(&invite, "180 Ringing", "", "");
    let state = lab.wait(|s| s.calls.len() == 2 && s.call == CallState::Ringing);
    let consultation = state.call_id.unwrap();
    lab.engine.cancel_consultation(&consultation).unwrap();
    let cancel = lab.receive("consultation CANCEL", |p| p.starts_with("CANCEL "));
    lab.reply(&cancel, "200 OK", "", "");
    lab.reply(&invite, "487 Request Terminated", "", "");
    lab.settle();
    lab.wait(|s| {
        s.calls.len() == 1
            && s.call_id.as_deref() == Some(original.as_str())
            && !s.calls[0].local_held
            && !s.calls[0].hold_pending
    });
    assert_eq!(
        lab.engine.hangup_call(&consultation).unwrap_err().code,
        ErrorCode::InvalidState
    );
    let returned = lab.engine.snapshot().unwrap();
    assert!(returned.last_error.is_none());
    assert!(returned.last_call_failure.is_none());
    lab.engine.hangup_call(&original).unwrap();
    lab.settle();
}

#[test]
fn controls_hold_without_sdp_answer_rolls_back_and_allows_retry() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    for body in ["", "invalid synthetic SDP"] {
        let mut lab = Lab::new("hold-invalid-sdp");
        lab.incoming();
        let id = lab.engine.snapshot().unwrap().call_id.unwrap();
        lab.engine.set_call_held(&id, true).unwrap();
        let offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
        lab.reply(&offer, "200 OK", "", body);
        lab.wait(|s| !s.calls[0].hold_pending && s.calls[0].last_error.is_some());
        assert!(!lab.engine.snapshot().unwrap().calls[0].local_held);
        lab.receive("ACK of invalid hold answer", |p| p.starts_with("ACK "));
        lab.engine.set_call_held(&id, true).unwrap();
        let retry = lab.receive("hold retry after missing SDP", |p| {
            p.starts_with("INVITE ") && header(p, "CSeq:") != header(&offer, "CSeq:")
        });
        lab.reply(&retry, "200 OK", "", &sdp(30, "recvonly"));
        lab.wait(|s| s.calls[0].local_held);
        lab.engine.hangup_call(&id).unwrap();
        lab.settle();
    }
}

#[test]
fn controls_provisional_hold_timeout_keeps_transaction_ack_and_retry() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("hold-cancel-transaction");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
    lab.reply(&offer, "180 Ringing", "", "");
    let cancel = lab.receive_for(Duration::from_secs(12), "CANCEL of expired hold", |p| {
        p.starts_with("CANCEL ")
    });
    lab.reply(&cancel, "200 OK", "", "");
    lab.reply(&offer, "487 Request Terminated", "", "");
    let ack_cseq = header(&offer, "CSeq:").replace("INVITE", "ACK");
    let ack = lab.receive("transaction ACK of rejected hold", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == ack_cseq
    });
    assert_eq!(header(&ack, "Via:"), header(&offer, "Via:"));
    lab.wait(|s| !s.calls[0].hold_pending);
    lab.engine.set_call_held(&id, true).unwrap();
    let retry = lab.receive("retry after CANCEL and 487", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&offer, "CSeq:")
    });
    lab.reply(&retry, "200 OK", "", &sdp(30, "recvonly"));
    lab.wait(|s| s.calls[0].local_held);
    lab.engine.hangup_call(&id).unwrap();
    lab.settle();
}

#[test]
fn controls_unresolved_hold_recovery_ends_uncertain_dialog_with_explicit_error() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("hold-recovery-deadline");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let _offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
    let until = Instant::now() + Duration::from_secs(23);
    while Instant::now() < until {
        let _ = Lab::read(&lab.peer);
    }
    let state = lab.engine.snapshot().unwrap();
    assert_eq!(state.call, CallState::Idle);
    assert!(state.calls.is_empty());
    assert!(
        state
            .last_error
            .as_ref()
            .unwrap()
            .message
            .contains("uncertain call was ended")
    );
    assert_eq!(state.last_error.as_ref().unwrap().sip_status, None);
}

#[test]
fn controls_consultation_failure_restores_original_intent_and_preserves_error() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    for was_held in [false, true] {
        let mut lab = Lab::new("consultation-failure");
        lab.incoming();
        let original = lab.engine.snapshot().unwrap().call_id.unwrap();
        if was_held {
            lab.engine.set_call_held(&original, true).unwrap();
            let hold = lab.receive("explicit original hold", |p| p.starts_with("INVITE "));
            lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
            lab.wait(|s| s.calls[0].local_held);
        }
        lab.engine.start_consultation(&original, "123").unwrap();
        if !was_held {
            let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
            lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
        }
        let invite = lab.receive("consultation INVITE without redundant hold", |p| {
            p.starts_with("INVITE ")
        });
        lab.reply(&invite, "486 Busy Here", "", "");
        lab.settle();
        let state = lab.wait(|s| {
            s.calls.len() == 1 && !s.calls[0].hold_pending && s.calls[0].local_held == was_held
        });
        assert_eq!(state.call_id.as_deref(), Some(original.as_str()));
        assert_eq!(state.last_error.as_ref().unwrap().code, ErrorCode::SipBusy);
        assert_eq!(state.last_error.as_ref().unwrap().sip_status, Some(486));
        lab.engine.set_call_muted(&original, true).unwrap();
        let state = lab.wait(|s| s.calls[0].muted);
        assert!(state.last_error.is_none());
        lab.engine.hangup_call(&original).unwrap();
        lab.settle();
    }
}

#[test]
fn controls_repeated_success_response_replays_complete_wire_ack() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("ack-cache");
    let invite = lab.outgoing();
    let body = sdp(1, "sendrecv");
    lab.reply(&invite, "200 OK", "", &body);
    let ack_cseq = header(&invite, "CSeq:").replace("INVITE", "ACK");
    let first = lab.receive("initial ACK", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == ack_cseq
    });
    lab.reply(&invite, "200 OK", "", &body);
    let repeated = lab.receive("cached complete ACK", |p| {
        p.starts_with("ACK ") && header(p, "CSeq:") == ack_cseq
    });
    assert_eq!(first, repeated);
    lab.engine.hangup().unwrap();
    lab.settle();
}

#[test]
fn controls_refused_compensation_ends_uncertain_dialog_with_sip_error() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("hold-recovery-refused");
    lab.incoming();
    let id = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.set_call_held(&id, true).unwrap();
    let offer = lab.receive("hold offer", |p| p.starts_with("INVITE "));
    let until = Instant::now() + Duration::from_secs(11);
    while Instant::now() < until {
        let _ = Lab::read(&lab.peer);
    }
    assert!(lab.engine.snapshot().unwrap().calls[0].hold_pending);
    lab.reply(&offer, "200 OK", "", &sdp(30, "recvonly"));
    let recovery = lab.receive("compensating re-INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "CSeq:") != header(&offer, "CSeq:")
    });
    assert!(recovery.contains("a=sendrecv"));
    lab.reply(&recovery, "488 Not Acceptable Here", "", "");
    lab.settle();
    let state = lab.wait(|s| s.call == CallState::Idle);
    let error = state.last_error.as_ref().unwrap();
    assert_eq!(error.code, ErrorCode::SipNotAcceptable);
    assert_eq!(error.sip_status, Some(488));
    assert!(error.message.contains("uncertain call was ended"));
}

#[test]
fn consultation_native_setup_failure_is_retained_on_original_without_sip_status() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("consultation-native-failure");
    lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&original, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    let child = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.reply(&invite, "183 Session Progress", "", "v=invalid\r\n");
    lab.settle();
    let state = lab.wait(|s| {
        s.calls.len() == 1
            && s.call_id.as_deref() == Some(original.as_str())
            && !s.calls[0].hold_pending
            && !s.calls[0].local_held
    });
    let error = state
        .last_error
        .as_ref()
        .expect("native child failure must survive return/resume");
    assert_eq!(error.code, ErrorCode::NativeFailure);
    assert_eq!(error.sip_status, None);
    let terminal = state.last_call_failure.as_ref().unwrap();
    assert_eq!(terminal.call_id, child);
    assert_eq!(terminal.account_id, state.calls[0].account_id);
    assert_eq!(terminal.error.code, ErrorCode::NativeFailure);
    assert_eq!(terminal.error.sip_status, None);
    lab.engine.hangup_call(&original).unwrap();
    lab.settle();
}

#[test]
fn consultation_return_and_switch_queue_resume_until_offerless_ack() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    for switching in [false, true] {
        let mut lab = Lab::new("offerless-return");
        let original_dialog = lab.incoming();
        let original = lab.engine.snapshot().unwrap().call_id.unwrap();
        lab.engine.start_consultation(&original, "123").unwrap();
        let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
        lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
        let invite = lab.receive("consultation INVITE", |p| {
            p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
        });
        lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
        let child = lab
            .wait(|s| s.calls.len() == 2 && s.call == CallState::Active)
            .call_id
            .unwrap();
        lab.request(&original_dialog, "INVITE", 2, "", "");
        let offer = lab.response(200, "2 INVITE");
        if switching {
            lab.engine.switch_call(&original).unwrap();
            let child_hold = lab.receive("consultation hold before switch", |p| {
                p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&invite, "Call-ID:")
            });
            lab.reply(&child_hold, "200 OK", "", &sdp(2, "recvonly"));
        } else {
            lab.engine.cancel_consultation(&child).unwrap();
        }
        lab.wait(|s| s.call_id.as_deref() == Some(original.as_str()) && s.calls[0].hold_pending);
        assert!(lab.engine.snapshot().unwrap().calls[0].local_held);
        lab.request(
            &offer,
            "ACK",
            2,
            "Content-Type: application/sdp\r\n",
            &sdp(40, "recvonly"),
        );
        let resume = lab.receive("deferred original resume", |p| {
            p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&hold, "Call-ID:")
        });
        assert!(resume.contains("a=sendrecv"));
        lab.reply(&resume, "200 OK", "", &sdp(41, "sendrecv"));
        lab.wait(|s| !s.calls[0].hold_pending && !s.calls[0].local_held);
        if switching {
            lab.engine.cancel_consultation(&child).unwrap();
        }
        lab.settle();
        lab.engine.hangup_call(&original).unwrap();
        lab.settle();
    }
}

#[test]
fn consultation_queued_resume_has_bounded_failure_and_allows_later_retry() {
    let _guard = crate::NATIVE_TEST_LOCK.lock().unwrap();
    let mut lab = Lab::new("offerless-return-deadline");
    let original_dialog = lab.incoming();
    let original = lab.engine.snapshot().unwrap().call_id.unwrap();
    lab.engine.start_consultation(&original, "123").unwrap();
    let hold = lab.receive("original hold", |p| p.starts_with("INVITE "));
    lab.reply(&hold, "200 OK", "", &sdp(30, "recvonly"));
    let invite = lab.receive("consultation INVITE", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") != header(&hold, "Call-ID:")
    });
    lab.reply(&invite, "200 OK", "", &sdp(1, "sendrecv"));
    let child = lab
        .wait(|s| s.calls.len() == 2 && s.call == CallState::Active)
        .call_id
        .unwrap();
    lab.request(&original_dialog, "INVITE", 2, "", "");
    let offer = lab.response(200, "2 INVITE");
    lab.engine.cancel_consultation(&child).unwrap();
    lab.wait(|s| s.calls.len() == 1 && s.calls[0].hold_pending);
    let until = Instant::now() + Duration::from_secs(11);
    while Instant::now() < until {
        let _ = Lab::read(&lab.peer);
    }
    let state = lab.engine.snapshot().unwrap();
    assert!(state.calls[0].local_held);
    assert!(!state.calls[0].hold_pending);
    assert_eq!(
        state.last_error.as_ref().unwrap().code,
        ErrorCode::NativeFailure
    );
    assert!(state.last_call_failure.is_none());
    lab.request(
        &offer,
        "ACK",
        2,
        "Content-Type: application/sdp\r\n",
        &sdp(40, "recvonly"),
    );
    lab.engine.set_call_held(&original, false).unwrap();
    let resume = lab.receive("explicit resume after remote ACK", |p| {
        p.starts_with("INVITE ") && header(p, "Call-ID:") == header(&hold, "Call-ID:")
    });
    lab.reply(&resume, "200 OK", "", &sdp(41, "sendrecv"));
    lab.wait(|s| !s.calls[0].hold_pending && !s.calls[0].local_held);
    lab.engine.hangup_call(&original).unwrap();
    lab.settle();
}
