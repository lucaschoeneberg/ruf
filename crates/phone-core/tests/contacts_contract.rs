//! Telephone targets from contact data must never change their meaning silently.

use phone_core::contact_dial_target;

#[test]
fn formatted_numbers_preserve_country_prefix_extensions_and_service_codes() {
    for (original, expected) in [
        ("+49 30 123-456", "+4930123456"),
        ("030 / 123 456", "030123456"),
        ("(030) 123.456", "030123456"),
        ("1234", "1234"),
        ("*21*1234#", "*21*1234#"),
        ("+49\u{a0}30\u{202f}1234", "+49301234"),
    ] {
        assert_eq!(contact_dial_target(original).as_deref(), Some(expected));
    }
}

#[test]
fn ambiguous_or_unsupported_numbers_cannot_become_a_different_call() {
    for original in [
        "",
        "  ",
        "+",
        "*#",
        "+49 (0)30 1234",
        "+49(0)301234",
        "0049 (0)30 1234",
        "123 ext. 45",
        "123;45",
        "123,45",
        "sip:alice@example.org",
        "123\n456",
        "☎123",
        "++49123",
        "123+45",
        "١٢٣",
        "１２３",
    ] {
        assert_eq!(contact_dial_target(original), None, "{original:?}");
    }
    assert_eq!(contact_dial_target(&"1".repeat(65)), None);
    assert_eq!(contact_dial_target(&"1".repeat(64)), Some("1".repeat(64)));
}
