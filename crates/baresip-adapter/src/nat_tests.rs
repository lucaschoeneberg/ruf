//! Synthetic loopback signaling only: no real account, DNS or external network.
use crate::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, RegistrationState,
};
use std::{
    io::ErrorKind,
    net::{SocketAddr, UdpSocket},
    thread,
    time::{Duration, Instant},
};

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
fn receive(socket: &UdpSocket) -> (String, SocketAddr) {
    let mut data = [0; 8192];
    let (size, source) = socket.recv_from(&mut data).expect("synthetic SIP datagram");
    (String::from_utf8(data[..size].to_vec()).unwrap(), source)
}
fn wait(engine: &Engine, predicate: impl Fn(&EngineSnapshot) -> bool) {
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        let state = engine.snapshot().unwrap();
        if predicate(&state) {
            return;
        }
        assert!(
            Instant::now() < until,
            "synthetic call state timed out: {state:?}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
struct Lab {
    registrar: UdpSocket,
    incoming: UdpSocket,
    mapped: UdpSocket,
    local: SocketAddr,
    target: String,
}
impl Lab {
    fn request_in_dialog(
        &self,
        response: &str,
        method: &str,
        sequence: u32,
        headers: &str,
        body: &str,
    ) {
        let endpoint = self.incoming.local_addr().unwrap();
        let packet = format!(
            "{method} {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKevent{sequence};rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {sequence} {method}\r\nContact: <sip:caller@{endpoint}>\r\n{headers}Content-Length: {}\r\n\r\n{body}",
            uri(header(response, "Contact:")),
            header(response, "From:"),
            header(response, "To:"),
            header(response, "Call-ID:"),
            body.len(),
        );
        self.incoming
            .send_to(packet.as_bytes(), self.local)
            .unwrap();
    }
    fn socket() -> UdpSocket {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
    }
    fn register(engine: &mut Engine) -> Self {
        Self::register_with_expiry(engine, 60)
    }
    fn register_with_expiry(engine: &mut Engine, expires: u32) -> Self {
        let registrar = Self::socket();
        let incoming = Self::socket();
        let mapped = Self::socket();
        engine
            .configure_account(&AccountConfig {
                transport: Default::default(),
                media_security: Default::default(),
                auth_username: None,
                outbound_proxy: None,
                registrar: registrar.local_addr().unwrap().to_string(),
                username: "nat-test".into(),
                password: "synthetic-only".into(),
                audio_mode: AudioMode::TestTone,
            })
            .unwrap();
        let mut local = registrar.local_addr().unwrap();
        let mut target = String::new();
        // Initial registration discovers a different public port; second removes
        // the stale binding and confirms the mapped Contact on the wire.
        for _ in 0..2 {
            let (packet, source) = receive(&registrar);
            local = source;
            let contact = packet
                .lines()
                .filter_map(|line| line.strip_prefix("Contact: "))
                .find(|value| !value.contains("expires=0"))
                .unwrap();
            target = uri(contact).to_owned();
            let response = format!(
                "SIP/2.0 200 OK\r\nVia: {};received=127.0.0.1;rport={}\r\nFrom: {}\r\nTo: {};tag=registrar\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <{target}>;expires={expires}\r\nContent-Length: 0\r\n\r\n",
                header(&packet, "Via:"),
                mapped.local_addr().unwrap().port(),
                header(&packet, "From:"),
                header(&packet, "To:"),
                header(&packet, "Call-ID:"),
                header(&packet, "CSeq:")
            );
            registrar.send_to(response.as_bytes(), local).unwrap();
        }
        wait(engine, |s| s.registration == RegistrationState::Registered);
        assert!(target.ends_with(&format!(":{}", mapped.local_addr().unwrap().port())));
        Self {
            registrar,
            incoming,
            mapped,
            local,
            target,
        }
    }
    fn invite(&self, sequence: u32) -> String {
        let long_caller = format!("\"{}\" <sip:{}@lab>", "é".repeat(80), "a".repeat(100));
        let caller = match sequence {
            2 => "\"Anonymous\" <sip:anonymous@anonymous.invalid>",
            5 => "\"SIP User\" <sip:alice.sip_42:private-password@lab>",
            6 => "\"Lab Caller\" <sip:unsafe%0Auser@lab>",
            7 => "\"Telefon\" <tel:+4930555>",
            8 => "\"Secret Name\" <sip:%61nonymous@lab>",
            9 => &long_caller,
            _ => "\"Lab Caller\" <sip:+493012345@lab>",
        };
        let privacy = if sequence == 3 {
            "Privacy: id\r\nP-Asserted-Identity: <sip:secret@lab>\r\n"
        } else {
            ""
        };
        let endpoint = self.incoming.local_addr().unwrap();
        let sdp = "v=0\r\no=lab 1 1 IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n";
        let packet = format!(
            "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {};branch=z9hG4bKsynthetic{};rport\r\nMax-Forwards: 70\r\nFrom: {};tag=caller{}\r\n{}To: <{}>\r\nCall-ID: synthetic-{}\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
            self.target,
            endpoint,
            sequence,
            caller,
            sequence,
            privacy,
            self.target,
            sequence,
            endpoint,
            sdp.len(),
            sdp
        );
        self.incoming
            .send_to(packet.as_bytes(), self.local)
            .unwrap();
        packet
    }
    fn response(&self, status: &str) -> String {
        loop {
            let (packet, _) = receive(&self.incoming);
            if packet.starts_with(status) {
                return packet;
            }
        }
    }
    fn contact(&self, response: &str) -> SocketAddr {
        let contact = uri(header(response, "Contact:"));
        let destination: SocketAddr = contact
            .rsplit_once('@')
            .unwrap()
            .1
            .parse()
            .expect("numeric Contact");
        assert_eq!(
            destination,
            self.mapped.local_addr().unwrap(),
            "incoming response must advertise the validated mapped port even when PBX source port changes"
        );
        destination
    }
    fn in_dialog(&self, response: &str, method: &str) {
        let contact = uri(header(response, "Contact:"));
        let destination = self.contact(response);
        let packet = format!(
            "{} {} SIP/2.0\r\nVia: SIP/2.0/UDP {};branch=z9hG4bK{}{};rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {} {}\r\nContent-Length: 0\r\n\r\n",
            method,
            contact,
            self.incoming.local_addr().unwrap(),
            method,
            header(response, "Call-ID:"),
            header(response, "From:"),
            header(response, "To:"),
            header(response, "Call-ID:"),
            if method == "ACK" { 1 } else { 2 },
            method
        );
        // The simulated outside address only forwards packets addressed to the
        // advertised mapped port, as a real UDP NAT binding would.
        self.incoming
            .send_to(packet.as_bytes(), destination)
            .unwrap();
        let (forwarded, _) = receive(&self.mapped);
        self.incoming
            .send_to(forwarded.as_bytes(), self.local)
            .unwrap();
    }

    fn register_response(&mut self, packet: &str) {
        let contact = packet
            .lines()
            .filter_map(|line| line.strip_prefix("Contact: "))
            .find(|value| !value.contains("expires=0"))
            .expect("live synthetic registration binding");
        self.target = uri(contact).to_owned();
        let response = format!(
            "SIP/2.0 200 OK\r\nVia: {};received=127.0.0.1;rport={}\r\nFrom: {}\r\nTo: {};tag=registrar\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <{}>;expires=60\r\nContent-Length: 0\r\n\r\n",
            header(packet, "Via:"),
            self.mapped.local_addr().unwrap().port(),
            header(packet, "From:"),
            header(packet, "To:"),
            header(packet, "Call-ID:"),
            header(packet, "CSeq:"),
            self.target
        );
        self.registrar
            .send_to(response.as_bytes(), self.local)
            .unwrap();
    }
}

fn read_if_available(socket: &UdpSocket) -> Option<String> {
    let mut data = [0; 8192];
    match socket.recv_from(&mut data) {
        Ok((size, _)) => Some(String::from_utf8(data[..size].to_vec()).unwrap()),
        Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => None,
        Err(error) => panic!("synthetic SIP datagram failed: {error}"),
    }
}

#[test]
fn signaling_blackout_times_out_then_reregisters_and_accepts_a_new_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut engine = Engine::start(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join("sip-blackout-recovery.wav")),
    )
    .unwrap();
    // The registrar's short lifetime causes a real refresh during the blackout.
    // Keep every socket bound: dropping datagrams, rather than closing a port,
    // exercises SIP timers instead of immediate ICMP connection refusal.
    let mut lab = Lab::register_with_expiry(&mut engine, 2);
    lab.registrar
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    engine.dial("4321").unwrap();
    let until = Instant::now() + Duration::from_secs(45);
    let mut dropped_invites = 0;
    let mut dropped_registers = 0;
    loop {
        if let Some(packet) = read_if_available(&lab.registrar) {
            dropped_invites += usize::from(packet.starts_with("INVITE "));
            dropped_registers += usize::from(packet.starts_with("REGISTER "));
        }
        let state = engine.snapshot().unwrap();
        if state.call == CallState::Idle && state.registration == RegistrationState::Failed {
            assert!(state.call_id.is_none());
            assert!(state.call_account_id.is_none());
            assert!(state.last_error.is_some());
            assert!(state.accounts[0].last_error.is_some());
            break;
        }
        assert!(
            Instant::now() < until,
            "blackout did not time out: {state:?}"
        );
    }
    assert!(dropped_invites >= 2, "INVITE retransmissions were not lost");
    assert!(
        dropped_registers >= 2,
        "REGISTER retransmissions were not lost"
    );

    // Restore the same transport and let libre's normal randomized retry run;
    // no save-account, reconnect command or engine recreation is involved.
    let until = Instant::now() + Duration::from_secs(75);
    loop {
        if let Some(packet) = read_if_available(&lab.registrar)
            && packet.starts_with("REGISTER ")
        {
            lab.register_response(&packet);
        }
        let state = engine.snapshot().unwrap();
        if state.registration == RegistrationState::Registered {
            assert!(state.accounts[0].last_error.is_none());
            break;
        }
        assert!(
            Instant::now() < until,
            "registration did not recover: {state:?}"
        );
    }
    lab.invite(70);
    wait(&engine, |s| s.call == CallState::Incoming);
    assert!(engine.snapshot().unwrap().last_error.is_none());
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    lab.in_dialog(&response, "BYE");
    lab.response("SIP/2.0 200");
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(engine.snapshot().unwrap().last_error.is_none());
    engine.shutdown().unwrap();
}

