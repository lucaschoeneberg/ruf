//! Isolated Asterisk interoperability: real bridges, native controls and fresh PCM.
use crate::Engine;
use phone_core::{
    AccountConfig, AudioMode, CallEventKind, CallSnapshot, CallState, EngineSnapshot, ErrorCode,
    RegistrationState, TransferState,
};
use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

const WAIT: Duration = Duration::from_secs(15);

#[test]
#[ignore = "Lab command helper regression; bun run test:asterisk"]
fn lab_command_drains_large_stdout_and_stderr_before_waiting() {
    let result = output(Command::new("python3").args([
        "-c",
        "import sys;sys.stdout.write('x'*1048576);sys.stderr.write('y'*1048576)",
    ]));
    assert!(result.status.success());
    assert_eq!(result.stdout.len(), 1048576);
    assert_eq!(result.stderr.len(), 1048576);
}

#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_dtmf_digits_and_hash_operate_ivr_and_reject_stale_tokens() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("dtmf");
    lab.engine.dial("605").unwrap();
    let state = wait(&lab.engine, |s| s.call == CallState::Active);
    let id = state.call_id.unwrap();
    assert_eq!(
        lab.engine.send_dtmf(&id, "AB").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    lab.engine.send_dtmf(&id, "5").unwrap();
    lab.engine.send_dtmf(&id, "#").unwrap();
    thread::sleep(Duration::from_secs(3));
    lab.hangup(&id);
    lab.engine.shutdown().unwrap();
    let samples = pcm(&lab.recording);
    tone(
        &samples[samples.len().saturating_sub(16000)..],
        880.0,
        "IVR-accepted-five-and-hash",
    );
    assert_eq!(
        lab.engine.send_dtmf(&id, "#").unwrap_err().code,
        ErrorCode::InvalidState
    );
    drop(lab);
    let mut lab = Lab::new("dtmf-cancel");
    lab.engine.dial("600").unwrap();
    let second = wait(&lab.engine, |s| s.call == CallState::Active)
        .call_id
        .unwrap();
    lab.engine.send_dtmf(&second, "1").unwrap();
    lab.engine.send_dtmf(&second, "2").unwrap();
    lab.hangup(&second);
    lab.engine.dial("600").unwrap();
    let third = wait(&lab.engine, |s| s.call == CallState::Active)
        .call_id
        .unwrap();
    assert_eq!(
        lab.engine.send_dtmf(&second, "#").unwrap_err().code,
        ErrorCode::InvalidState
    );
    thread::sleep(Duration::from_millis(600));
    let current = lab.engine.snapshot().unwrap();
    assert!(
        current
            .call_events
            .iter()
            .all(|event| !matches!(event.event, CallEventKind::DtmfStart { .. }))
    );
    assert_eq!(current.call_id.as_deref(), Some(third.as_str()));
    lab.hangup(&third);
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
// All diagnostic/output commands are small and bounded; a broken Docker client
// must not leave the native singleton test process waiting indefinitely.
fn output(command: &mut Command) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Drain both pipes while the process is running. Waiting for exit before
    // reading can deadlock even a healthy Asterisk command on a full pipe.
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let stdout = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let stderr = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return Output {
                status,
                stdout: stdout.join().unwrap(),
                stderr: stderr.join().unwrap(),
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("bounded lab command timed out: {command:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
fn cli(command: &str) -> String {
    let output = output(
        Command::new("docker")
            .args(["compose", "-f"])
            .arg(root().join("tests/interop/asterisk/compose.yml"))
            .args(["exec", "-T", "asterisk", "asterisk", "-rx", command]),
    );
    assert!(
        output.status.success(),
        "Asterisk command failed: {command}"
    );
    String::from_utf8(output.stdout).unwrap()
}
fn channels() -> BTreeSet<String> {
    cli("core show channels concise")
        .lines()
        .filter_map(|line| {
            let name = line.split('!').next()?;
            (name.starts_with("Local/") || name.starts_with("PJSIP/")).then(|| name.to_owned())
        })
        .collect()
}
fn wait(engine: &Engine, predicate: impl Fn(&EngineSnapshot) -> bool) -> EngineSnapshot {
    let until = Instant::now() + WAIT;
    loop {
        let state = engine.snapshot().unwrap();
        if predicate(&state) {
            return state;
        }
        assert!(Instant::now() < until, "Asterisk state timeout: {state:?}");
        thread::sleep(Duration::from_millis(20));
    }
}
fn call<'a>(s: &'a EngineSnapshot, id: &str) -> &'a CallSnapshot {
    s.calls.iter().find(|c| c.id == id).expect("owned dialog")
}
fn config(username: &str) -> AccountConfig {
    AccountConfig {
        registrar: "127.0.0.1:5070".into(),
        username: username.into(),
        password: std::env::var("SIP_TEST_PASSWORD").expect("synthetic lab password"),
        audio_mode: AudioMode::TestTone,
        transport: Default::default(),
        media_security: Default::default(),
        auth_username: None,
        outbound_proxy: None,
    }
}
fn recording(label: &str) -> PathBuf {
    root().join(format!("native/build/a{}-{label}.wav", std::process::id()))
}
fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |m| m.len())
}
fn pcm(path: &Path) -> Vec<i16> {
    let wav = std::fs::read(path).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    let mut at = 12;
    let mut result = None;
    let mut format = false;
    while at + 8 <= wav.len() {
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().unwrap()) as usize;
        assert!(
            at + 8 + size <= wav.len(),
            "truncated WAV: {}",
            path.display()
        );
        if &wav[at..at + 4] == b"fmt " {
            let f = &wav[at + 8..at + 8 + size];
            assert_eq!(u16::from_le_bytes(f[..2].try_into().unwrap()), 1);
            assert_eq!(u16::from_le_bytes(f[2..4].try_into().unwrap()), 1);
            assert_eq!(u32::from_le_bytes(f[4..8].try_into().unwrap()), 8000);
            assert_eq!(u16::from_le_bytes(f[14..16].try_into().unwrap()), 16);
            format = true;
        }
        if &wav[at..at + 4] == b"data" {
            result = Some(
                wav[at + 8..at + 8 + size]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|v| i16::from_le_bytes(*v))
                    .collect(),
            );
        }
        at += 8 + size + (size % 2);
    }
    assert!(format);
    result.expect("PCM data")
}
fn tone(samples: &[i16], hz: f64, label: &str) {
    assert!(
        samples.len() >= 8000,
        "{label}: need >=1s NEW audio, got {} samples",
        samples.len()
    );
    let rms =
        (samples.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / samples.len() as f64).sqrt();
    let crossings = samples.windows(2).filter(|p| p[0] < 0 && p[1] >= 0).count();
    let frequency = crossings as f64 * 8000.0 / samples.len() as f64;
    assert!(rms > 500.0, "{label}: silent audio RMS={rms}");
    assert!(
        (frequency - hz).abs() < 20.0,
        "{label}: expected {hz}Hz, got {frequency}"
    );
    println!(
        "MEDIA_OK {label} samples={} RMS={rms:.0} frequency={frequency:.1}Hz",
        samples.len()
    );
}
struct Lab {
    engine: Engine,
    recording: PathBuf,
    before: BTreeSet<String>,
    owned: BTreeSet<String>,
    remote_files: BTreeSet<String>,
}
impl Lab {
    fn new(label: &str) -> Self {
        let path = recording(label);
        let _ = std::fs::remove_file(&path);
        assert!(path.as_os_str().len() < 128);
        let before = channels();
        let mut engine = Engine::start(AudioMode::TestTone, Some(path.clone())).unwrap();
        engine.configure_account(&config("1000")).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        Self {
            engine,
            recording: path,
            before,
            owned: BTreeSet::new(),
            remote_files: BTreeSet::new(),
        }
    }
    fn collect(&mut self) {
        let current = channels();
        for name in current.difference(&self.before) {
            self.owned.insert(name.clone());
            if name.starts_with("Local/") {
                let data = cli(&format!("core show channel {name}"));
                if let Some(rx) = data
                    .lines()
                    .find_map(|line| line.trim().strip_prefix("LAB_RX="))
                {
                    self.remote_files.insert(rx.into());
                    self.remote_files.insert(rx.replace("-rx.wav", "-mix.wav"));
                }
            }
        }
    }
    fn incoming(&mut self, tone_extension: &str) -> String {
        assert_eq!(self.engine.snapshot().unwrap().call, CallState::Idle);
        cli(&format!(
            "channel originate Local/650@lab/n extension {tone_extension}@lab"
        ));
        let s = wait(&self.engine, |s| s.call == CallState::Incoming);
        let id = s.call_id.unwrap();
        self.engine.answer_call(&id).unwrap();
        wait(&self.engine, |s| s.call == CallState::Active);
        self.collect();
        id
    }
    fn role(&mut self, role: &str) -> (String, String) {
        let until = Instant::now() + WAIT;
        loop {
            self.collect();
            for name in channels().difference(&self.before) {
                let data = cli(&format!("core show channel {name}"));
                if data.lines().any(|l| l.trim() == format!("LAB_ROLE={role}")) {
                    let rx = data
                        .lines()
                        .find_map(|l| l.trim().strip_prefix("LAB_RX="))
                        .expect("role receive file")
                        .to_owned();
                    self.remote_files.insert(rx.clone());
                    self.remote_files.insert(rx.replace("-rx.wav", "-mix.wav"));
                    return (name.clone(), rx);
                }
            }
            assert!(Instant::now() < until, "missing synthetic role {role}");
            thread::sleep(Duration::from_millis(30));
        }
    }
    fn bridged(&self, name: &str) {
        let all = cli("bridge show all");
        let ids: Vec<_> = all
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|id| id.len() == 36 && id.bytes().filter(|&b| b == b'-').count() == 4)
            .collect();
        assert!(
            ids.iter()
                .any(|id| cli(&format!("bridge show {id}")).contains(name)),
            "{name} has no actual bridge: {all}"
        );
        println!("BRIDGE_OK {name}");
    }
    fn pjsip(&mut self) -> Vec<String> {
        self.collect();
        channels()
            .difference(&self.before)
            .filter(|c| c.starts_with("PJSIP/1000-"))
            .cloned()
            .collect()
    }
    fn child(&mut self, original: &str, destination: &str) -> String {
        self.engine
            .start_consultation(original, destination)
            .unwrap();
        let state = wait(&self.engine, |s| {
            s.calls
                .iter()
                .any(|c| c.consultation && c.state == CallState::Active)
                && s.calls.iter().all(|c| !c.hold_pending)
        });
        self.collect();
        assert!(call(&state, original).local_held);
        let child = state.calls.iter().find(|c| c.consultation).unwrap();
        assert_eq!(child.account_id, call(&state, original).account_id);
        assert_ne!(child.id, original);
        child.id.clone()
    }
    fn hangup(&mut self, id: &str) {
        self.engine.hangup_call(id).unwrap();
        wait(&self.engine, |s| !s.calls.iter().any(|c| c.id == id));
    }
    fn end_channel(&mut self, name: &str) {
        assert!(self.owned.contains(name));
        cli(&format!("channel request hangup {name}"));
    }
    fn copy(path: &str, label: &str) -> PathBuf {
        let destination = recording(label);
        let result = output(
            Command::new("docker")
                .args(["compose", "-f"])
                .arg(root().join("tests/interop/asterisk/compose.yml"))
                .args(["cp", &format!("asterisk:{path}")])
                .arg(&destination),
        );
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        destination
    }
    fn monitor(&mut self, channel: &str, label: &str) -> String {
        cli(&format!("mixmonitor stop {channel}"));
        let file = format!(
            "/tmp/ruf-lab-{}-{label}-rx.wav",
            self.recording.file_stem().unwrap().to_str().unwrap()
        );
        let mix = file.replace("-rx.wav", "-mix.wav");
        self.remote_files.insert(file.clone());
        self.remote_files.insert(mix.clone());
        let started = cli(&format!("mixmonitor start {channel} {mix},r({file})"));
        let listing = cli(&format!("mixmonitor list {channel}"));
        assert!(
            listing.contains(mix.trim_end_matches(".wav"))
                && listing.contains(file.trim_end_matches(".wav")),
            "start={started} list={listing}"
        );
        file
    }
    fn continuity(&mut self) {
        let (a, _) = self.role("original");
        let (b, _) = self.role("target");
        // New audiohooks/files started strictly AFTER final NOTIFY and Ruf teardown.
        let rx_a = self.monitor(&a, "transfer-a");
        let rx_b = self.monitor(&b, "transfer-b");
        thread::sleep(Duration::from_secs(2));
        let surviving = channels();
        assert!(
            surviving.contains(&a) && surviving.contains(&b),
            "remote peers ended with transferor"
        );
        assert!(!surviving.iter().any(|c| c.starts_with("PJSIP/1000-")));
        let partner = |name: &str| {
            if name.ends_with(";1") {
                format!("{};2", name.trim_end_matches(";1"))
            } else {
                format!("{};1", name.trim_end_matches(";2"))
            }
        };
        let pa = partner(&a);
        let pb = partner(&b);
        let all = cli("bridge show all");
        assert!(
            all.lines()
                .filter_map(|l| l.split_whitespace().next())
                .filter(|id| id.len() == 36)
                .any(|id| {
                    let bridge = cli(&format!("bridge show {id}"));
                    bridge.contains(&pa) && bridge.contains(&pb)
                }),
            "surviving peers do not share a bridge: {all}"
        );
        cli(&format!("mixmonitor stop {a}"));
        cli(&format!("mixmonitor stop {b}"));
        let stem = self.recording.file_stem().unwrap().to_str().unwrap();
        let path_a = Self::copy(&rx_a, &format!("{stem}-a"));
        let path_b = Self::copy(&rx_b, &format!("{stem}-b"));
        tone(&pcm(&path_a), 880.0, "post-NOTIFY target880->original");
        tone(&pcm(&path_b), 440.0, "post-NOTIFY original440->target");
        println!(
            "TRANSFER_CONTINUITY_OK fresh_files={}+{} bytes={}+{}",
            rx_a,
            rx_b,
            file_len(&path_a),
            file_len(&path_b)
        );
        self.end_channel(&a);
        self.end_channel(&b);
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        let _ = self.engine.shutdown();
        self.collect();
        for name in &self.owned {
            if channels().contains(name) {
                cli(&format!("channel request hangup {name}"));
            }
        }
        if !self.remote_files.is_empty() {
            let result = output(
                Command::new("docker")
                    .args(["compose", "-f"])
                    .arg(root().join("tests/interop/asterisk/compose.yml"))
                    .args(["exec", "-T", "asterisk", "rm", "-f"])
                    .args(&self.remote_files),
            );
            assert!(result.status.success());
        }
    }
}

