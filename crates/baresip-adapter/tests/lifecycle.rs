//! Real native lifecycle tests; no mock telephone engine.
#![cfg(feature = "native")]

use baresip_adapter::Engine;
use phone_core::{AudioMode, ErrorCode, RegistrationState};

#[test]
fn native_engine_releases_its_singleton_and_resources_after_shutdown() {
    for _ in 0..3 {
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-lifecycle.wav")),
        )
        .unwrap();
        assert_eq!(
            engine.snapshot().unwrap().registration,
            RegistrationState::Unconfigured
        );
        assert_eq!(
            engine.dial("600").unwrap_err().code,
            ErrorCode::NotRegistered
        );
        assert_eq!(
            Engine::start(AudioMode::System, None).err().unwrap().code,
            ErrorCode::Busy
        );
        engine.shutdown().unwrap();
    }
}
