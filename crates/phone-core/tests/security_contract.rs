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
