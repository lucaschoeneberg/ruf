//! Actual native account recovery with synthetic in-memory credentials only.
use super::*;
use phone_core::RegistrationState;
use std::{
    net::UdpSocket,
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

static NATIVE_RECOVERY_TEST: Mutex<()> = Mutex::new(());

struct RecoveryCredentials {
    permit: bool,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
    interactive_reads: AtomicUsize,
    background_reads: AtomicUsize,
    mutations: AtomicUsize,
}
impl store::CredentialStore for RecoveryCredentials {
    fn read(&self, _key: &str, interactive: bool) -> Result<String, PhoneError> {
        if !interactive {
            self.background_reads.fetch_add(1, Ordering::Relaxed);
        }
        if interactive {
            self.interactive_reads.fetch_add(1, Ordering::Relaxed);
            self.entered.send(()).unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            if self.permit {
                return Ok("synthetic-recovery-password".into());
            }
        }
        Err(PhoneError::new(
            ErrorCode::CredentialAccessRequired,
            "Synthetic authorization required.",
        ))
    }
    fn write(&self, _key: &str, _password: &str) -> Result<(), PhoneError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(PhoneError::new(
            ErrorCode::CredentialFailure,
            "Recovery must not write credentials.",
        ))
    }
    fn delete(&self, _key: &str) -> Result<(), PhoneError> {
        self.mutations.fetch_add(1, Ordering::Relaxed);
        Err(PhoneError::new(
            ErrorCode::CredentialFailure,
            "Recovery must not delete credentials.",
        ))
    }
}

