//! Caller matching must never guess ownership from names or short number suffixes.
use phone_core::{Contact, ContactPhone, ContactSource, contact_match_key, resolve_contact};

fn contact(id: &str, number: &str) -> Contact {
    Contact {
        id: id.into(),
        source: ContactSource::Macos,
        name: "Testkontakt".into(),
        organization: String::new(),
        favorite: false,
        phones: vec![ContactPhone {
            label: "Arbeit".into(),
            number: number.into(),
            destination: phone_core::contact_dial_target(number),
        }],
    }
}

#[test]
fn resolves_formatting_and_explicit_international_prefixes_only() {
    let contacts = [contact("a", "+49 (30) 123-456")];
    assert_eq!(resolve_contact(&contacts, "004930123456").unwrap().id, "a");
    assert_eq!(resolve_contact(&contacts, "+49 30 123456").unwrap().id, "a");
    for target in [
        "123456",
        "030123456",
        "anonymous",
        "",
        "sip:4930123456@pbx",
        "+49(0)30123456",
    ] {
        assert!(resolve_contact(&contacts, target).is_none(), "{target}");
    }
}

#[test]
fn namesakes_and_duplicate_owners_never_become_a_false_identity() {
    let mut first = contact("a", "1234");
    first.phones.push(first.phones[0].clone());
    assert_eq!(resolve_contact(&[first.clone()], "1234").unwrap().id, "a");
    assert!(resolve_contact(&[first, contact("b", "1234")], "1234").is_none());
    assert_eq!(contact_match_key("0012").as_deref(), Some("0012"));
    assert_eq!(contact_match_key("*21*1234#").as_deref(), Some("*21*1234#"));
}

#[test]
fn german_number_matching_is_explicit_and_never_matches_short_suffixes() {
    let contacts = [contact("mobile", "+49 151 55501234")];
    let preferences = phone_core::ContactsPreferences::default();
    for number in ["0151 55501234", "+4915155501234", "004915155501234"] {
        assert_eq!(
            phone_core::resolve_contact_with_preferences(&contacts, number, &preferences)
                .unwrap()
                .id,
            "mobile"
        );
    }
    for number in ["55501234", "1234", "+4315155501234", "anonymous"] {
        assert!(
            phone_core::resolve_contact_with_preferences(&contacts, number, &preferences).is_none()
        );
    }
    let disabled = phone_core::ContactsPreferences {
        match_german_numbers: false,
        ..preferences
    };
    assert!(
        phone_core::resolve_contact_with_preferences(&contacts, "015155501234", &disabled)
            .is_none()
    );
    assert!(
        phone_core::resolve_contact_with_preferences(
            &[contact("a", "015155501234"), contact("b", "+4915155501234")],
            "015155501234",
            &phone_core::ContactsPreferences::default()
        )
        .is_none()
    );
}
