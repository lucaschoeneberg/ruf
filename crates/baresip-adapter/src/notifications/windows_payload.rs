use super::CallNotificationAction;

pub(super) fn parse_activation(arguments: &str) -> Option<CallNotificationAction> {
    let (action, token) = arguments.split_once(':')?;
    if token.is_empty()
        || token.len() > 128
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return None;
    }
    match action {
        "answer" => Some(CallNotificationAction::Answer(token.to_owned())),
        "reject" => Some(CallNotificationAction::Reject(token.to_owned())),
        "open" => Some(CallNotificationAction::Open(token.to_owned())),
        _ => None,
    }
}

pub(super) fn toast_xml(token: &str, title: &str, body: &str) -> String {
    format!(
        "<toast launch=\"open:{token}\" scenario=\"incomingCall\" activationType=\"foreground\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual><actions><action content=\"Annehmen\" arguments=\"answer:{token}\" activationType=\"foreground\"/><action content=\"Ablehnen\" arguments=\"reject:{token}\" activationType=\"foreground\"/></actions><audio silent=\"true\"/></toast>",
        escape_text(title),
        escape_text(body),
    )
}

fn escape_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' | '\n' | '\r' => escaped.push(character),
            character if character >= ' ' && character != '\u{fffe}' && character != '\u{ffff}' => {
                escaped.push(character)
            }
            _ => {}
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_keep_the_exact_opaque_call_token() {
        assert!(
            matches!(parse_activation("answer:call_A-2"), Some(CallNotificationAction::Answer(token)) if token == "call_A-2")
        );
        assert!(
            matches!(parse_activation("reject:old_call"), Some(CallNotificationAction::Reject(token)) if token == "old_call")
        );
        assert!(
            matches!(parse_activation("open:call_1"), Some(CallNotificationAction::Open(token)) if token == "call_1")
        );
    }

    #[test]
    fn malformed_or_unbounded_activations_are_ignored() {
        for value in [
            "",
            "answer:",
            "answer:with:colon",
            "Answer:abc",
            "reject:<xml>",
            "open:a b",
            "answer:ümlaut",
            "answer:a\0b",
            "answer:a\nb",
        ] {
            assert!(parse_activation(value).is_none(), "{value:?}");
        }
        assert!(parse_activation(&format!("answer:{}", "a".repeat(129))).is_none());
    }

    #[test]
    fn payload_escapes_caller_text_and_disables_duplicate_audio() {
        let xml = toast_xml("call_1", "A & <B> \"C\"", "O'Connor\u{0} +49");
        assert!(xml.contains("A &amp; &lt;B&gt; &quot;C&quot;"));
        assert!(xml.contains("O&apos;Connor +49"));
        assert!(xml.contains("arguments=\"answer:call_1\""));
        assert!(xml.contains("arguments=\"reject:call_1\""));
        assert!(xml.contains("launch=\"open:call_1\""));
        assert!(xml.contains("<audio silent=\"true\"/>"));
    }
}
