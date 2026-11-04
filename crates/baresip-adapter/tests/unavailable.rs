//! Portable builds must never report simulated native readiness.
#![cfg(not(feature = "native"))]

use baresip_adapter::Engine;
use phone_core::{AudioMode, ErrorCode};

#[test]
fn portable_build_explicitly_rejects_engine_start() {
    let error = Engine::start(AudioMode::System, None).err().unwrap();
    assert_eq!(error.code, ErrorCode::EngineUnavailable);
}