#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_hold_resume_returns_new_audio() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("hold");
    lab.engine.dial("600").unwrap();
    let s = wait(&lab.engine, |s| s.call == CallState::Active);
    let id = s.call_id.unwrap();
    thread::sleep(Duration::from_secs(1));
    lab.engine.set_call_held(&id, true).unwrap();
    let state = wait(&lab.engine, |s| {
        call(s, &id).local_held && !call(s, &id).hold_pending
    });
    assert!(!call(&state, &id).remote_held);
    lab.engine.set_call_held(&id, false).unwrap();
    wait(&lab.engine, |s| {
        !call(s, &id).local_held && !call(s, &id).hold_pending
    });
    thread::sleep(Duration::from_millis(300));
    let base = file_len(&lab.recording);
    thread::sleep(Duration::from_secs(4));
    lab.hangup(&id);
    lab.engine.shutdown().unwrap();
    let samples = pcm(&lab.recording);
    assert!(file_len(&lab.recording) > base + 48000);
    tone(&samples[(base / 2) as usize..], 440.0, "post-resume440");
}
#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_mute_suppresses_tx_and_keeps_incoming_audio() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("mute");
    let id = lab.incoming("653");
    let (remote, _) = lab.role("original");
    thread::sleep(Duration::from_millis(400));
    let initial = lab.monitor(&remote, "mute-before");
    thread::sleep(Duration::from_secs(2));
    cli(&format!("mixmonitor stop {remote}"));
    lab.engine.set_call_muted(&id, true).unwrap();
    wait(&lab.engine, |s| call(s, &id).muted);
    thread::sleep(Duration::from_millis(400));
    let muted = lab.monitor(&remote, "mute-during");
    let received = file_len(&lab.recording);
    thread::sleep(Duration::from_secs(2));
    let received_end = file_len(&lab.recording);
    cli(&format!("mixmonitor stop {remote}"));
    lab.engine.set_call_muted(&id, false).unwrap();
    wait(&lab.engine, |s| !call(s, &id).muted);
    thread::sleep(Duration::from_millis(400));
    let restored = lab.monitor(&remote, "mute-after");
    thread::sleep(Duration::from_secs(2));
    cli(&format!("mixmonitor stop {remote}"));
    lab.hangup(&id);
    lab.engine.shutdown().unwrap();
    tone(
        &pcm(&lab.recording)[(received / 2) as usize..(received_end / 2) as usize],
        880.0,
        "incoming-while-muted",
    );
    tone(&pcm(&Lab::copy(&initial, "mute-pre")), 440.0, "before-mute");
    let silence = pcm(&Lab::copy(&muted, "mute-tx"));
    assert!(silence.len() >= 8000);
    let rms =
        (silence.iter().map(|&v| f64::from(v).powi(2)).sum::<f64>() / silence.len() as f64).sqrt();
    assert!(rms < 100.0, "mute transmitted audible audio RMS={rms}");
    tone(
        &pcm(&Lab::copy(&restored, "mute-post")),
        440.0,
        "after-unmute",
    );
    println!("MUTE_OK rms={rms:.1}");
}
#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_blind_transfer_final_notify_and_remote_audio_survive() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("blind");
    let id = lab.incoming("651");
    for c in lab.pjsip() {
        lab.bridged(&c);
    }
    lab.engine.transfer_call(&id, "602").unwrap();
    let s = wait(&lab.engine, |s| {
        s.calls.is_empty()
            && s.call_events
                .iter()
                .any(|e| e.call_id == id && matches!(e.event, CallEventKind::TransferSucceeded))
    });
    assert!(
        s.call_events
            .iter()
            .any(|e| e.call_id == id && matches!(e.event, CallEventKind::TransferStarted))
    );
    assert!(
        s.call_events
            .iter()
            .any(|e| e.call_id == id && matches!(e.event, CallEventKind::TransferSucceeded))
    );
    assert_eq!(s.call, CallState::Idle);
    lab.continuity();
}
#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_attended_transfer_switches_and_connects_remote_peers() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("attend");
    let original = lab.incoming("651");
    let child = lab.child(&original, "602");
    let legs = lab.pjsip();
    assert_eq!(legs.len(), 2);
    for c in legs {
        lab.bridged(&c);
    }
    for id in [&original, &child] {
        lab.engine.switch_call(id).unwrap();
        let s = wait(&lab.engine, |s| {
            s.call_id.as_ref() == Some(id) && s.calls.iter().all(|c| !c.hold_pending)
        });
        assert!(!call(&s, id).local_held);
        assert_eq!(s.calls.iter().filter(|c| !c.local_held).count(), 1);
    }
    lab.engine.complete_transfer(&child).unwrap();
    // Asterisk may close a leg before final NOTIFY arrives. Wait for native
    // success as well as closure; the terminal projection retains the last
    // closing leg's FIFO, which can belong to either member of this transfer.
    let s = wait(&lab.engine, |s| {
        s.calls.is_empty()
            && s.call_events.iter().any(|e| {
                (e.call_id == original || e.call_id == child)
                    && matches!(e.event, CallEventKind::TransferSucceeded)
            })
    });
    assert!(
        s.call_events
            .iter()
            .any(|e| (e.call_id == original || e.call_id == child)
                && matches!(e.event, CallEventKind::TransferSucceeded))
    );
    println!("FINAL_NOTIFY_OK attended child={child}");
    for id in [&original, &child] {
        assert_eq!(
            lab.engine.set_call_muted(id, true).unwrap_err().code,
            ErrorCode::InvalidState
        );
    }
    lab.continuity();
}
#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_incoming_consultation_cancel_failure_and_stale_tokens_recover() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("cancel");
    let original = lab.incoming("651");
    lab.engine.start_consultation(&original, "603").unwrap();
    let s = wait(&lab.engine, |s| {
        s.calls
            .iter()
            .any(|c| c.consultation && c.state == CallState::Ringing)
    });
    let child = s.calls.iter().find(|c| c.consultation).unwrap().id.clone();
    lab.engine.cancel_consultation(&child).unwrap();
    wait(&lab.engine, |s| {
        s.calls.len() == 1 && !call(s, &original).local_held && !call(s, &original).hold_pending
    });
    assert_eq!(
        lab.engine.set_call_muted(&child, true).unwrap_err().code,
        ErrorCode::InvalidState
    );
    let child = lab.child(&original, "602");
    lab.engine.cancel_consultation(&child).unwrap();
    wait(&lab.engine, |s| {
        s.calls.len() == 1 && !call(s, &original).local_held && !call(s, &original).hold_pending
    });
    lab.engine.start_consultation(&original, "999999").unwrap();
    let s = wait(&lab.engine, |s| {
        s.calls.len() == 1
            && !call(s, &original).local_held
            && !call(s, &original).hold_pending
            && call(s, &original).last_error.is_some()
    });
    assert!(call(&s, &original).last_error.is_some());
    lab.engine.transfer_call(&original, "999999").unwrap();
    wait(&lab.engine, |s| {
        call(s, &original).transfer == TransferState::Failed
    });
    assert_eq!(
        lab.engine.snapshot().unwrap().call_id.as_deref(),
        Some(original.as_str())
    );
    thread::sleep(Duration::from_millis(300));
    let base = file_len(&lab.recording);
    thread::sleep(Duration::from_secs(2));
    lab.hangup(&original);
    lab.engine.shutdown().unwrap();
    tone(
        &pcm(&lab.recording)[(base / 2) as usize..],
        440.0,
        "recovered-original",
    );
}
#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_remote_end_during_consultation_preserves_survivor() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    for end_original in [true, false] {
        let mut lab = Lab::new(if end_original { "end-a" } else { "end-b" });
        let original = lab.incoming("651");
        let original_leg = lab.pjsip().pop().unwrap();
        let child = lab.child(&original, "602");
        let target_leg = lab
            .pjsip()
            .into_iter()
            .find(|c| c != &original_leg)
            .unwrap();
        let (ended, survivor, leg) = if end_original {
            (&original, &child, &original_leg)
        } else {
            (&child, &original, &target_leg)
        };
        lab.end_channel(leg);
        wait(&lab.engine, |s| {
            s.calls.len() == 1
                && s.calls[0].id == *survivor
                && !s.calls[0].local_held
                && !s.calls[0].hold_pending
                && !s.calls[0].consultation
        });
        assert_eq!(
            lab.engine.set_call_muted(ended, true).unwrap_err().code,
            ErrorCode::InvalidState
        );
        lab.engine.set_call_muted(survivor, true).unwrap();
        wait(&lab.engine, |s| call(s, survivor).muted);
        thread::sleep(Duration::from_millis(300));
        let base = file_len(&lab.recording);
        thread::sleep(Duration::from_secs(2));
        lab.hangup(survivor);
        tone(
            &pcm(&lab.recording)[(base / 2) as usize..],
            if end_original { 880.0 } else { 440.0 },
            "remote-end-survivor",
        );
        lab.engine.dial("601").unwrap();
        wait(&lab.engine, |s| s.call == CallState::Active);
        wait(&lab.engine, |s| s.call == CallState::Idle);
    }
}

