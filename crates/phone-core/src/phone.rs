use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// Audio path selected for the initial local test account.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioMode {
    /// Platform microphone and speaker.
    #[default]
    System,
    /// Send a 440 Hz tone and record received audio for an isolated lab call.
    TestTone,
}

/// Ephemeral SIP/UDP account input. Never serialize this credential-bearing type.
#[derive(Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountConfig {
    /// DNS name or IPv4 registrar, optionally with a nonzero port.
    pub registrar: String,
    /// SIP and authentication username for this first account.
    pub username: String,
    /// In-memory password; never returned through IPC or written to configuration.
    pub password: String,
    /// Audio backend choice; microphone/speaker by default.
    #[serde(default)]
    pub audio_mode: AudioMode,
}

impl fmt::Debug for AccountConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccountConfig")
            .field("registrar", &self.registrar)
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("audio_mode", &self.audio_mode)
            .finish()
    }
}

/// Stable IPC error classification, separate from SIP status codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// A field is invalid or unsupported.
    InvalidInput,
    /// The account has not registered successfully.
    NotRegistered,
    /// Another call or native engine instance occupies the resource.
    Busy,
    /// The operation is invalid in the current call state.
    InvalidState,
    /// The build does not contain native support.
    EngineUnavailable,
    /// Native initialization, queueing or execution failed.
    NativeFailure,
}

/// Safe application error. Native protocol text and credentials are not included.
#[derive(Debug, Clone, Serialize, TS)]
pub struct PhoneError {
    /// Machine-readable error code.
    pub code: ErrorCode,
    /// Safe explanatory message.
    pub message: String,
}

impl PhoneError {
    /// Construct an error from a code and credential-free explanation.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for PhoneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}
impl std::error::Error for PhoneError {}

/// Current account registration state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RegistrationState {
    /// No account is configured.
    #[default]
    Unconfigured,
    /// A registration attempt is pending.
    Registering,
    /// The registrar accepted the account.
    Registered,
    /// Registration failed; safe details are in lastError.
    Failed,
}

/// One-call state for the native milestone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CallState {
    /// No call occupies the slot.
    #[default]
    Idle,
    /// An outgoing request has reserved the slot.
    Calling,
    /// The remote endpoint reports ringing.
    Ringing,
    /// An incoming call awaits a user decision.
    Incoming,
    /// The call is established; media requires separate verification.
    Active,
}

/// Password-free current state, suitable for repeated reads from the frontend.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    /// Account registration state.
    pub registration: RegistrationState,
    /// Occupancy of the only call slot.
    pub call: CallState,
    /// Last safe error, or null.
    pub last_error: Option<PhoneError>,
}

impl AccountConfig {
    /// Validate the deliberately restricted first-milestone account syntax.
    pub fn validate(&self) -> Result<(), PhoneError> {
        let invalid = || {
            PhoneError::new(
                ErrorCode::InvalidInput,
                "Invalid account fields: use a DNS/IPv4 registrar, optional port, and a simple SIP username.",
            )
        };
        let mut parts = self.registrar.split(':');
        let host = parts.next().unwrap_or_default();
        let port = parts.next();
        let valid_host = !host.is_empty()
            && host.len() <= 253
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            });
        if !valid_host
            || self.registrar.len() > 255
            || parts.next().is_some()
            || port.is_some_and(|p| {
                !p.bytes().all(|b| b.is_ascii_digit()) || p.parse::<u16>().map_or(true, |n| n == 0)
            })
        {
            return Err(invalid());
        }
        if self.username.is_empty()
            || self.username.len() > 80
            || !self
                .username
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.+-".contains(&c))
        {
            return Err(invalid());
        }
        if self.password.is_empty() || self.password.len() > 1024 || self.password.contains('\0') {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Password must be nonempty, at most 1024 bytes and contain no NUL byte.",
            ));
        }
        Ok(())
    }
}

impl EngineSnapshot {
    /// Reserve the single outgoing-call slot, rejecting malformed targets first.
    pub fn reserve_dial(&mut self, destination: &str) -> Result<(), PhoneError> {
        if destination.is_empty()
            || destination.len() > 64
            || !destination
                .bytes()
                .all(|c| c.is_ascii_digit() || b"+*#".contains(&c))
        {
            return Err(PhoneError::new(
                ErrorCode::InvalidInput,
                "Dial a number or extension of at most 64 characters; SIP URIs are not supported yet.",
            ));
        }
        if self.registration != RegistrationState::Registered {
            return Err(PhoneError::new(
                ErrorCode::NotRegistered,
                "Register an account before dialing.",
            ));
        }
        if self.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "A call already occupies the call slot.",
            ));
        }
        self.call = CallState::Calling;
        self.last_error = None;
        Ok(())
    }

    /// Reserve account replacement only when no call is in progress.
    pub fn reserve_registration(&mut self) -> Result<(), PhoneError> {
        if self.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before replacing the account.",
            ));
        }
        self.registration = RegistrationState::Registering;
        self.last_error = None;
        Ok(())
    }
}
