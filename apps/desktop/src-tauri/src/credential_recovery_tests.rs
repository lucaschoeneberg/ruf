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
    mutations: AtomicUsize,
}
impl store::CredentialStore for RecoveryCredentials {
    fn read(&self, _key: &str, interactive: bool) -> Result<String, PhoneError> {
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
        let directory =
            std::env::temp_dir().join(format!("ruf-recovery-{label}-{}", std::process::id()));
        let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
        peer.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let key = "abcd-1234".to_owned();
        let saved = SavedSettings {
            accounts: vec![SavedAccount {
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
        assert_eq!(snapshot.accounts[0].registration, RegistrationState::Failed);
        assert_eq!(
            snapshot.accounts[0].last_error.as_ref().unwrap().code,
            ErrorCode::CredentialAccessRequired
        );
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
                key: "abcd-2222".into(),
                name: "Other account".into(),
                registrar: fixture.peer.local_addr().unwrap().to_string(),
                username: "other".into(),
            };
            let id = session.engine()?.restore_unavailable_account(
                &saved.name,
                &AccountConfig {
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
