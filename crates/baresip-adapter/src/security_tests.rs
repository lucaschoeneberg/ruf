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
struct Peer {
    process: Child,
    directory: PathBuf,
    port: u16,
}
impl Peer {
    fn new(label: &str, mode: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../");
        let directory = root
            .join("native/build/security-tests")
            .join(format!("{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
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
        Self {
            process,
            directory,
            port: port.trim().parse().unwrap(),
        }
    }
    fn engine(&self) -> Engine {
        Engine::start_with_test_ca(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join(format!("ruf-sec-{}.wav", std::process::id()))),
            self.directory.join("ca.pem"),
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
    let mut engine = Engine::start(
        AudioMode::TestTone,
        Some(std::env::temp_dir().join(format!("ruf-sec-{}.wav", std::process::id()))),
    )
    .unwrap();
    engine.configure_account(&peer.config("localhost")).unwrap();
    let state = wait(&engine, |s| s.registration == RegistrationState::Failed);
    assert_eq!(
        state.accounts[0].last_error.as_ref().unwrap().code,
        ErrorCode::CertificateRejected
    );
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
