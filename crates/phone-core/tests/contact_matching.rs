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