struct Fixture {
    state: PhoneState,
    directory: PathBuf,
    before: Vec<u8>,
    id: u32,
    peer: UdpSocket,
    credentials: Arc<RecoveryCredentials>,
    entered: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
}
impl Fixture {
    fn new(label: &str, permit: bool) -> Self {
        Self::with_enabled(label, permit, true)
    }
    fn with_enabled(label: &str, permit: bool, enabled: bool) -> Self {
        Self::with_profile(label, permit, enabled, false)
    }
    fn with_profile(label: &str, permit: bool, enabled: bool, secure: bool) -> Self {
        let directory =
            std::env::temp_dir().join(format!("ruf-recovery-{label}-{}", std::process::id()));
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let key = "abcd-1234".to_owned();
        let saved = SavedSettings {
            accounts: vec![SavedAccount {
                transport: if secure {
                    phone_core::SipTransport::Tls
                } else {
                    phone_core::SipTransport::Udp
                },
                media_security: if secure {
                    phone_core::MediaSecurityPolicy::Required
                } else {
                    phone_core::MediaSecurityPolicy::Disabled
                },
                auth_username: secure.then(|| "digest-user".into()),
                outbound_proxy: secure.then(|| "proxy.test:5061".into()),
                enabled,
                key: key.clone(),
                name: "Synthetic recovery".into(),
                registrar: peer.local_addr().unwrap().to_string(),
                username: "recovery".into(),
            }],
            selected_key: Some(key),
            audio_mode: AudioMode::TestTone,
            ..Default::default()
        };
        let file = AccountStore {
            path: directory.join("accounts.json"),
        };
        file.write(&saved).unwrap();
        let before = std::fs::read(&file.path).unwrap();
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        let credentials = Arc::new(RecoveryCredentials {
            permit,
            entered: entered_tx,
            release: Mutex::new(release_rx),
            interactive_reads: AtomicUsize::new(0),
            background_reads: AtomicUsize::new(0),
            mutations: AtomicUsize::new(0),
        });
        let state = PhoneState::default();
        state
            .with(|session| {
                session.credentials = credentials.clone();
                session.load(directory.clone(), directory.clone())
            })
            .unwrap();
        let snapshot = state.with(|s| s.snapshot()).unwrap();
        assert_eq!(snapshot.accounts[0].enabled, enabled);
        if enabled {
            assert_eq!(snapshot.accounts[0].registration, RegistrationState::Failed);
            assert_eq!(
                snapshot.accounts[0].last_error.as_ref().unwrap().code,
                ErrorCode::CredentialAccessRequired
            );
        } else {
            assert_eq!(
                snapshot.accounts[0].registration,
                RegistrationState::Unconfigured
            );
            assert!(snapshot.accounts[0].last_error.is_none());
            assert_eq!(credentials.background_reads.load(Ordering::Relaxed), 0);
        }
        assert_eq!(
            credentials.interactive_reads.load(Ordering::Relaxed),
            0,
            "startup must stay silent"
        );
        Self {
            state,
            directory,
            before,
            id: snapshot.accounts[0].id,
            peer,
            credentials,
            entered,
            release,
        }
    }
    fn reconnect(&self) -> std::thread::JoinHandle<Result<EngineSnapshot, PhoneError>> {
        let state = self.state.clone();
        let directory = self.directory.clone();
        let id = self.id;
        std::thread::spawn(move || {
            state.reconnect_account_blocking(directory.clone(), directory, id)
        })
    }
    fn unchanged(&self) {
        assert_eq!(
            std::fs::read(self.directory.join("accounts.json")).unwrap(),
            self.before
        );
        assert_eq!(self.credentials.mutations.load(Ordering::Relaxed), 0);
    }
    fn accept_registration(&self) -> (String, std::net::SocketAddr) {
        let mut bytes = [0; 8192];
        let (size, source) = self.peer.recv_from(&mut bytes).unwrap();
        let packet = std::str::from_utf8(&bytes[..size]).unwrap();
        assert!(packet.starts_with("REGISTER "));
        let contact = header(packet, "Contact:")
            .split_once('<')
            .unwrap()
            .1
            .split_once('>')
            .unwrap()
            .0
            .to_owned();
        let response = format!(
            "SIP/2.0 200 OK\r\nVia: {}\r\nFrom: {}\r\nTo: {};tag=recovery\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: {}\r\nContent-Length: 0\r\n\r\n",
            header(packet, "Via:"),
            header(packet, "From:"),
            header(packet, "To:"),
            header(packet, "Call-ID:"),
            header(packet, "CSeq:"),
            header(packet, "Contact:")
        );
        self.peer.send_to(response.as_bytes(), source).unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            let snapshot = self.state.with(|s| s.snapshot()).unwrap();
            if snapshot.registration == RegistrationState::Registered {
                assert!(snapshot.accounts[0].last_error.is_none());
                break;
            }
            assert!(
                Instant::now() < until,
                "recovered native account failed to register"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        (contact, source)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.state.shutdown();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn header<'a>(packet: &'a str, name: &str) -> &'a str {
    packet
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .unwrap()
        .trim()
}

#[test]
#[ignore = "Requires the isolated Asterisk lab; bun run test:asterisk runs it"]
fn saved_account_accepts_first_incoming_call_after_each_restart() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());

    // Only OS credential access is substituted. Metadata restoration, native
    // digest authentication, Contact registration and inbound signaling are real.
    struct StartupCredentials;
    impl store::CredentialStore for StartupCredentials {
        fn read(&self, key: &str, interactive: bool) -> Result<String, PhoneError> {
            assert_eq!(key, "abcd-1003");
            assert!(!interactive, "startup must not require foreground recovery");
            Ok("sip-lab-only-not-a-secret".into())
        }
        fn write(&self, _key: &str, _password: &str) -> Result<(), PhoneError> {
            panic!("startup must not rewrite saved passwords")
        }
        fn delete(&self, _key: &str) -> Result<(), PhoneError> {
            panic!("startup must not delete saved passwords")
        }
    }
    fn wait(state: &PhoneState, predicate: impl Fn(&EngineSnapshot) -> bool) -> EngineSnapshot {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let snapshot = state.with(|session| session.snapshot()).unwrap();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "restored account did not reach the expected native state: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let directory =
        std::env::temp_dir().join(format!("ruf-startup-incoming-{}", std::process::id()));
    let store = AccountStore {
        path: directory.join("accounts.json"),
    };
    store
        .write(&SavedSettings {
            accounts: vec![SavedAccount {
                transport: Default::default(),
                media_security: Default::default(),
                auth_username: None,
                outbound_proxy: None,
                enabled: true,
                key: "abcd-1003".into(),
                name: "Synthetic startup".into(),
                registrar: "127.0.0.1:5070".into(),
                // This lab endpoint deliberately disables server Contact rewriting.
                username: "1003".into(),
            }],
            selected_key: Some("abcd-1003".into()),
            audio_mode: AudioMode::TestTone,
            ..Default::default()
        })
        .unwrap();
    let before = std::fs::read(&store.path).unwrap();
    let lab = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/interop/asterisk/compose.yml");

    // Recreate desktop state and the native engine, reusing only durable metadata
    // and credentials. Neither cycle saves/reconnects an account or dials first.
    for _ in 0..2 {
        let state = PhoneState::default();
        state
            .with(|session| {
                session.credentials = Arc::new(StartupCredentials);
                session.load(directory.clone(), directory.clone())
            })
            .unwrap();
        let registered = wait(&state, |s| s.registration == RegistrationState::Registered);
        assert_eq!(registered.accounts.len(), 1);
        let account = &registered.accounts[0];
        assert_eq!(account.username, "1003");
        assert_eq!(registered.selected_account_id, Some(account.id));
        assert!(account.registered_at_ms.is_some());
        assert!(account.last_error.is_none());
        assert_eq!(registered.call, CallState::Idle);

        assert!(
            std::process::Command::new("docker")
                .args(["compose", "-f"])
                .arg(&lab)
                .args([
                    "exec",
                    "-T",
                    "asterisk",
                    "asterisk",
                    "-rx",
                    "channel originate PJSIP/1003 application Wait 1",
                ])
                .status()
                .unwrap()
                .success()
        );
        let incoming = wait(&state, |s| s.call == CallState::Incoming);
        assert_eq!(incoming.call_account_id, Some(account.id));
        let call_id = incoming.call_id.unwrap();
        state
            .with(|session| session.engine()?.answer_call(&call_id))
            .unwrap();
        wait(&state, |s| s.call == CallState::Active);
        let ended = wait(&state, |s| s.call == CallState::Idle);
        assert!(ended.call_account_id.is_none());
        assert!(ended.last_error.is_none(), "{ended:?}");
        assert_eq!(ended.registration, RegistrationState::Registered);
        state.shutdown().unwrap();
        assert_eq!(std::fs::read(&store.path).unwrap(), before);
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn foreground_reconnect_recovers_without_rewriting_metadata_or_credentials() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("success", true);
    let reconnect = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    // Reading live phone state remains possible while OS authorization is pending.
    let (tx, rx) = mpsc::channel();
    let reading = fixture.state.clone();
    std::thread::spawn(move || tx.send(reading.with(|s| s.snapshot())).unwrap());
    let observed = rx.recv_timeout(Duration::from_secs(1));
    fixture.release.send(()).unwrap();
    assert!(observed.is_ok(), "authorization held the phone state lock");
    let applied = reconnect.join().unwrap().unwrap();
    assert_eq!(applied.accounts[0].id, fixture.id);
    fixture.accept_registration();
    fixture.unchanged();
}

#[test]
fn cancelled_authorization_keeps_the_saved_account_and_password_reference() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("cancel", false);
    let reconnect = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        reconnect.join().unwrap().unwrap_err().code,
        ErrorCode::CredentialAccessRequired
    );
    let snapshot = fixture.state.with(|s| s.snapshot()).unwrap();
    assert_eq!(snapshot.accounts[0].id, fixture.id);
    assert_eq!(snapshot.accounts[0].registration, RegistrationState::Failed);
    fixture.unchanged();
}

