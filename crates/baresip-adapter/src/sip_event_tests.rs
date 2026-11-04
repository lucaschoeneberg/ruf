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
}

impl Lab {
    fn new(label: &str) -> Self {
        Self::registration_response(label, "200 OK")
    }

    fn registration_response(label: &str, status: &str) -> Self {
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
        };
        lab.reply(&registration, status, "", "");
        let expected = if status.starts_with("200 ") {
            RegistrationState::Registered
        } else {
            RegistrationState::Failed
        };
        lab.wait(|s| s.registration == expected);
        lab
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
        let until = Instant::now() + TIMEOUT;
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
                "native event state timed out: call={:?}, events={:?}, error={:?}",
                state.call,
                state.call_events,
                state.last_error
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn incoming(&mut self) -> String {
        let endpoint = self.peer.local_addr().unwrap();
        let body = sdp(1, "sendrecv");
        self.send(&format!(
            "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-initial;rport\r\nMax-Forwards: 70\r\nFrom: \"Event Caller\" <sip:12345@lab>;tag=caller\r\nTo: <{}>\r\nCall-ID: synthetic-events-incoming\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{endpoint}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{body}",
            self.target, self.target, body.len(),
        ));
        self.wait(|s| s.call == CallState::Incoming);
        self.receive("incoming ringing", |p| p.starts_with("SIP/2.0 180"));
        self.engine.answer().unwrap();
        let response = self.response(200, "1 INVITE");
        self.request(&response, "ACK", 1, "", "");
        self.wait(|s| s.call == CallState::Active);
        self.settle();
        response
    }

    fn request(&self, dialog: &str, method: &str, sequence: u32, extra: &str, body: &str) {
        let endpoint = self.peer.local_addr().unwrap();
        self.send(&format!(
            "{method} {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent-{method}-{sequence};rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {sequence} {method}\r\nContact: <sip:caller@{endpoint}>\r\n{extra}Content-Length: {}\r\n\r\n{body}",
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
