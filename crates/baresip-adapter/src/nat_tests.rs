//! Synthetic loopback signaling only: no real account, DNS or external network.
use crate::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, RegistrationState,
};
use std::{
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
    fn socket() -> UdpSocket {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket
    }
    fn register(engine: &mut Engine) -> Self {
        let registrar = Self::socket();
        let incoming = Self::socket();
        let mapped = Self::socket();
        engine
            .configure_account(&AccountConfig {
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
                "SIP/2.0 200 OK\r\nVia: {};received=127.0.0.1;rport={}\r\nFrom: {}\r\nTo: {};tag=registrar\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: {}\r\nContent-Length: 0\r\n\r\n",
                header(&packet, "Via:"),
                mapped.local_addr().unwrap().port(),
                header(&packet, "From:"),
                header(&packet, "To:"),
                header(&packet, "Call-ID:"),
                header(&packet, "CSeq:"),
                contact
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
        let endpoint = self.incoming.local_addr().unwrap();
        let sdp = "v=0\r\no=lab 1 1 IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n";
        let packet = format!(
            "INVITE {} SIP/2.0\r\nVia: SIP/2.0/UDP {};branch=z9hG4bKsynthetic{};rport\r\nMax-Forwards: 70\r\nFrom: <sip:caller@lab>;tag=caller{}\r\nTo: <{}>\r\nCall-ID: synthetic-{}\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
            self.target,
            endpoint,
            sequence,
            sequence,
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
}

#[test]
fn incoming_nat_peer_port_variation_routes_ack_and_bye() {
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
        assert_eq!(
            engine.answer_call(&token).unwrap_err().code,
            ErrorCode::InvalidState
        );
        let response = lab.response("SIP/2.0 200");
        lab.in_dialog(&response, "ACK");
        wait(&engine, |s| s.call == CallState::Active);
        lab.in_dialog(&response, "BYE");
        lab.response("SIP/2.0 200");
        wait(&engine, |s| s.call == CallState::Idle);
        assert!(engine.snapshot().unwrap().call_id.is_none());
        previous_token = Some(token);
    }
    // A remote CANCEL must release an unanswered call and preserve the next slot.
    let invite = lab.invite(3);
    wait(&engine, |s| s.call == CallState::Incoming);
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
    engine.answer().unwrap();
    let response = lab.response("SIP/2.0 200");
    lab.in_dialog(&response, "ACK");
    wait(&engine, |s| s.call == CallState::Active);
    lab.in_dialog(&response, "BYE");
    lab.response("SIP/2.0 200");
    wait(&engine, |s| s.call == CallState::Idle);
    engine.shutdown().unwrap();
}