#[test]
fn shutdown_during_authorization_prevents_a_late_reconnect() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("shutdown", true);
    let reconnect = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.state.shutdown().unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        reconnect.join().unwrap().unwrap_err().code,
        ErrorCode::InvalidState
    );
    fixture.unchanged();
}

#[test]
fn incoming_call_during_authorization_prevents_account_replacement() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("incoming", true);
    let first = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.release.send(()).unwrap();
    first.join().unwrap().unwrap();
    let (target, local) = fixture.accept_registration();

    let reconnect = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    let peer = fixture.peer.local_addr().unwrap();
    let sdp = "v=0\r\no=recovery 1 1 IN IP4 127.0.0.1\r\ns=synthetic\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=audio 9000 RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n";
    let invite = format!(
        "INVITE {target} SIP/2.0\r\nVia: SIP/2.0/UDP {peer};branch=z9hG4bKrecovery;rport\r\nMax-Forwards: 70\r\nFrom: <sip:caller@lab>;tag=recovery-caller\r\nTo: <{target}>\r\nCall-ID: recovery-live\r\nCSeq: 1 INVITE\r\nContact: <sip:caller@{peer}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{sdp}",
        sdp.len()
    );
    fixture.peer.send_to(invite.as_bytes(), local).unwrap();
    let mut packet = [0; 8192];
    let until = Instant::now() + Duration::from_secs(3);
    loop {
        let (length, _) = fixture.peer.recv_from(&mut packet).unwrap();
        if packet[..length].starts_with(b"SIP/2.0 180") {
            break;
        }
        assert!(
            Instant::now() < until,
            "incoming call did not ring during credential authorization"
        );
    }
    fixture.release.send(()).unwrap();
    assert_eq!(reconnect.join().unwrap().unwrap_err().code, ErrorCode::Busy);
    let snapshot = fixture.state.with(|s| s.snapshot()).unwrap();
    assert_eq!(snapshot.call, CallState::Incoming);
    assert_eq!(snapshot.call_account_id, Some(fixture.id));
    assert_eq!(
        snapshot.accounts[0].registration,
        RegistrationState::Registered
    );
    fixture.unchanged();
}

