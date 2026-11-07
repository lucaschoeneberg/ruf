//! Real TLS handshakes and SIP dialogs with generated exclusive test roots.
use crate::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, MediaProtection,
    MediaSecurityPolicy, RegistrationState, SipTransport,
};
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub(super) struct Peer {
    process: Child,
    directory: PathBuf,
    port: u16,
    pub(super) dns: std::net::SocketAddr,
}
impl Peer {
    fn new(label: &str, mode: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
        let directory = root
            .join("native/build/security-tests")
            .join(format!("{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        Self::from_directory(directory, mode)
    }
    // Existing runner-generated certificates must remain mounted and unchanged.
    pub(super) fn from_directory(directory: PathBuf, mode: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
        let mut process = Command::new("python3")
            .arg(root.join("tests/native/tls_peer.py"))
            .arg(&directory)
            .arg(mode)
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut port = String::new();
        BufReader::new(process.stdout.take().unwrap())
            .read_line(&mut port)
            .unwrap();
        let mut ports = port.split_whitespace();
        let port = ports.next().unwrap().parse().unwrap();
        let dns_port: u16 = ports.next().unwrap().parse().unwrap();
        Self {
            process,
            directory,
            port,
            dns: ([127, 0, 0, 1], dns_port).into(),
        }
    }
    fn engine(&self) -> Engine {
        Engine::start_with_test_ca_and_dns(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join(format!("ruf-sec-{}.wav", std::process::id()))),
            self.directory.join("ca.pem"),
            self.dns,
        )
        .unwrap()
    }
    fn config(&self, host: &str) -> AccountConfig {
        AccountConfig {
            registrar: format!("{host}:{}", self.port),
            username: "tls-fixture".into(),
            password: "synthetic-only".into(),
            audio_mode: AudioMode::TestTone,
            transport: SipTransport::Tls,
            media_security: MediaSecurityPolicy::Required,
            auth_username: Some("digest-user".into()),
            outbound_proxy: None,
        }
    }
    fn packets(&self) -> String {
        std::fs::read_to_string(self.directory.join("packets.log")).unwrap_or_default()
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
fn wait(engine: &Engine, predicate: impl Fn(&EngineSnapshot) -> bool) -> EngineSnapshot {
    let until = Instant::now() + Duration::from_secs(6);
    loop {
        let state = engine.snapshot().unwrap();
        if predicate(&state) {
            return state;
        }
        assert!(Instant::now() < until, "TLS fixture timeout: {state:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}
#[test]
fn trusted_tls_contact_ip_keeps_identity_and_other_hostname_cannot_borrow_connection() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("verified", "valid");
    let mut engine = peer.engine();
    let first = engine
        .save_account(None, "A", &peer.config("localhost"))
        .unwrap();
    engine.wait_account_update(first).unwrap();
    wait(&engine, |s| {
        s.accounts[0].registration == RegistrationState::Registered
    });
    let mut other = peer.config("127.0.0.1");
    other.username = "other-host".into();
    let second = engine.save_account(None, "B", &other).unwrap();
    engine.wait_account_update(second).unwrap();
    let state = wait(&engine, |s| {
        s.accounts
            .iter()
            .any(|a| a.id == second && a.registration == RegistrationState::Failed)
    });
    assert_eq!(
        state
            .accounts
            .iter()
            .find(|a| a.id == second)
            .unwrap()
            .last_error
            .as_ref()
            .unwrap()
            .code,
        ErrorCode::CertificateRejected
    );
    assert!(!peer.packets().contains("other-host"));
    engine.dial("600").unwrap();
    let state = wait(&engine, |s| {
        s.call == CallState::Active
            && s.calls.first().is_some_and(|c| {
                c.security.tls_peer_verified && c.security.media == MediaProtection::SdesSrtp
            })
    });
    let id = state.call_id.unwrap();
    engine.set_call_held(&id, true).unwrap();
    wait(&engine, |s| {
        s.calls[0].local_held && !s.calls[0].hold_pending
    });
    engine.set_call_held(&id, false).unwrap();
    wait(&engine, |s| {
        !s.calls[0].local_held && !s.calls[0].hold_pending
    });
    engine.hangup_call(&id).unwrap();
    wait(&engine, |s| s.call == CallState::Idle);
    assert!(peer.packets().contains("DNS 1"));
    assert!(peer.packets().contains("ACK "));
    assert!(peer.packets().contains("BYE "));
    engine.shutdown().unwrap();
}
#[test]
fn required_media_rejects_missing_crypto_without_encrypted_indicator() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("missing-crypto", "missingcrypto");
    let mut engine = peer.engine();
    engine.configure_account(&peer.config("localhost")).unwrap();
    wait(&engine, |s| s.registration == RegistrationState::Registered);
    engine.dial("600").unwrap();
    let state = wait(&engine, |s| s.last_call_failure.is_some());
    assert_eq!(
        state.last_call_failure.unwrap().error.code,
        ErrorCode::MediaSecurityRequired
    );
    assert!(
        state
            .calls
            .iter()
            .all(|c| c.security.media != MediaProtection::SdesSrtp)
    );
    engine.shutdown().unwrap();
}
#[test]
fn platform_default_does_not_trust_generated_private_root() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("untrusted", "valid");
    let mut engine = Engine::start_with_test_dns(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join(format!("ruf-sec-{}.wav", std::process::id()))),
        peer.dns,
    )
    .unwrap();
    engine.configure_account(&peer.config("localhost")).unwrap();
    let state = wait(&engine, |s| s.registration == RegistrationState::Failed);
    assert_eq!(
        state.accounts[0].last_error.as_ref().unwrap().code,
        ErrorCode::CertificateRejected
    );
    assert!(peer.packets().contains("DNS 1"));
    assert!(!peer.packets().contains("REGISTER "));
    engine.shutdown().unwrap();
}

#[test]
fn tls_proxy_routes_and_separate_digest_username_authenticates() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("proxy", "digest");
    let mut engine = peer.engine();
    let mut config = peer.config("unused.invalid");
    config.outbound_proxy = Some(format!("localhost:{}", peer.port));
    engine.configure_account(&config).unwrap();
    wait(&engine, |s| s.registration == RegistrationState::Registered);
    assert!(peer.packets().contains("DNS 1"));
    assert!(peer.packets().contains("AUTH digest-user"));
    engine.dial("600").unwrap();
    wait(&engine, |s| {
        s.call == CallState::Active && s.calls[0].security.tls_peer_verified
    });
    engine.hangup().unwrap();
    engine.shutdown().unwrap();
    assert!(!peer.packets().contains("PLAINTEXT"));
}
#[test]
fn contact_udp_and_invalid_crypto_fail_without_plaintext_fallback() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for (label, mode, expected) in [
        ("contact-udp", "contactudp", ErrorCode::TransportFailure),
        ("bad-key", "badkey", ErrorCode::MediaSecurityRequired),
        (
            "plaintext-media",
            "plaintext",
            ErrorCode::MediaSecurityRequired,
        ),
    ] {
        let peer = Peer::new(label, mode);
        let mut engine = peer.engine();
        engine.configure_account(&peer.config("localhost")).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        engine.dial("600").unwrap();
        let state = wait(&engine, |s| s.last_call_failure.is_some());
        assert_eq!(state.last_call_failure.unwrap().error.code, expected);
        assert!(!peer.packets().contains("PLAINTEXT"));
        engine.shutdown().unwrap();
    }
}
#[test]
fn explicit_tcp_keeps_plaintext_policy_separate_from_tls_and_media() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("tcp", "tcp");
    let mut engine = peer.engine();
    let mut config = peer.config("127.0.0.1");
    config.transport = SipTransport::Tcp;
    config.media_security = MediaSecurityPolicy::Disabled;
    engine.configure_account(&config).unwrap();
    wait(&engine, |s| s.registration == RegistrationState::Registered);
    engine.dial("600").unwrap();
    let state = wait(&engine, |s| s.call == CallState::Active);
    assert_eq!(
        state.calls[0].security.signaling_transport,
        Some(SipTransport::Tcp)
    );
    assert!(!state.calls[0].security.tls_peer_verified);
    assert_eq!(state.calls[0].security.media, MediaProtection::Unprotected);
    engine.hangup().unwrap();
    engine.shutdown().unwrap();
    assert!(!peer.packets().contains("PLAINTEXT"));
}

