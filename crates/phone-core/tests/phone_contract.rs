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

#[test]
fn registrar_port_accepts_only_ascii_digits() {
    let mut input = account();
    input.registrar = "127.0.0.1:+5070".into();
    assert_eq!(input.validate().unwrap_err().code, ErrorCode::InvalidInput);
}

fn multiple_accounts() -> EngineSnapshot {
    use phone_core::AccountSnapshot;
    EngineSnapshot {
        accounts: vec![
            AccountSnapshot {
                id: 1,
                name: "Büro".into(),
                registrar: "pbx.test".into(),
                username: "1000".into(),
                registration: RegistrationState::Registered,
                last_error: None,
                removing: false,
            },
            AccountSnapshot {
                id: 2,
                name: "Privat".into(),
                registrar: "home.test".into(),
                username: "2000".into(),
                registration: RegistrationState::Failed,
                last_error: None,
                removing: false,
            },
        ],
        selected_account_id: Some(1),
        registration: RegistrationState::Registered,
        ..Default::default()
    }
}

#[test]
fn outgoing_selection_uses_its_own_registration_and_rejects_unknown_ids() {
    let mut state = multiple_accounts();
    state.select_account(2).unwrap();
    assert_eq!(state.selected_account_id, Some(2));
    assert_eq!(state.registration, RegistrationState::Failed);
    assert_eq!(
        state.reserve_dial("600").unwrap_err().code,
        ErrorCode::NotRegistered
    );
    assert_eq!(
        state.select_account(999).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(state.selected_account_id, Some(2));
    state.select_account(1).unwrap();
    state.reserve_dial("600").unwrap();
    assert_eq!(state.call_account_id, Some(1));
    assert_eq!(state.select_account(2).unwrap_err().code, ErrorCode::Busy);
    assert_eq!(state.selected_account_id, Some(1));
}

#[test]
fn sound_defaults_and_device_boundaries_preserve_native_configuration() {
    use phone_core::SoundSettings;
    let mut sound = SoundSettings::default();
    assert!(sound.ringtone_enabled);
    assert!(
        sound.input_device.is_empty()
            && sound.output_device.is_empty()
            && sound.ring_device.is_empty()
    );
    sound.input_device = "Mikrofon – Büro".into();
    assert!(sound.validate().is_ok());
    sound.output_device = "x".repeat(128);
    assert_eq!(sound.validate().unwrap_err().code, ErrorCode::InvalidInput);
    sound.output_device = "default".into();
    sound.ring_device = "bad\0device".into();
    assert!(sound.validate().is_err());
}

#[test]
fn reserving_outgoing_call_discards_previous_caller_identity() {
    let mut state = multiple_accounts();
    state.caller = Some(phone_core::CallerIdentity {
        name: Some("Lab Caller".into()),
        number: Some("1001".into()),
    });
    state.reserve_dial("600").unwrap();
    assert!(state.caller.is_none());
}
