//! Account encryption policy and input safety contracts.
use phone_core::{AccountConfig, ErrorCode};

fn input() -> serde_json::Value {
    serde_json::json!({"registrar":"localhost:5061","username":"alice","password":"not-a-production-secret","transport":"tls","mediaSecurity":"required","authUsername":"digest-user","outboundProxy":"proxy.example:5061"})
}
#[test]
fn encrypted_account_accepts_structured_fields() {
    let account: AccountConfig = serde_json::from_value(input()).expect("typed security account");
    account.validate().unwrap();
    assert!(!format!("{account:?}").contains("not-a-production-secret"));
}
#[test]
fn security_policy_and_optional_fields_reject_downgrade_and_injection() {
    for (field, value) in [
        ("transport", "udp"),
        ("transport", "tcp"),
        ("authUsername", ""),
        ("authUsername", "alice;auth_pass=injected"),
        ("outboundProxy", "sip:proxy.example"),
        ("outboundProxy", "proxy.example;transport=udp"),
        ("outboundProxy", "proxy.example\r\nBad: value"),
        ("outboundProxy", "proxy.example:0"),
    ] {
        let mut data = input();
        data[field] = value.into();
        let account: AccountConfig = serde_json::from_value(data).expect("input contract");
        assert_eq!(
            account.validate().unwrap_err().code,
            ErrorCode::InvalidInput,
            "{field}"
        );
    }
}

#[test]
fn registrar_and_proxy_share_host_port_boundaries() {
    let maximum_host = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    ]
    .join(".");
    let too_long_label = format!("{}.example", "a".repeat(64));
    let too_long_address = format!("{maximum_host}:65535");
    for (server, valid) in [
        ("localhost", true),
        ("127.0.0.1:1", true),
        ("proxy.example:65535", true),
        (maximum_host.as_str(), true),
        ("", false),
        ("proxy..example", false),
        ("-proxy.example", false),
        ("proxy-.example", false),
        ("proxy.example:", false),
        ("proxy.example:0", false),
        ("proxy.example:65536", false),
        ("proxy.example:+5061", false),
        ("proxy.example:5061:5062", false),
        ("sip:proxy.example", false),
        ("proxy.example;transport=tls", false),
        (too_long_label.as_str(), false),
        (too_long_address.as_str(), false),
    ] {
        for field in ["registrar", "outboundProxy"] {
            let mut data = input();
            data[field] = server.into();
            let account: AccountConfig = serde_json::from_value(data).unwrap();
            assert_eq!(account.validate().is_ok(), valid, "{field}: {server:?}");
        }
    }
}

#[test]
fn account_validation_preserves_first_error_and_password_boundaries() {
    let mut account: AccountConfig = serde_json::from_value(input()).unwrap();
    account.password.clear();
    account.outbound_proxy = Some("proxy.example:0".into());
    assert!(
        account
            .validate()
            .unwrap_err()
            .message
            .starts_with("Invalid account fields")
    );
    account.auth_username = Some("invalid;user".into());
    assert!(
        account
            .validate()
            .unwrap_err()
            .message
            .starts_with("Authentication username")
    );
    account.auth_username = None;
    account.outbound_proxy = Some("proxy.example:5061".into());
    for (password, valid) in [
        (String::new(), false),
        ("x".repeat(1024), true),
        ("x".repeat(1025), false),
        ("null\0byte".into(), false),
    ] {
        account.password = password;
        assert_eq!(account.validate().is_ok(), valid);
    }
}

#[test]
fn legacy_json_defaults_and_unknown_observation_are_safe() {
    let account: AccountConfig = serde_json::from_value(
        serde_json::json!({"registrar":"localhost","username":"alice","password":"synthetic"}),
    )
    .unwrap();
    assert_eq!(account.transport, phone_core::SipTransport::Udp);
    assert_eq!(
        account.media_security,
        phone_core::MediaSecurityPolicy::Disabled
    );
    assert!(account.auth_username.is_none() && account.outbound_proxy.is_none());
    let security = phone_core::CallSecurity::default();
    assert!(security.signaling_transport.is_none() && !security.tls_peer_verified);
    assert_eq!(security.media, phone_core::MediaProtection::Unknown);
}