#[test]
fn incoming_call_on_authenticated_outbound_tls_connection_retains_actual_peer_evidence() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new("incoming", "incoming");
    let mut engine = peer.engine();
    engine.configure_account(&peer.config("localhost")).unwrap();
    let state = wait(&engine, |s| s.call == CallState::Incoming);
    assert!(state.calls[0].security.tls_peer_verified);
    engine
        .answer_call(state.call_id.as_deref().unwrap())
        .unwrap();
    wait(&engine, |s| {
        s.call == CallState::Active && s.calls[0].security.media == MediaProtection::SdesSrtp
    });
    engine.hangup().unwrap();
    engine.shutdown().unwrap();
    assert!(!peer.packets().contains("PLAINTEXT"));
}

#[test]
fn another_verified_tls_host_cannot_terminate_an_existing_dialog() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let peer = Peer::new("peer-authority", "peer-attack");
    let mut engine = peer.engine();
    let first = engine
        .save_account(None, "A", &peer.config("localhost"))
        .unwrap();
    engine.wait_account_update(first).unwrap();
    wait(&engine, |s| {
        s.accounts[0].registration == RegistrationState::Registered
    });
    let mut second = peer.config("127.0.0.1");
    second.username = "other-host".into();
    let id = engine.save_account(None, "B", &second).unwrap();
    engine.wait_account_update(id).unwrap();
    wait(&engine, |s| {
        s.accounts
            .iter()
            .all(|a| a.registration == RegistrationState::Registered)
    });
    engine.dial("600").unwrap();
    let state = wait(&engine, |s| s.call == CallState::Active);
    let token = state.call_id.unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !peer.packets().contains("ATTACK_SENT") {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        engine.snapshot().unwrap().call,
        CallState::Active,
        "a verified wrong-host socket acquired dialog authority"
    );
    engine.set_call_held(&token, true).unwrap();
    wait(&engine, |s| {
        s.calls[0].local_held && !s.calls[0].hold_pending
    });
    engine.hangup_call(&token).unwrap();
    engine.shutdown().unwrap();
}

