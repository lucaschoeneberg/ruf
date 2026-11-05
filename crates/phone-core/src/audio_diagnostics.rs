//! Memory-only native audio tests and platform authorization contracts.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Built-in PCM ringtone; no external files or downloads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum Ringtone {
    /// Original two-pulse 440/480 Hz ringtone.
    #[default]
    Classic,
    /// Quieter 440/480 Hz pulses with gentler edges.
    Soft,
    /// One short 440/480 Hz pulse.
    Short,
}

/// Explicit native diagnostic, exclusive with calls and ringtone previews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioTestKind {
    /// Capture through the existing baresip source for at most ten seconds.
    Microphone,
    /// Play two seconds through the existing baresip player on the call output.
    Speaker,
}

/// One active test. Never persisted; microphone samples are discarded immediately.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AudioTestSnapshot {
    /// Opaque owner token; stale stop requests cannot stop a newer test.
    pub id: String,
    /// Capture meter or output sample.
    pub kind: AudioTestKind,
    /// Peak amplitude in 0..=1, updated at most ten times per second; zero for speaker tests.
    pub level: f32,
}

/// Microphone authorization, queried independently of SIP and account storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MicrophonePermission {
    /// No user decision; an explicit request may prompt.
    NotDetermined,
    /// Access is restricted by the operating system.
    Restricted,
    /// The user denied access; use OS settings to change the decision.
    Denied,
    /// The operating system currently permits microphone capture.
    Authorized,
    /// This platform has no implemented authorization query; never implies access.
    Unsupported,
}
