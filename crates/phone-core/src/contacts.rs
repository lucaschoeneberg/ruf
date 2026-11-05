use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Origin of a contact; source identities must never be merged by display name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ContactSource {
    /// Contacts accessible through the macOS Contacts framework.
    Macos,
}

/// Minimal authorized contact reference for caller/history display; not an authenticated identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContactReference {
    /// Native source identifier.
    pub id: String,
    /// Contact source namespace.
    pub source: ContactSource,
    /// Current contact name, not a historical cached name.
    pub name: String,
}

/// Current platform permission, independent of SIP engine linkage.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ContactsPermission {
    /// This platform has no implemented contact provider.
    Unsupported,
    /// The user has not been asked; only an explicit request may prompt.
    NotDetermined,
    /// System policy prevents access.
    Restricted,
    /// Access was denied or revoked in system settings.
    Denied,
    /// The app may read the contacts granted by the operating system.
    Authorized,
}

/// One labeled original phone number and its conservatively validated dial target.
#[derive(Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContactPhone {
    /// Localized source label, such as Mobil or Arbeit; never a unique identifier.
    pub label: String,
    /// Unmodified source spelling for display and search.
    pub number: String,
    /// Existing dial-command syntax, or null for ambiguous/unsupported numbers.
    pub destination: Option<String>,
}

/// A read-only source contact. Namesakes remain separate entries.
#[derive(Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    /// Opaque source identifier, stable until the underlying source replaces the record.
    pub id: String,
    /// Provider used to interpret the identifier.
    pub source: ContactSource,
    /// Native formatted name, falling back to company, number or Ohne Namen.
    pub name: String,
    /// Company/organization, empty when absent.
    pub organization: String,
    /// All phone numbers, including numbers unsupported by the current dial API.
    pub phones: Vec<ContactPhone>,
    /// App-local favorite; never writes back to the system address book.
    pub favorite: bool,
}

/// A complete read of the currently accessible native address book.
#[derive(Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContactsSnapshot {
    /// Current access decision; no entries are returned without access.
    pub permission: ContactsPermission,
    /// Source contacts; empty on unsupported, undecided, restricted or denied access.
    pub contacts: Vec<Contact>,
}

/// Remove visual separators without guessing a country, trunk prefix or post-dial sequence.
///
/// Returns `None` for unsupported symbols, control characters, more than 64 dial
/// characters, misplaced plus signs and an international optional `(0)` prefix.
/// Internal extensions and star/hash service codes retain their original meaning.
#[must_use]
pub fn contact_dial_target(number: &str) -> Option<String> {
    if number.chars().any(char::is_control) {
        return None;
    }
    let without_spaces: String = number.chars().filter(|c| !c.is_whitespace()).collect();
    if (without_spaces.starts_with('+') || without_spaces.starts_with("00"))
        && without_spaces.contains("(0)")
    {
        return None;
    }
    let mut target = String::new();
    for character in without_spaces.chars() {
        match character {
            '0'..='9' | '+' | '*' | '#' => target.push(character),
            '(' | ')' | '-' | '.' | '/' => {}
            _ => return None,
        }
    }
    if target.is_empty()
        || target.len() > 64
        || !target.bytes().any(|c| c.is_ascii_digit())
        || target.char_indices().any(|(i, c)| c == '+' && i != 0)
        || (target.starts_with('+') && target.contains(['*', '#']))
    {
        return None;
    }
    Some(target)
}

/// Conservative comparison key. No country, national trunk prefix or short suffix is guessed.
/// Explicit international `00` and `+` spellings are equivalent for external-length numbers;
/// short extensions and service codes retain their full spelling.
#[must_use]
pub fn contact_match_key(number: &str) -> Option<String> {
    const MIN_EXTERNAL_LENGTH: usize = 7;
    let target = contact_dial_target(number)?;
    if target.len() >= MIN_EXTERNAL_LENGTH
        && target.starts_with("00")
        && target.bytes().all(|byte| byte.is_ascii_digit())
    {
        Some(format!("+{}", &target[2..]))
    } else {
        Some(target)
    }
}

/// Resolve a number only when exactly one authorized source contact owns its key.
/// Duplicate numbers on one contact are allowed; multiple owners produce no identity.
#[must_use]
pub fn resolve_contact(contacts: &[Contact], number: &str) -> Option<ContactReference> {
    resolve_with_key(contacts, number, contact_match_key)
}

/// Comparison under the user's explicit regional preference. Only German external
/// numbers gain a national/international alias; extensions, service codes and all
/// other countries keep conservative exact matching. This never changes dialing.
#[must_use]
pub fn contact_match_key_with_preferences(
    number: &str,
    preferences: &crate::ContactsPreferences,
) -> Option<String> {
    let key = contact_match_key(number)?;
    if preferences.match_german_numbers
        && key.len() >= 7
        && key.starts_with('0')
        && !key.starts_with("00")
        && key.bytes().all(|c| c.is_ascii_digit())
    {
        Some(format!("+49{}", &key[1..]))
    } else {
        Some(key)
    }
}

/// Resolve current authorized contact identity with regional aliases, rejecting
/// ambiguous ownership even when two contacts use different equivalent spellings.
#[must_use]
pub fn resolve_contact_with_preferences(
    contacts: &[Contact],
    number: &str,
    preferences: &crate::ContactsPreferences,
) -> Option<ContactReference> {
    resolve_with_key(contacts, number, |n| {
        contact_match_key_with_preferences(n, preferences)
    })
}

fn resolve_with_key(
    contacts: &[Contact],
    number: &str,
    key_for: impl Fn(&str) -> Option<String>,
) -> Option<ContactReference> {
    let key = key_for(number)?;
    let mut matches = contacts.iter().filter(|contact| {
        contact
            .phones
            .iter()
            .any(|phone| key_for(&phone.number).as_ref() == Some(&key))
    });
    let contact = matches.next()?;
    if matches.any(|other| other.id != contact.id || other.source != contact.source) {
        return None;
    }
    Some(ContactReference {
        id: contact.id.clone(),
        source: contact.source,
        name: contact.name.clone(),
    })
}
