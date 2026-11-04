//! Contract tests for account safety and single-call reservation.

use phone_core::{
    AccountConfig, AudioMode, CallState, EngineSnapshot, ErrorCode, RegistrationState,
};

fn account() -> AccountConfig {
    AccountConfig {
        registrar: "127.0.0.1:5070".into(),
        username: "1000".into(),
        password: "lab-password;with@punctuation".into(),
        audio_mode: AudioMode::TestTone,
    }
}

#[test]
fn account_rejects_parameter_injection_but_preserves_password_punctuation() {
    assert!(account().validate().is_ok());
    for registrar in [
        "127.0.0.1;transport=tcp",
        "host\r\nmodule evil",
        "host:0",
        "host:65536",
        "",
        "-invalid.example",
    ] {
        let mut input = account();
        input.registrar = registrar.into();
        assert_eq!(
            input.validate().unwrap_err().code,
            ErrorCode::InvalidInput,
            "{registrar:?}"
        );
    }
    let mut input = account();
    input.username = "1000>;auth_pass=other".into();
    assert!(input.validate().is_err());
    input = account();
    input.password = "null\0byte".into();
    assert!(input.validate().is_err());
}

#[test]
fn debug_output_never_discloses_password() {
    let input = account();
    let debug = format!("{input:?}");
    assert!(!debug.contains(&input.password));
    assert!(debug.contains("1000"));
}

#[test]
fn dialing_requires_registration_and_reserves_the_only_call_slot() {
    let mut state = EngineSnapshot::default();
    assert_eq!(
        state.reserve_dial("600").unwrap_err().code,
        ErrorCode::NotRegistered
    );
    state.registration = RegistrationState::Registered;
    assert_eq!(
        state.reserve_dial("600\r\nBad: value").unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(state.call, CallState::Idle);
    state.reserve_dial("600").unwrap();
    assert_eq!(state.call, CallState::Calling);
    assert_eq!(state.reserve_dial("601").unwrap_err().code, ErrorCode::Busy);
    assert_eq!(
        state.reserve_registration().unwrap_err().code,
        ErrorCode::Busy
    );
    assert_eq!(state.call, CallState::Calling);
}

#[test]
fn registrar_with_port_must_fit_without_truncation() {
    let mut input = account();
    let host = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    ]
    .join(".");
    input.registrar = format!("{host}:65535");
    assert_eq!(input.validate().unwrap_err().code, ErrorCode::InvalidInput);
}
