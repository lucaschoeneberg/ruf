use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

/// Shared audio path for the native engine and all its SIP accounts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioMode {
    /// Platform microphone and speaker.
    #[default]
    System,
    /// Send a 440 Hz tone and record received audio for an isolated lab call.
    TestTone,
}

/// Persistent, nonsecret audio routing preferences. Empty device IDs follow the OS default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SoundSettings {
    /// Input device ID, or empty for the system default microphone.
    pub input_device: String,
    /// Call output device ID, or empty for the system default speaker.
    pub output_device: String,
    /// Independent ringtone output ID, or empty for the system default output.
    pub ring_device: String,
    /// Play the built-in ringtone for incoming calls (lab test-tone mode stays silent).
    pub ringtone_enabled: bool,
}
impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            input_device: String::new(),
            output_device: String::new(),
            ring_device: String::new(),
            ringtone_enabled: true,
        }
    }
}
impl SoundSettings {
    /// Respect the native 128-byte device fields without truncation or control bytes.
    pub fn validate(&self) -> Result<(), PhoneError> {
        for id in [&self.input_device, &self.output_device, &self.ring_device] {
            if id.len() > 127 || id.chars().any(char::is_control) {
                return Err(PhoneError::new(
                    ErrorCode::InvalidInput,
                    "Audio device IDs must fit 127 UTF-8 bytes without control characters.",
                ));
            }
        }
        Ok(())
    }
}

/// One currently available native audio endpoint; IDs must be treated as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    /// Backend identifier accepted unchanged in SoundSettings.
    pub id: String,
    /// Human-readable platform device name.
    pub name: String,
    /// False when the backend cannot distinguish identically named endpoints.
    pub selectable: bool,
}

/// Current supported input/output endpoints; the system default is an explicit empty-ID choice in the UI.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    /// Input endpoints.
    pub inputs: Vec<AudioDevice>,
    /// Output endpoints, usable for calls and ringtone independently.
    pub outputs: Vec<AudioDevice>,
}

/// SIP/UDP account input. Never serialize this credential-bearing type.
#[derive(Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountConfig {
    /// DNS name or IPv4 registrar, optionally with a nonzero port.
    pub registrar: String,
    /// SIP and authentication username.
    pub username: String,
    /// Secret input; never returned through IPC or written to metadata.
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
    /// Account metadata or the operating-system credential store is unavailable.
    StorageFailure,
    /// Saved changes succeeded but obsolete credential deletion needs retrying.
    CredentialCleanupPending,
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
    /// An answer is queued or sent; the remote ACK is still pending.
    Answering,
    /// The call is established; media requires separate verification.
    Active,
}

/// Password-free description of one SIP account in this application session.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshot {
    /// Stable, nonzero identity within this engine session.
    pub id: u32,
    /// User-visible account name, 1–64 Unicode characters.
    pub name: String,
    /// SIP registrar; validated using AccountConfig rules.
    pub registrar: String,
    /// SIP authentication username, without the password.
    pub username: String,
    /// This account's independent registration state.
    pub registration: RegistrationState,
    /// Safe registration error, or null.
    pub last_error: Option<PhoneError>,
    /// A queued deletion is awaiting the native event loop.
    pub removing: bool,
}

/// Password-free current state, suitable for repeated reads from the frontend.
#[derive(Debug, Clone, Default, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EngineSnapshot {
    /// Registration of the selected outgoing account (compatibility projection).
    pub registration: RegistrationState,
    /// Occupancy of the only call slot.
    pub call: CallState,
    /// Opaque current-call token for guarded actions; null before native allocation or while idle.
    pub call_id: Option<String>,
    /// Last safe error, or null.
    pub last_error: Option<PhoneError>,
    /// All configured accounts, in creation order; never contains passwords.
    pub accounts: Vec<AccountSnapshot>,
    /// Account used for the next outgoing call, or null before configuration.
    pub selected_account_id: Option<u32>,
    /// Account owning the current call, independent of outgoing selection.
    pub call_account_id: Option<u32>,
    /// Actual engine-wide audio backend, null before engine initialization.
    pub audio_mode: Option<AudioMode>,
    /// Audio routing preferences actually applied, or saved defaults before engine startup.
    pub sound_settings: SoundSettings,
    /// A preview or incoming-call ringtone is currently playing through baresip.
    pub ringtone_playing: bool,
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
    /// Select an existing outgoing account while idle, without changing registration.
    pub fn select_account(&mut self, id: u32) -> Result<(), PhoneError> {
        if self.call != CallState::Idle {
            return Err(PhoneError::new(
                ErrorCode::Busy,
                "End the current call before switching accounts.",
            ));
        }
        let account = self
            .accounts
            .iter()
            .find(|a| a.id == id && !a.removing)
            .ok_or_else(|| {
                PhoneError::new(ErrorCode::InvalidInput, "Unknown or removing account.")
            })?;
        self.selected_account_id = Some(account.id);
        self.sync_selected_registration();
        Ok(())
    }

    /// Refresh the legacy registration projection from authoritative per-account state.
    pub fn sync_selected_registration(&mut self) {
        self.registration = self
            .accounts
            .iter()
            .find(|a| Some(a.id) == self.selected_account_id && !a.removing)
            .map_or(RegistrationState::Unconfigured, |a| a.registration);
    }

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
        self.call_account_id = self.selected_account_id;
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
