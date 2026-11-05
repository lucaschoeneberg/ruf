//! Real native lifecycle tests; no mock telephone engine.
#![cfg(feature = "native")]

use baresip_adapter::Engine;
use phone_core::{AudioMode, EngineSnapshot, ErrorCode, RegistrationState};

#[test]
fn native_engine_releases_its_singleton_and_resources_after_shutdown() {
    for _ in 0..3 {
        let before_native = EngineSnapshot::default().with_read_revision();
        let mut engine = Engine::start(
            AudioMode::TestTone,
            Some(std::env::temp_dir().join("sip-lifecycle.wav")),
        )
        .unwrap();
        let observed = engine.snapshot().unwrap();
        assert_eq!(observed.registration, RegistrationState::Unconfigured);
        assert!(before_native.revision < observed.revision);
        assert_eq!(
            engine.dial("600").unwrap_err().code,
            ErrorCode::NotRegistered
        );
        assert_eq!(
            Engine::start(AudioMode::System, None).err().unwrap().code,
            ErrorCode::Busy
        );
        engine.shutdown().unwrap();
        let after_native = EngineSnapshot::default().with_read_revision();
        assert!(observed.revision < after_native.revision);
    }
}