#[test]
#[ignore = "Requires isolated Asterisk; bun run test:asterisk"]
fn asterisk_original_remote_end_while_consultation_rings_still_allows_answer() {
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut lab = Lab::new("end-ring");
    let original = lab.incoming("651");
    let original_leg = lab.pjsip().pop().unwrap();
    lab.engine.start_consultation(&original, "603").unwrap();
    let s = wait(&lab.engine, |s| {
        s.calls
            .iter()
            .any(|c| c.consultation && c.state == CallState::Ringing)
    });
    let child = s.calls.iter().find(|c| c.consultation).unwrap().id.clone();
    let child_leg = lab
        .pjsip()
        .into_iter()
        .find(|c| c != &original_leg)
        .unwrap();
    lab.end_channel(&original_leg);
    wait(&lab.engine, |s| {
        s.calls.len() == 1
            && s.calls[0].id == child
            && s.calls[0].state == CallState::Ringing
            && !s.calls[0].consultation
    });
    assert_eq!(
        lab.engine.hangup_call(&original).unwrap_err().code,
        ErrorCode::InvalidState
    );
    cli(&format!("channel redirect {child_leg} lab,602,1"));
    wait(&lab.engine, |s| {
        s.calls.len() == 1 && s.calls[0].id == child && s.call == CallState::Active
    });
    thread::sleep(Duration::from_millis(300));
    let base = file_len(&lab.recording);
    thread::sleep(Duration::from_secs(2));
    lab.hangup(&child);
    tone(
        &pcm(&lab.recording)[(base / 2) as usize..],
        880.0,
        "ringing-child-survives-original-end",
    );
}