#[test]
fn unknown_reconnect_id_is_rejected_before_requesting_credentials() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("unknown", true);
    let error = fixture
        .state
        .reconnect_account_blocking(
            fixture.directory.clone(),
            fixture.directory.clone(),
            fixture.id + 1,
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(
        fixture
            .credentials
            .interactive_reads
            .load(Ordering::Relaxed),
        0
    );
    fixture.unchanged();
}

#[test]
fn failed_removal_restores_the_outgoing_account_selection() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("remove-rollback", false);
    fixture
        .state
        .with(|session| {
            let saved = SavedAccount {
                transport: Default::default(),
                media_security: Default::default(),
                auth_username: None,
                outbound_proxy: None,
                enabled: true,
                key: "abcd-2222".into(),
                name: "Other account".into(),
                registrar: fixture.peer.local_addr().unwrap().to_string(),
                username: "other".into(),
            };
            let id = session.engine()?.restore_unavailable_account(
                &saved.name,
                &AccountConfig {
                    transport: Default::default(),
                    media_security: Default::default(),
                    auth_username: None,
                    outbound_proxy: None,
                    registrar: saved.registrar.clone(),
                    username: saved.username.clone(),
                    password: "unavailable".into(),
                    audio_mode: AudioMode::TestTone,
                },
                PhoneError::new(
                    ErrorCode::CredentialAccessRequired,
                    "Synthetic authorization required.",
                ),
            )?;
            let mut next = session.settings.clone();
            next.accounts.push(saved.clone());
            session.write(&next)?;
            session.settings = next;
            session.keys.insert(id, saved.key);
            Ok(())
        })
        .unwrap();
    // A directory at the temporary-file path deterministically rejects replacement.
    std::fs::create_dir(fixture.directory.join("accounts.json.tmp")).unwrap();
    let error = fixture
        .state
        .with(|session| session.remove(fixture.id))
        .unwrap_err();
    let selected = fixture
        .state
        .with(|session| {
            let snapshot = session.snapshot()?;
            Ok(session.keys[&snapshot.selected_account_id.unwrap()].clone())
        })
        .unwrap();
    assert_eq!(error.code, ErrorCode::StorageFailure);
    assert_eq!(
        selected, "abcd-1234",
        "failed removal changed outgoing account"
    );
}