#[test]
fn lost_bye_response_releases_local_call_and_keeps_the_next_incoming_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut engine = Engine::start(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join("sip-lost-bye-recovery.wav")),
    )
    .unwrap();
    let lab = Lab::register(&mut engine);
    lab.invite(80);
    wait(&engine, |s| s.call == CallState::Incoming);
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    let old_token = engine.snapshot().unwrap().call_id.unwrap();
    engine.hangup_call(&old_token).unwrap();
    loop {
        let (packet, _) = receive(&lab.incoming);
        if packet.starts_with("BYE ") {
            break; // The network drops the peer's response to this BYE.
        }
    }
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(engine.snapshot().unwrap().last_error.is_none());
    lab.invite(81);
    wait(&engine, |s| s.call == CallState::Incoming);
    let new_token = engine.snapshot().unwrap().call_id.unwrap();
    assert_ne!(old_token, new_token);
    assert_eq!(
        engine.hangup_call(&old_token).unwrap_err().code,
        ErrorCode::InvalidState
    );
    engine.answer_call(&new_token).unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    lab.incoming
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    // Cross the old BYE's real 32-second transaction timeout while a different
    // call owns the slot: its late teardown must not clear the replacement.
    let until = Instant::now() + Duration::from_secs(35);
    let mut old_bye_retransmitted = false;
    while Instant::now() < until {
        if let Some(packet) = read_if_available(&lab.incoming)
            && packet.starts_with("BYE ")
            && header(&packet, "Call-ID:") == "synthetic-80"
        {
            old_bye_retransmitted = true;
        }
        let state = engine.snapshot().unwrap();
        assert_eq!(state.call, CallState::Active);
        assert_eq!(state.call_id.as_deref(), Some(new_token.as_str()));
        assert!(state.last_error.is_none());
    }
    assert!(
        old_bye_retransmitted,
        "old BYE transaction was not still pending"
    );
    lab.incoming
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    lab.in_dialog(&response, "BYE");
    lab.response("SIP/2.0 200");
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(engine.snapshot().unwrap().last_error.is_none());
    engine.shutdown().unwrap();
}

