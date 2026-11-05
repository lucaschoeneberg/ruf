# Native audio diagnostics and ringtone preferences

These local Tauri IPC commands use `PhoneError`, not HTTP status codes. They do
not read account metadata or passwords to inspect microphone authorization.

| Command                         | Required input                                        | Result                                                                                                                                                                                                 |
| ------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `get_microphone_permission`     | none                                                  | `MicrophonePermission`; read-only, never prompts or opens an input stream.                                                                                                                             |
| `request_microphone_permission` | none                                                  | `MicrophonePermission`; explicit user action only. Prompts only while undecided; denied/restricted permission must be changed in OS settings.                                                          |
| `start_audio_test`              | `{ kind: "microphone" \| "speaker", device: string }` | Opaque owner token (`string`) for the accepted native test. Empty device follows the system default. Other IDs are unchanged native inventory IDs, at most 127 UTF-8 bytes without control characters. |
| `stop_audio_test`               | `{ id: string }`                                      | No result (`void`). Idempotent for a valid stale token; never stops another test, call audio or an incoming ringtone.                                                                                  |

`MicrophonePermission` is `notDetermined`, `restricted`, `denied`, `authorized`
or `unsupported`. macOS uses AVFoundation authorization metadata independently
of SIP linkage, credential-store state and saved accounts. Reads and explicit
requests run on blocking workers while the application main loop runs. A native
read failure, unknown native status or authorization timeout returns
`nativeFailure`; consumers must display an unknown/error state, never infer
denial. Other platforms currently return `unsupported`; this does not establish
that microphone access is available. Opening a settings page never captures
samples or automatically requests permission.

Tests require the idle call slot and exclusive native test/preview ownership.
Where authorization metadata is supported, the microphone test requires already-authorized
access and never requests it implicitly. When the platform reports `unsupported`,
the explicit test attempts native capture; the audio backend determines whether
access succeeds. A failed/unknown permission read still rejects the test. It captures through the existing baresip `ausrc` backend for at most
ten seconds. The meter is the peak absolute signed-16-bit amplitude, normalized
to 0..1 and updated at most ten times per second. Samples are discarded in the
capture callback: there is no recording, file, upload, browser microphone access
or second audio pipeline. The two-second speaker tone uses the existing baresip
player and the requested call-output device. It is separate from the existing
ringtone preview, which uses the independent ringtone output.

`EngineSnapshot.audioTest` is null while idle or `{ id, kind, level }` for an
active diagnostic. All three nested fields are required. `level` is zero for
speaker tests. `EngineSnapshot.audioTestError` is null or a safe `PhoneError`,
separate from call/registration failures, reset by the next accepted test start.
Both fields are memory-only. Incoming and outgoing calls stop and release test
resources first. Navigation/closing settings must send `stop_audio_test` with
the owned token; engine shutdown also releases all resources. Tokens are opaque
canonical nonzero decimal strings, unique across engine recreation in the
process; clients must not manufacture or numerically interpret them.

Malformed kinds/devices/tokens return `invalidInput`. Concurrent calls/tests or
ringtone previews return `busy`; a real incoming call winning after validation
can yield `nativeFailure` from the repeated native guard. Unlinked builds return
`engineUnavailable`. Missing/ambiguous/unavailable hardware, capture/playback
errors and missing authorization return `nativeFailure`. An accepted start does
not prove that physical speakers are audible or that microphone input contains
speech. Device disappearance during a test stops capture and surfaces a native
failure when the backend reports it; the ten-second bound remains in force.

`SoundSettings` adds `ringtone: "classic" | "soft" | "short"` and
`ringtoneVolume: number` (integer 0..100). Both fields have deserialization
defaults for legacy metadata: `classic` and `100`. The default reproduces the
original 48 kHz 440/480 Hz two-pulse PCM amplitude exactly. `soft` uses quieter
pulses with gentler edges; `short` uses one quarter-second pulse. Zero volume
produces silence; percentages scale ringtone PCM only, without changing system
volume, call audio, speaker-test amplitude, or SIP media. `ringtoneEnabled`
retains its existing incoming-alert meaning. Existing `save_sound_settings`,
`preview_ringtone` and `stop_ringtone_preview` routes remain compatible; previews
use the currently applied tone/volume and the separately supplied output device.

Synthetic native tests use the existing `ausine` source and `aufile` player in
lab mode, never physical microphone capture. Automated evidence covers exact
PCM defaults/scaling, peak normalization, exclusive ownership, stale stop
requests, two-/ten-second limits, incoming preemption and shutdown. Physical
macOS capture/playback and Windows runtime behavior require explicit local
verification; synthetic success does not establish either.
