//! Observed call lifecycle and durable, password-free history contracts.

use crate::{AccountSnapshot, ContactReference, PhoneError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Which side initiated an observed call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CallDirection {
    /// A native incoming offer.
    Incoming,
    /// An outgoing attempt accepted by the native engine.
    Outgoing,
}

/// Actual observed result; a SIP CANCEL does not imply pickup on another device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CallOutcome {
    /// A call is still open in this app session.
    InProgress,
    /// A connected call ended normally.
    Completed,
    /// An incoming offer ended without a local answer or rejection.
    Missed,
    /// An incoming offer was explicitly rejected, or the peer declined an outgoing call.
    Rejected,
    /// The peer was busy, or a second incoming call was rejected because this app was busy.
    Busy,
    /// A local outgoing attempt was cancelled before connection.
    Cancelled,
    /// A call attempt failed before connection.
    Failed,
    /// Engine termination or restart left no known normal end.
    Interrupted,
}

/// One owned native call observation; never contains a credential or native handle.
#[derive(Debug, Clone)]
pub struct ObservedCallEvent {
    /// Unique within this process, including engine recreation and busy offers.
    pub observation_id: u64,
    /// Native UTC observation timestamp in milliseconds since Unix epoch.
    pub at_ms: u64,
    /// The observed lifecycle transition.
    pub kind: ObservedCallKind,
}

/// Transitions delivered in order to a nonblocking observer, independently of UI snapshots.
#[derive(Debug, Clone)]
pub enum ObservedCallKind {
    /// The native engine observed a real incoming/outgoing call.
    Started {
        /// Owning account at start, if available. Password-free source snapshot only.
        account: Option<AccountSnapshot>,
        /// Initiating side.
        direction: CallDirection,
        /// Original selected dial target or sanitized incoming number; null if withheld.
        number: Option<String>,
        /// Sanitized peer display name; never inferred from contact data.
        remote_name: Option<String>,
    },
    /// SIP connection was actually observed.
    Connected,
    /// The native call ended.
    Finished {
        /// Evidence-based outcome.
        outcome: CallOutcome,
        /// Real final SIP status, when supplied and in 100–699; native errno is never a SIP status.
        sip_status: Option<u16>,
        /// Monotonic connected duration, or null when the call never connected.
        duration_ms: Option<u64>,
    },
}

/// Durable call facts plus current contact/account projections for the history view.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// Stable opaque record ID, independent of ephemeral SIP call tokens.
    pub id: String,
    /// Originating account name at call start.
    pub account_name: String,
    /// Original registrar, null if unavailable; used with username for safe account matching.
    pub account_registrar: Option<String>,
    /// Original nonsecret username, null if unavailable. Never a password.
    pub account_username: Option<String>,
    /// Initiating side.
    pub direction: CallDirection,
    /// Original selected target or sanitized peer number; null for unavailable/withheld numbers.
    pub number: Option<String>,
    /// Sanitized SIP name at the time of the call; null when unavailable.
    pub remote_name: Option<String>,
    /// UTC start milliseconds since Unix epoch.
    #[ts(type = "number")]
    pub started_at_ms: u64,
    /// UTC connected timestamp, null before connection.
    #[ts(type = "number | null")]
    pub answered_at_ms: Option<u64>,
    /// Observed UTC end timestamp; null while open or after an unknown crash end.
    #[ts(type = "number | null")]
    pub ended_at_ms: Option<u64>,
    /// Observed monotonic connected duration, null when unknown/not connected.
    #[ts(type = "number | null")]
    pub duration_ms: Option<u64>,
    /// Current or terminal result.
    pub outcome: CallOutcome,
    /// Real SIP status only, null when unknown.
    pub sip_status: Option<u16>,
    /// Current authorized address-book projection. Never persisted to the history database.
    #[serde(default)]
    pub contact: Option<ContactReference>,
    /// Currently registered matching account ID for callback, or null. Never persisted.
    #[serde(default)]
    pub callback_account_id: Option<u32>,
}

/// History direction/result filter.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum HistoryFilter {
    /// All calls.
    #[default]
    All,
    /// Unanswered incoming offers.
    Missed,
    /// All incoming calls, including rejected/busy offers.
    Incoming,
    /// All outgoing attempts.
    Outgoing,
}

/// Validated paginated history query; contact numbers restrict a contact-specific view.
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryQuery {
    /// Search name/number/account text, default empty; at most 200 Unicode characters.
    #[serde(default)]
    pub search: String,
    /// Direction/result filter, default all.
    #[serde(default)]
    pub filter: HistoryFilter,
    /// Zero-based offset, default zero.
    #[serde(default)]
    pub offset: u32,
    /// Page size 1–100; null/missing means 50.
    pub limit: Option<u32>,
    /// Original numbers to restrict to one contact, default empty; at most 100 numbers.
    #[serde(default)]
    pub contact_numbers: Vec<String>,
}

/// Current history page and explicit recording status.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    /// Newest-first matching records.
    pub entries: Vec<HistoryEntry>,
    /// Total number matching this query.
    pub total: u32,
    /// Another page exists after these records.
    pub has_more: bool,
    /// A prior write failed; null when recording is healthy.
    pub recording_error: Option<PhoneError>,
}