struct InvalidSavedPassword;
impl store::CredentialStore for InvalidSavedPassword {
    fn read(&self, key: &str, _interactive: bool) -> Result<String, PhoneError> {
        Ok(if key == "abcd-2222" {
            "synthetic-password"
        } else {
            ""
        }
        .into())
    }
    fn write(&self, _key: &str, _password: &str) -> Result<(), PhoneError> {
        panic!("startup must not rewrite saved passwords")
    }
    fn delete(&self, _key: &str) -> Result<(), PhoneError> {
        panic!("startup must not delete saved passwords")
    }
}

#[test]
fn invalid_saved_password_does_not_discard_other_restored_accounts() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let directory =
        std::env::temp_dir().join(format!("ruf-invalid-saved-password-{}", std::process::id()));
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    let store = AccountStore {
        path: directory.join("accounts.json"),
    };
    store
        .write(&SavedSettings {
            accounts: [("abcd-1111", "broken"), ("abcd-2222", "valid")]
                .into_iter()
                .map(|(key, username)| SavedAccount {
                    transport: Default::default(),
                    media_security: Default::default(),
                    auth_username: None,
                    outbound_proxy: None,
                    enabled: true,
                    key: key.into(),
                    name: username.into(),
                    registrar: peer.local_addr().unwrap().to_string(),
                    username: username.into(),
                })
                .collect(),
            audio_mode: AudioMode::TestTone,
            ..Default::default()
        })
        .unwrap();
    let before = std::fs::read(&store.path).unwrap();
    let mut session = PhoneSession {
        credentials: Arc::new(InvalidSavedPassword),
        ..Default::default()
    };
    let result = session.load(directory.clone(), directory.clone());
    let snapshot = session.snapshot().unwrap();
    session.shutdown().unwrap();
    assert_eq!(std::fs::read(&store.path).unwrap(), before);
    std::fs::remove_dir_all(directory).unwrap();
    assert!(
        result.is_ok(),
        "one invalid saved password prevented all account restoration: {result:?}"
    );
    assert_eq!(snapshot.accounts.len(), 2);
    assert_eq!(snapshot.accounts[0].registration, RegistrationState::Failed);
    assert_eq!(
        snapshot.accounts[0].last_error.as_ref().unwrap().code,
        ErrorCode::CredentialFailure
    );
    assert!(snapshot.accounts[1].last_error.is_none());
}

#[test]
fn reconnect_classifies_malformed_saved_password_without_changing_the_account() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("reconnect-malformed", false);
    fixture
        .state
        .with(|session| {
            session.credentials = Arc::new(InvalidSavedPassword);
            Ok(())
        })
        .unwrap();
    let error = fixture
        .state
        .reconnect_account_blocking(
            fixture.directory.clone(),
            fixture.directory.clone(),
            fixture.id,
        )
        .unwrap_err();
    let snapshot = fixture.state.with(|session| session.snapshot()).unwrap();
    fixture.unchanged();
    assert_eq!(error.code, ErrorCode::CredentialFailure);
    assert_eq!(snapshot.accounts[0].id, fixture.id);
}