#[cfg(feature = "native-test-hooks")]
#[test]
#[ignore = "Requires isolated Asterisk TLS/SRTP; bun run test:asterisk"]
fn asterisk_verified_tls_required_srtp_returns_audio_and_refuses_plaintext() {
    use phone_core::{MediaProtection, MediaSecurityPolicy, SipTransport};
    let _guard = crate::NATIVE_TEST_LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let directory = PathBuf::from(
        std::env::var("SIP_TEST_CERT_DIRECTORY").expect("runner generated certificates"),
    );
    let peer = crate::security_tests::Peer::from_directory(directory.clone(), "valid");
    for username in ["1100", "1101"] {
        let path = recording(username);
        let _ = std::fs::remove_file(&path);
        let mut engine = Engine::start_with_test_ca_and_dns(
            AudioMode::TestTone,
            Some(path.clone()),
            directory.join("ca.pem"),
            peer.dns,
        )
        .unwrap();
        let mut account = config(username);
        account.registrar = "localhost:5071".into();
        account.transport = SipTransport::Tls;
        account.media_security = MediaSecurityPolicy::Required;
        engine.configure_account(&account).unwrap();
        wait(&engine, |s| s.registration == RegistrationState::Registered);
        engine.dial("600").unwrap();
        if username == "1101" {
            let s = wait(&engine, |s| s.last_call_failure.is_some());
            let failure = s.last_call_failure.unwrap();
            assert_eq!(failure.error.code, ErrorCode::SipNotAcceptable);
            assert_eq!(failure.error.sip_status, Some(488));
            assert!(
                s.calls
                    .iter()
                    .all(|c| c.security.media != MediaProtection::SdesSrtp)
            );
            println!("ASTERISK_PLAINTEXT_REFUSED {:?}", failure.error.code);
        } else {
            let s = wait(&engine, |s| {
                s.call == CallState::Active
                    && s.calls[0].security.media == MediaProtection::SdesSrtp
            });
            assert_eq!(
                s.calls[0].security.signaling_transport,
                Some(SipTransport::Tls)
            );
            assert!(s.calls[0].security.tls_peer_verified);
            let id = s.call_id.unwrap();
            thread::sleep(Duration::from_secs(1));
            engine.set_call_held(&id, true).unwrap();
            wait(&engine, |s| {
                call(s, &id).local_held && !call(s, &id).hold_pending
            });
            engine.set_call_held(&id, false).unwrap();
            wait(&engine, |s| {
                !call(s, &id).local_held && !call(s, &id).hold_pending
            });
            thread::sleep(Duration::from_millis(300));
            let base = file_len(&path);
            thread::sleep(Duration::from_secs(4));
            engine.hangup_call(&id).unwrap();
            wait(&engine, |s| s.call == CallState::Idle);
            engine.shutdown().unwrap();
            assert!(file_len(&path) > base + 48000);
            tone(
                &pcm(&path)[(base / 2) as usize..],
                440.0,
                "verified-TLS-required-SRTP-post-resume",
            );
        }
        engine.shutdown().unwrap();
    }
}
