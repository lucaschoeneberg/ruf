# Ton settings

## Scope and implementation

User confirmed microphone/speaker selection, independent ringtone output, ring
on/off and an audible preview. Volume and a microphone meter are outside scope.
The separate Zed-style settings window and existing row controls remain intact.
SoundPanel.svelte owns the controls, Settings.svelte keeps the draft/baseline,
and existing Rust/ts-rs IPC and versioned metadata persist the preferences.
No dependencies or frontend audio pipeline were added.

macOS now uses pinned upstream CoreAudio; the former AudioUnit backend ignored
explicit device choices. Windows retains WASAPI and uses native friendly names
with endpoint IDs. Inventory reads metadata without opening capture. Duplicate
CoreAudio names are unselectable rather than arbitrarily routing to the first
endpoint. Missing saved selections remain visible for repair. Explicit routing
is rechecked before media starts. Settings changes and preview playback recheck
the native call slot, beyond the frontend/Rust guard.

The original PCM two-pulse ringtone is generated in the adapter and played through
baresip. Preview lasts two seconds and supports stop. Automatic incoming ringing
stops on answer/reject/close/shutdown; developer test-tone sessions stay silent
unless the user explicitly previews. No audio is sent into a SIP call by preview.

## Observed verification

- Core contract regression initially failed because SoundSettings was absent;
  final core suite has seven passing tests, including device byte/control limits.
- Native `sound_smoke` on macOS read four inputs and four outputs, applied an
  explicit selectable output, rejected a missing device without changing the
  applied preferences, and exercised actual playback start, explicit stop and
  automatic finish. It never opened microphone capture. No subjective listening
  or microphone routing test is claimed from callback success.
- The startup regression was observed failing with availability validation
  removed, then passing with it restored: unavailable IDs cannot be saved even
  before an engine exists, no engine is started and no account file is created.
- Saved-account restoration test additionally persists/restores ringtone disabled;
  old version-1 metadata without `sound` still loads with system-default routing
  and ringtone enabled.
- Real Asterisk echo remains 4.04s, RMS5811, 439.8Hz. Multi-account incoming/
  outgoing calls, busy sound/preview guards, independent removal and stale-call
  regression run serially through `bun run test:asterisk`.
- Synthetic browser IPC fixtures passed routing fields, separate draft preview,
  explicit save, dirty draft preservation across polling/navigation, missing and
  ambiguous devices, inventory errors and live-call guards. Captures cover
  900×638 and 740×518. These are UI fixtures, not native SIP/audio evidence.
- The visual reviewer requested clearing a stale success notice after further
  edits. Rendering success only for a clean draft resolved it; the browser check
  confirms fresh save feedback remains and is absent after a new edit. The final
  visual verdict scored this one fix resolved and returned ship at that scope.
- Code review identified Windows header ordering, missing availability checks
  before engine initialization and ambiguous CoreAudio names; all addressed.
  The final option-disable/label correction was independently approved.
- One mechanical design pass returned 14 advisory token-catalog findings and no
  non-advisory findings. Subsequent design hooks reported no deterministic issues.
  No ignore/suppression was added. Existing unrelated token drift was preserved.

`get_audio_devices`, `save_sound_settings`, `preview_ringtone`,
`stop_ringtone_preview`, SoundSettings, AudioDevices and snapshot additions are
specified in docs/api/README.md. New commands are permitted only in settings.

## Checks and limits

Passed: Bun format/version/tests/typecheck/build, generated-type stability,
portable/native workspace Clippy (-D warnings) and tests, native app bundle,
Asterisk smoke, opt-in startup/persistence tests and sound_smoke. Svelte/TypeScript
reported zero errors and warnings. Commands are documented in docs/development.md
and native/README.md; validation logs are ignored local artifacts.

Native macOS playback/inventory were exercised; hardware microphone capture,
duplicate-device hardware, Windows builds/runtime, hosted CI and production
notarization were not verified. Browser screenshots are the visual evidence for
this Ton screen; no native Ton screenshot is claimed. The settings window's
native open/close/reopen behavior was established in the preceding milestone.