#[test]
fn unsupported_refer_is_rejected_without_changing_the_established_call() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut engine = Engine::start(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join("sip-refer-policy.wav")),
    )
    .unwrap();
    let lab = Lab::register(&mut engine);
    lab.invite(90);
    wait(&engine, |s| s.call == CallState::Incoming);
    lab.response("SIP/2.0 180");
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    let token = engine.snapshot().unwrap().call_id;
    lab.request_in_dialog(
        &response,
        "REFER",
        2,
        "Refer-To: <sip:999@127.0.0.1>\r\nRefer-Sub: false\r\n",
        "",
    );
    let rejected = lab.response("SIP/2.0 ");
    assert!(
        rejected.starts_with("SIP/2.0 501"),
        "unsupported REFER was accepted: {}",
        rejected.lines().next().unwrap()
    );
    let state = engine.snapshot().unwrap();
    assert_eq!(state.call, CallState::Active);
    assert_eq!(state.call_id, token);
    assert!(
        !header(&response, "Allow:")
            .split(',')
            .any(|m| m.trim() == "REFER")
    );
    engine.shutdown().unwrap();
}

#[test]
fn incoming_nat_peer_port_variation_routes_ack_and_bye() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut engine = Engine::start(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join("sip-synthetic-nat.wav")),
    )
    .unwrap();
    let lab = Lab::register(&mut engine);
    assert_ne!(
        lab.registrar.local_addr().unwrap().port(),
        lab.incoming.local_addr().unwrap().port()
    );
    let mut previous_token: Option<String> = None;
    for sequence in 1..=2 {
        lab.invite(sequence);
        wait(&engine, |s| s.call == CallState::Incoming);
        let identity = engine.snapshot().unwrap().caller.unwrap();
        if sequence == 1 {
            assert_eq!(identity.name.as_deref(), Some("Lab Caller"));
            assert_eq!(identity.number.as_deref(), Some("+493012345"));
        } else {
            assert!(identity.number.is_none());
            assert_eq!(identity.name.as_deref(), Some("Unterdrückte Rufnummer"));
        }
        lab.contact(&lab.response("SIP/2.0 180"));
        let token = engine.snapshot().unwrap().call_id.unwrap();
        if let Some(previous) = &previous_token {
            assert_eq!(
                engine.answer_call(previous).unwrap_err().code,
                ErrorCode::InvalidState
            );
            assert_eq!(
                engine.hangup_call(previous).unwrap_err().code,
                ErrorCode::InvalidState
            );
        }
        engine.answer_call(&token).unwrap();
        assert_eq!(engine.snapshot().unwrap().call, CallState::Answering);
        assert_eq!(engine.snapshot().unwrap().caller.as_ref(), Some(&identity));
        assert_eq!(
            engine.reject_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        assert_eq!(
            engine.answer_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        let response = lab.response("SIP/2.0 200");
        lab.in_dialog(&response, "ACK");
        wait(&engine, |s| s.call == CallState::Active);
        assert_eq!(engine.snapshot().unwrap().caller.as_ref(), Some(&identity));
        assert_eq!(
            engine.reject_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        lab.in_dialog(&response, "BYE");
        lab.response("SIP/2.0 200");
        wait(&engine, |s| s.call == CallState::Idle);
        assert!(engine.snapshot().unwrap().call_id.is_none());
        assert!(engine.snapshot().unwrap().caller.is_none());
        previous_token = Some(token);
    }
    // A remote CANCEL must release an unanswered call and preserve the next slot.
    let invite = lab.invite(3);
    wait(&engine, |s| s.call == CallState::Incoming);
    assert_eq!(
        engine.snapshot().unwrap().caller.unwrap(),
        phone_core::CallerIdentity {
            name: Some("Unterdrückte Rufnummer".into()),
            number: None,
        }
    );
    let cancel = format!(
        "CANCEL {} SIP/2.0\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 1 CANCEL\r\nContent-Length: 0\r\n\r\n",
        lab.target,
        header(&invite, "Via:"),
        header(&invite, "From:"),
        header(&invite, "To:"),
        header(&invite, "Call-ID:")
    );
    lab.incoming.send_to(cancel.as_bytes(), lab.local).unwrap();
    let rejected = lab.response("SIP/2.0 487");
    let ack = format!(
        "ACK {} SIP/2.0\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 1 ACK\r\nContent-Length: 0\r\n\r\n",
        lab.target,
        header(&invite, "Via:"),
        header(&invite, "From:"),
        header(&rejected, "To:"),
        header(&invite, "Call-ID:")
    );
    lab.incoming.send_to(ack.as_bytes(), lab.local).unwrap();
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(engine.snapshot().unwrap().caller.is_none());
    // No ACK: the native transaction must time out, close the slot and permit
    // another incoming call. This deliberately exercises the real SIP timer.
    lab.invite(4);
    wait(&engine, |s| s.call == CallState::Incoming);
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    assert_eq!(engine.snapshot().unwrap().call, CallState::Answering);
    lab.incoming
        .set_read_timeout(Some(Duration::from_secs(35)))
        .unwrap();
    let until = Instant::now() + Duration::from_secs(40);
    loop {
        let (packet, _) = receive(&lab.incoming);
        if packet.starts_with("BYE ") {
            let reply = format!(
                "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {}\r\nContent-Length: 0\r\n\r\n",
                header(&packet, "Via:"),
                header(&packet, "From:"),
                header(&packet, "To:"),
                header(&packet, "Call-ID:"),
                header(&packet, "CSeq:")
            );
            lab.incoming.send_to(reply.as_bytes(), lab.local).unwrap();
            break;
        }
        assert!(
            Instant::now() < until,
            "native missing-ACK timeout did not terminate"
        );
    }
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(engine.snapshot().unwrap().call_id.is_none());
    lab.contact(&response);
    lab.incoming
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    lab.invite(5);
    wait(&engine, |s| s.call == CallState::Incoming);
    assert_eq!(
        engine.snapshot().unwrap().caller.unwrap(),
        phone_core::CallerIdentity {
            name: Some("SIP User".into()),
            number: Some("alice.sip_42".into()),
        }
    );
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    lab.in_dialog(&response, "BYE");
    lab.response("SIP/2.0 200");
    wait(&engine, |s| s.call == CallState::Idle);
    for (sequence, expected_number) in [
        (6, None),
        (7, Some("+4930555".to_owned())),
        (8, None),
        (9, Some("a".repeat(80))),
    ] {
        let invite = lab.invite(sequence);
        wait(&engine, |s| s.call == CallState::Incoming);
        let state = engine.snapshot().unwrap();
        let identity = state.caller.unwrap();
        assert_eq!(identity.number, expected_number);
        if sequence == 9 {
            assert_eq!(identity.name, Some("é".repeat(64)));
        }
        if sequence == 8 {
            assert_eq!(identity.name.as_deref(), Some("Unterdrückte Rufnummer"));
        }
        engine
            .reject_call(state.call_id.as_deref().unwrap())
            .unwrap();
        let response = lab.response("SIP/2.0 486");
        let ack = format!(
            "ACK {} SIP/2.0\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 1 ACK\r\nContent-Length: 0\r\n\r\n",
            lab.target,
            header(&invite, "Via:"),
            header(&invite, "From:"),
            header(&response, "To:"),
            header(&invite, "Call-ID:")
        );
        lab.incoming.send_to(ack.as_bytes(), lab.local).unwrap();
        wait(&engine, |s| s.call == CallState::Idle);
        assert!(engine.snapshot().unwrap().caller.is_none());
    }
    engine.shutdown().unwrap();
    assert!(engine.snapshot().unwrap().caller.is_none());
}

#[test]
fn media_blackout_releases_call_without_user_action_and_preserves_remote_hold() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let recording = std::env::temp_dir().join("sip-media-blackout.wav");
    let mut engine = Engine::start(AudioMode::TestTone, Some(recording.clone())).unwrap();
    let mut lab = Lab::register(&mut engine);
    lab.invite(20);
    wait(&engine, |s| s.call == CallState::Incoming);
    engine.answer().unwrap();
    let dialog = lab.response("SIP/2.0 200");
    lab.in_dialog(&dialog, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    let media_port: u16 = dialog
        .lines()
        .find_map(|line| line.strip_prefix("m=audio "))
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let rtp = UdpSocket::bind("127.0.0.1:0").unwrap();
    lab.incoming.set_nonblocking(true).unwrap();
    lab.registrar.set_nonblocking(true).unwrap();
    // Feed real PCMU to the native pipeline, and finish initial SIP media
    // updates before simulating a total loss of the peer's network path.
    for sequence in 0_u16..150 {
        let mut packet = [0x80_u8; 172];
        packet[0] = 0x80;
        packet[1] = 0;
        packet[2..4].copy_from_slice(&sequence.to_be_bytes());
        packet[4..8].copy_from_slice(&(u32::from(sequence) * 160).to_be_bytes());
        packet[8..12].copy_from_slice(&1_u32.to_be_bytes());
        rtp.send_to(&packet, (std::net::Ipv4Addr::LOCALHOST, media_port))
            .unwrap();
        service_media_peer(&mut lab);
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        std::fs::metadata(&recording).unwrap().len() > 4096,
        "native receive recording must prove media flowed before the blackout"
    );
    assert_eq!(engine.snapshot().unwrap().call, CallState::Active);

    // No BYE, RTP, REGISTER replies or user hangup: the media watchdog must
    // release the global slot by itself. Real upstream timers are unchanged.
    let until = Instant::now() + Duration::from_secs(65);
    while engine.snapshot().unwrap().call != CallState::Idle && Instant::now() < until {
        thread::sleep(Duration::from_millis(20));
    }
    let ended = engine.snapshot().unwrap();
    assert_eq!(
        ended.call,
        CallState::Idle,
        "lost network must not reserve the only call slot forever"
    );
    assert_eq!(ended.last_error.unwrap().code, ErrorCode::NativeFailure);
    assert!(ended.call_id.is_none());
    service_media_peer(&mut lab);
    lab.incoming.set_nonblocking(false).unwrap();
    lab.invite(21);
    wait(&engine, |s| s.call == CallState::Incoming);
    assert!(engine.snapshot().unwrap().last_error.is_none());
    engine.answer().unwrap();
    let held_dialog = lab.response("SIP/2.0 200");
    lab.in_dialog(&held_dialog, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    let endpoint = lab.incoming.local_addr().unwrap();
    let hold_sdp = "v=0\r\no=lab 1 2 IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\na=sendonly\r\n";
    let hold = format!(
        "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKmedia-hold;rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 2 INVITE\r\nContact: <sip:caller@{endpoint}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{hold_sdp}",
        lab.target,
        header(&held_dialog, "From:"),
        header(&held_dialog, "To:"),
        header(&held_dialog, "Call-ID:"),
        hold_sdp.len()
    );
    lab.incoming.send_to(hold.as_bytes(), lab.local).unwrap();
    let held_response = lab.response("SIP/2.0 200");
    let hold_ack = format!(
        "ACK {} SIP/2.0\r\nVia: SIP/2.0/UDP {endpoint};branch=z9hG4bKmedia-hold-ack;rport\r\nMax-Forwards: 70\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: 2 ACK\r\nContent-Length: 0\r\n\r\n",
        lab.target,
        header(&held_response, "From:"),
        header(&held_response, "To:"),
        header(&held_response, "Call-ID:")
    );
    lab.incoming
        .send_to(hold_ack.as_bytes(), lab.local)
        .unwrap();
    wait(&engine, |s| s.call == CallState::Held);
    let token = engine.snapshot().unwrap().call_id.unwrap();
    lab.incoming.set_nonblocking(true).unwrap();
    let until = Instant::now() + Duration::from_secs(65);
    while Instant::now() < until {
        service_media_peer(&mut lab);
        let state = engine.snapshot().unwrap();
        assert_eq!(
            state.call,
            CallState::Held,
            "remote hold must not time out for missing RTP"
        );
        assert_eq!(state.call_id.as_deref(), Some(token.as_str()));
        thread::sleep(Duration::from_millis(20));
    }
    lab.incoming.set_nonblocking(false).unwrap();
    let resume = hold
        .replace("media-hold", "media-resume")
        .replace("CSeq: 2 INVITE", "CSeq: 3 INVITE")
        .replace("o=lab 1 2", "o=lab 1 3")
        .replace("sendonly", "sendrecv");
    lab.incoming.send_to(resume.as_bytes(), lab.local).unwrap();
    lab.response("SIP/2.0 200");
    let resume_ack = hold_ack
        .replace("media-hold-ack", "media-resume-ack")
        .replace("CSeq: 2 ACK", "CSeq: 3 ACK");
    lab.incoming
        .send_to(resume_ack.as_bytes(), lab.local)
        .unwrap();
    wait(&engine, |s| s.call == CallState::Active);
    lab.incoming.set_nonblocking(true).unwrap();
    // Long hold must not consume the resumed call's media recovery interval.
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until {
        service_media_peer(&mut lab);
        assert_eq!(engine.snapshot().unwrap().call, CallState::Active);
        thread::sleep(Duration::from_millis(20));
    }
    engine.hangup_call(&token).unwrap();
    wait(&engine, |s| s.call == CallState::Idle);
    engine.shutdown().unwrap();
}

fn service_media_peer(lab: &mut Lab) {
    while let Some(packet) = read_if_available(&lab.registrar) {
        if packet.starts_with("REGISTER ") {
            lab.register_response(&packet);
        }
    }
    while let Some(packet) = read_if_available(&lab.incoming) {
        if !packet.starts_with("INVITE ") && !packet.starts_with("BYE ") {
            continue;
        }
        let body = if packet.starts_with("INVITE ") {
            "v=0\r\no=lab 1 3 IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n"
        } else {
            ""
        };
        let content_type = if body.is_empty() {
            ""
        } else {
            "Content-Type: application/sdp\r\n"
        };
        let response = format!(
            "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:caller@{}>\r\n{content_type}Content-Length: {}\r\n\r\n{body}",
            header(&packet, "Via:"),
            header(&packet, "From:"),
            header(&packet, "To:"),
            header(&packet, "Call-ID:"),
            header(&packet, "CSeq:"),
            lab.incoming.local_addr().unwrap(),
            body.len()
        );
        lab.incoming
            .send_to(response.as_bytes(), lab.local)
            .unwrap();
    }
}