#[test]
fn failed_removal_keeps_an_account_with_a_malformed_saved_password() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::new("remove-malformed", false);
    fixture
        .state
        .with(|session| {
            session.credentials = Arc::new(InvalidSavedPassword);
            Ok(())
        })
        .unwrap();
    std::fs::create_dir(fixture.directory.join("accounts.json.tmp")).unwrap();
    let error = fixture
        .state
        .with(|session| session.remove(fixture.id))
        .unwrap_err();
    let snapshot = fixture.state.with(|session| session.snapshot()).unwrap();
    fixture.unchanged();
    assert_eq!(error.code, ErrorCode::StorageFailure);
    assert_eq!(
        snapshot.accounts.len(),
        1,
        "failed removal lost the saved account"
    );
    assert_eq!(snapshot.accounts[0].registration, RegistrationState::Failed);
    assert_eq!(
        snapshot.accounts[0].last_error.as_ref().unwrap().code,
        ErrorCode::CredentialFailure
    );
    fixture
        .state
        .with(|session| {
            assert_eq!(session.keys[&snapshot.accounts[0].id], "abcd-1234");
            assert_eq!(snapshot.selected_account_id, Some(snapshot.accounts[0].id));
            Ok(())
        })
        .unwrap();
}

fn activate(
    state: &PhoneState,
    directory: PathBuf,
    cache: PathBuf,
    id: u32,
    enabled: bool,
) -> Result<EngineSnapshot, PhoneError> {
    let intent = state.begin_activation(id)?;
    state.apply_activation(directory, cache, id, enabled, intent)
}

#[test]
fn disabled_account_load_never_reads_credentials_and_activation_preserves_identity_and_metadata() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::with_enabled("activation", true, false);
    let state = fixture.state.clone();
    let directory = fixture.directory.clone();
    let id = fixture.id;
    let enabling =
        std::thread::spawn(move || activate(&state, directory.clone(), directory, id, true));
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    assert!(!fixture.state.with(|s| s.snapshot()).unwrap().accounts[0].enabled);
    fixture.release.send(()).unwrap();
    let enabled = enabling.join().unwrap().unwrap();
    assert_eq!(enabled.selected_account_id, Some(id));
    assert!(enabled.accounts[0].enabled);
    fixture.accept_registration();
    let disabled = activate(
        &fixture.state,
        fixture.directory.clone(),
        fixture.directory.clone(),
        id,
        false,
    )
    .unwrap();
    assert_eq!(disabled.selected_account_id, Some(id));
    assert!(!disabled.accounts[0].enabled);
    assert!(disabled.accounts[0].registered_at_ms.is_none());
    assert_eq!(
        fixture
            .credentials
            .interactive_reads
            .load(Ordering::Relaxed),
        1
    );
    assert_eq!(
        fixture.credentials.background_reads.load(Ordering::Relaxed),
        0
    );
    let metadata = AccountStore {
        path: fixture.directory.join("accounts.json"),
    }
    .load()
    .unwrap();
    assert!(!metadata.accounts[0].enabled);
    assert_eq!(metadata.accounts[0].key, "abcd-1234");
    fixture.state.shutdown().unwrap();
    let mut reloaded = PhoneSession {
        credentials: fixture.credentials.clone(),
        ..Default::default()
    };
    reloaded
        .load(fixture.directory.clone(), fixture.directory.clone())
        .unwrap();
    assert!(!reloaded.snapshot().unwrap().accounts[0].enabled);
    assert_eq!(
        fixture.credentials.background_reads.load(Ordering::Relaxed),
        0
    );
    reloaded.shutdown().unwrap();
}

#[test]
fn newer_disable_intent_cancels_an_enable_waiting_for_authorization() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::with_enabled("activation-stale", true, false);
    let state = fixture.state.clone();
    let directory = fixture.directory.clone();
    let id = fixture.id;
    let enabling =
        std::thread::spawn(move || activate(&state, directory.clone(), directory, id, true));
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    // Register the newer intent before releasing the OS authorization callback.
    let disable_intent = fixture.state.begin_activation(id).unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        enabling.join().unwrap().unwrap_err().code,
        ErrorCode::InvalidState
    );
    let disabled = fixture
        .state
        .apply_activation(
            fixture.directory.clone(),
            fixture.directory.clone(),
            id,
            false,
            disable_intent,
        )
        .unwrap();
    assert!(!disabled.accounts[0].enabled);
    assert_eq!(disabled.selected_account_id, Some(id));
    fixture.unchanged();
}