fn established_rekey_failure(mode: &str) {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let peer = Peer::new(mode, mode);
    let mut engine = peer.engine();
    let account = engine
        .save_account(None, "Rekey", &peer.config("localhost"))
        .unwrap();
    engine.wait_account_update(account).unwrap();
    wait(&engine, |s| s.registration == RegistrationState::Registered);
    engine.dial("600").unwrap();
    let active = wait(&engine, |s| {
        s.call == CallState::Active && s.calls[0].security.media == MediaProtection::SdesSrtp
    });
    let id = active.call_id.unwrap();
    let survivor = if mode == "rekey-offer" {
        engine.start_consultation(&id, "601").unwrap();
        let pair = wait(&engine, |s| {
            s.calls.len() == 2
                && s.call_id.as_deref() != Some(id.as_str())
                && s.call == CallState::Active
        });
        Some(pair.call_id.unwrap())
    } else {
        None
    };
    if mode == "rekey-local" {
        engine.set_call_held(&id, true).unwrap();
    } else {
        std::fs::write(peer.directory.join("rekey"), "trigger").unwrap();
    }
    let ended = wait(&engine, |s| {
        !s.calls.iter().any(|c| c.id == id) && s.last_call_failure.is_some()
    });
    if let Some(survivor) = &survivor {
        assert_eq!(ended.calls.len(), 1);
        assert_eq!(ended.calls[0].id, *survivor);
        assert_eq!(ended.calls[0].state, CallState::Active);
        assert_eq!(ended.calls[0].security.media, MediaProtection::SdesSrtp);
        assert!(!ended.calls[0].local_held);
    } else {
        assert!(ended.calls.is_empty());
        assert_eq!(ended.call, CallState::Idle);
    }
    let failure = ended.last_call_failure.unwrap();
    assert_eq!(failure.call_id, id);
    assert_eq!(failure.account_id, account);
    assert_eq!(failure.error.code, ErrorCode::MediaSecurityRequired);
    assert_eq!(ended.registration, RegistrationState::Registered);
    let packets = peer.packets();
    let initial = packets
        .lines()
        .find(|l| l.starts_with("RX_TRANSACTION|INVITE|"))
        .unwrap();
    let wire_id = initial.split('|').nth(2).unwrap();
    let bye_marker = format!("RX_TRANSACTION|BYE|{wire_id}|");
    let until = Instant::now() + Duration::from_secs(2);
    while !peer.packets().contains(&bye_marker) {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let packets = peer.packets();
    let bye = packets.find(&bye_marker).unwrap();
    let marker = match mode {
        "rekey-local" => {
            let reinvite = packets
                .lines()
                .filter(|l| l.starts_with(&format!("RX_TRANSACTION|INVITE|{wire_id}|")))
                .nth(1)
                .unwrap();
            let sequence = reinvite
                .rsplit('|')
                .next()
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap();
            format!("RX_TRANSACTION|ACK|{wire_id}|{sequence} ACK")
        }
        "rekey-offer" | "rekey-codec" | "rekey-proto" => {
            format!("RX_TRANSACTION|SIP/2.0 488|{wire_id}|999 INVITE")
        }
        _ => format!("REKEY_ACK_SENT|{wire_id}|999 ACK"),
    };
    assert!(
        packets[..bye].contains(&marker),
        "the failed transaction's ACK/488 must precede its BYE: {packets}"
    );
    if let Some(survivor) = survivor {
        engine.set_call_muted(&survivor, true).unwrap();
        wait(&engine, |s| s.calls[0].muted);
        engine.hangup_call(&survivor).unwrap();
        wait(&engine, |s| s.call == CallState::Idle);
    }
    // The same account/UA must remain reusable after only its failed dialog ends.
    engine.dial("601").unwrap();
    let next = wait(&engine, |s| {
        s.call == CallState::Active && s.calls[0].security.media == MediaProtection::SdesSrtp
    });
    assert_ne!(next.call_id.as_deref(), Some(id.as_str()));
    engine.hangup().unwrap();
    wait(&engine, |s| s.call == CallState::Idle);
    engine.shutdown().unwrap();
}
#[test]
fn established_local_rekey_failure_is_terminal_and_owned() {
    established_rekey_failure("rekey-local");
}
#[test]
fn established_remote_rekey_offer_failure_is_terminal_and_owned() {
    established_rekey_failure("rekey-offer");
}
#[test]
fn established_remote_rekey_ack_failure_is_terminal_and_owned() {
    established_rekey_failure("rekey-ack");
}

#[test]
fn invalid_crypto_with_incompatible_codec_remains_fatal() {
    established_rekey_failure("rekey-codec");
}

#[test]
fn unsupported_media_transport_remains_fatal() {
    established_rekey_failure("rekey-proto");
}
