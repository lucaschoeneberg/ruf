//! Authenticated real SIP/RTP echo call using synthetic lab credentials.

#[cfg(feature = "native")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use baresip_adapter::Engine;
    use phone_core::{AccountConfig, AudioMode, CallState, EngineSnapshot, RegistrationState};
    use std::{
        env,
        path::PathBuf,
        thread,
        time::{Duration, Instant},
    };

    fn wait(
        engine: &Engine,
        label: &str,
        predicate: impl Fn(&EngineSnapshot) -> bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            let snapshot = engine.snapshot()?;
            if predicate(&snapshot) {
                return Ok(());
            }
            if let Some(error) = snapshot.last_error {
                return Err(error.into());
            }
            if Instant::now() >= until {
                return Err(format!("Timed out waiting for {label}").into());
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
    let recording = PathBuf::from(
        env::var("SIP_TEST_RECORDING").unwrap_or_else(|_| "native/build/echo.wav".into()),
    );
    let recording = std::path::absolute(recording)?;
    let account = AccountConfig {
        transport: Default::default(),
        media_security: Default::default(),
        auth_username: None,
        outbound_proxy: None,
        registrar: env::var("SIP_TEST_REGISTRAR").unwrap_or_else(|_| "127.0.0.1:5070".into()),
        username: env::var("SIP_TEST_USERNAME").unwrap_or_else(|_| "1000".into()),
        password: env::var("SIP_TEST_PASSWORD")
            .map_err(|_| "Set SIP_TEST_PASSWORD to the synthetic lab password")?,
        audio_mode: AudioMode::TestTone,
    };
    let extension = env::var("SIP_TEST_EXTENSION").unwrap_or_else(|_| "600".into());
    let mut engine = Engine::start(AudioMode::TestTone, Some(recording.clone()))?;
    engine.configure_account(&account)?;
    wait(&engine, "registration", |s| {
        s.registration == RegistrationState::Registered
    })?;
    println!("REGISTER_OK");
    engine.dial(&extension)?;
    wait(&engine, "call establishment", |s| {
        s.call == CallState::Active
    })?;
    println!("CALL_ESTABLISHED");
    assert!(
        engine.dial(&extension).is_err(),
        "A second call must be rejected"
    );
    assert!(
        engine.configure_account(&account).is_err(),
        "A live account must not be replaced"
    );
    thread::sleep(Duration::from_secs(4));
    engine.hangup()?;
    wait(&engine, "call termination", |s| s.call == CallState::Idle)?;
    engine.shutdown()?;
    println!("CALL_CLOSED; received audio: {}", recording.display());
    Ok(())
}

#[cfg(not(feature = "native"))]
fn main() {
    eprintln!("Run this example with --features native after bun run native:build.");
    std::process::exit(1);
}