#[test]
fn failed_activation_metadata_write_preserves_disabled_state_and_credential_key() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::with_enabled("activation-storage", true, false);
    let state = fixture.state.clone();
    let directory = fixture.directory.clone();
    let id = fixture.id;
    let enabling =
        std::thread::spawn(move || activate(&state, directory.clone(), directory, id, true));
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    std::fs::create_dir(fixture.directory.join("accounts.json.tmp")).unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        enabling.join().unwrap().unwrap_err().code,
        ErrorCode::StorageFailure
    );
    assert!(
        !fixture
            .state
            .with(|session| session.snapshot())
            .unwrap()
            .accounts[0]
            .enabled
    );
    fixture.unchanged();
}

#[test]
fn cancelled_enable_keeps_disabled_metadata_and_credentials_unchanged() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::with_enabled("activation-denied", false, false);
    let state = fixture.state.clone();
    let directory = fixture.directory.clone();
    let id = fixture.id;
    let enabling =
        std::thread::spawn(move || activate(&state, directory.clone(), directory, id, true));
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        enabling.join().unwrap().unwrap_err().code,
        ErrorCode::CredentialAccessRequired
    );
    assert!(
        !fixture
            .state
            .with(|session| session.snapshot())
            .unwrap()
            .accounts[0]
            .enabled
    );
    fixture.unchanged();
}

#[test]
fn shutdown_during_enable_authorization_prevents_native_start_and_metadata_changes() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let fixture = Fixture::with_enabled("activation-shutdown", true, false);
    let state = fixture.state.clone();
    let directory = fixture.directory.clone();
    let id = fixture.id;
    let enabling =
        std::thread::spawn(move || activate(&state, directory.clone(), directory, id, true));
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.state.shutdown().unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        enabling.join().unwrap().unwrap_err().code,
        ErrorCode::InvalidState
    );
    assert!(
        fixture
            .state
            .with(|session| Ok(session.engine.is_none()))
            .unwrap()
    );
    fixture.unchanged();
}

#[test]
fn encrypted_profile_survives_startup_missing_credentials_reconnect_and_restart() {
    let _guard = NATIVE_RECOVERY_TEST
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let fixture = Fixture::with_profile("security-profile", false, true, true);
    let check = |snapshot: &EngineSnapshot| {
        let account = &snapshot.accounts[0];
        assert_eq!(account.transport, phone_core::SipTransport::Tls);
        assert_eq!(
            account.media_security,
            phone_core::MediaSecurityPolicy::Required
        );
        assert_eq!(account.auth_username.as_deref(), Some("digest-user"));
        assert_eq!(account.outbound_proxy.as_deref(), Some("proxy.test:5061"));
    };
    check(&fixture.state.with(|session| session.snapshot()).unwrap());
    let reconnect = fixture.reconnect();
    fixture
        .entered
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    fixture.release.send(()).unwrap();
    assert_eq!(
        reconnect.join().unwrap().unwrap_err().code,
        ErrorCode::CredentialAccessRequired
    );
    check(&fixture.state.with(|session| session.snapshot()).unwrap());
    fixture.unchanged();
    fixture.state.shutdown().unwrap();
    let restored = PhoneState::default();
    restored
        .with(|session| {
            session.credentials = fixture.credentials.clone();
            session.load(fixture.directory.clone(), fixture.directory.clone())
        })
        .unwrap();
    check(&restored.with(|session| session.snapshot()).unwrap());
    restored.shutdown().unwrap();
    fixture.unchanged();
}
