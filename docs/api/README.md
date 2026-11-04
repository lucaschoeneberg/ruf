# Desktop IPC API

No HTTP server or HTTP status-code mapping is exposed. Frontend wrappers reject
browser previews. Rust phone-core and ts-rs own the DTOs; `bun run types:generate`
updates the TypeScript contract. All success responses are password-free.

## Windows and permissions

`main` reads app/engine state, opens settings, selects the outgoing account and
controls calls. `settings` reads app/engine state and saves/removes accounts.
Explicit generated command permissions apply separately. Neither window has
arbitrary window creation, filesystem, shell or keychain access. `open_settings`
is an async Rust command; it creates one `settings` window (900 × 660, minimum
740 × 540), or restores/focuses the existing window. Closing it preserves the
shared Rust session, registrations and any call. The main phone stays compact.

## Commands

| Command               | Input                                                                          | Result / behavior                                                                                                                                                                   |
| --------------------- | ------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_app_info`        | none                                                                           | `AppInfo {version, telephony: "linked" \| "notLinked"}`. Compiled availability only; does not initialize SIP or read credentials.                                                   |
| `open_settings`       | optional `{section: "accounts" \| "audio" \| "general" \| "about"}`, main only | `null` on success; opens/focuses the singleton settings window.                                                                                                                     |
| `get_engine_snapshot` | none                                                                           | `EngineSnapshot`. Startup restores saved accounts in Rust independently of windows. Reads share the same managed session and can complete initialization if needed.                 |
| `save_account`        | `{id: number \| null, name: string, account: AccountConfig}`                   | Null/missing ID creates; existing ID updates. Saves metadata/keychain credentials and waits for native configuration acceptance. Actual registration remains asynchronous.          |
| `select_account`      | `{id: number}`                                                                 | Persists outgoing selection without unregistering other accounts. Requires idle call state.                                                                                         |
| `remove_account`      | `{id: number}`                                                                 | Requires idle state. Waits for native removal, commits metadata deletion and removes the keychain secret. A native incoming-call rejection leaves saved data unchanged.             |
| `configure_account`   | `{account: AccountConfig}`                                                     | Compatibility command, settings only: save/replace the selected account, using username as its name. **Now durable**, unlike the earlier ephemeral API. New UI uses `save_account`. |
| `dial`                | `{destination: string}`                                                        | Reserves the single call slot on the selected, registered account. Number/extension: 1–64 ASCII digits or `+ * #`; arbitrary SIP URIs are unsupported.                              |
| `answer`              | required `{callId: string}`                                                    | Accepts the current incoming call on its own account, independent of outgoing selection.                                                                                            |
| `hangup`              | required `{callId: string}`                                                    | Rejects/ends the current call; native completion follows through state.                                                                                                             |

All except `get_app_info` can reject with `PhoneError`. Tauri separately rejects
malformed arguments, missing required fields, unknown account fields/enum values,
runtime failures or denied permissions. Queue acceptance does not imply SIP/media
success: poll real registration/call state. Account save/remove wait for native
application/rejection, rather than abandoning a delayed mutation after a timeout.
Native shutdown causes those waits to reject.

## AccountConfig (credential-bearing input only)

| Field       | Required / default | Validation and meaning                                                                                                                                                                                                |
| ----------- | ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `registrar` | required string    | DNS/IPv4 spelling, optional `:port`. <=255 bytes total; host <=253, labels 1–63 ASCII alphanumeric/hyphen, no leading/trailing hyphen. Port is ASCII digits, nonzero u16. No URI, userinfo, parameters, path or IPv6. |
| `username`  | required string    | 1–80 ASCII letters/digits or `_ . + -`. Also the authentication username.                                                                                                                                             |
| `password`  | required string    | 1–1024 UTF-8 bytes, no NUL. **Only on update**, an empty string retrieves/keeps the existing secret. New accounts require a password. Never returned to webviews, serialized in metadata or logged; Debug redacts it. |
| `audioMode` | optional, `system` | Engine-wide `system` microphone/speaker, or `testTone` (440 Hz input, received.wav output in app cache). Applies to all accounts; cannot change while the engine exists. Restart to choose another mode.              |

`name` is a trimmed user-visible name of 1–64 Unicode characters, without control
characters. At most eight accounts; duplicate registrar/username combinations are
rejected (registrar comparison is case insensitive). IDs are nonzero u32 session
identities: they can change after restart. The persisted selection uses an opaque
credential key and is mapped back to the restored ID. Unknown/removing IDs reject.
Save, removal and outgoing selection require an idle call. The native event-loop
thread rechecks mutations against incoming calls, preserving the old account
metadata and UA when configuration is rejected.

Currently SIP/UDP, G.711 and one global call are supported. No silent fallback from
encrypted transport. Use the development build in an isolated test network.
Test-tone recordings overwrite the previous received.wav and remain in the cache;
do not use this mode for private calls. The upstream path limit is 127 UTF-8 bytes.

## EngineSnapshot (required output fields)

```ts
{
  registration: "unconfigured" | "registering" | "registered" | "failed";
  call: "idle" | "calling" | "ringing" | "incoming" | "answering" | "active";
  lastError: PhoneError | null;
  accounts: AccountSnapshot[];
  selectedAccountId: number | null;
  callAccountId: number | null;
  callId: string | null;
  audioMode: "system" | "testTone" | null;
  soundSettings: SoundSettings;
  ringtonePlaying: boolean;
}
```

`registration` projects the selected outgoing account; other accounts retain their
own status. `callAccountId` identifies the UA actually owning the call, including
incoming calls to another account. `audioMode` is the real shared native mode,
null before engine initialization. Webview reloads/settings windows read this
value and do not infer it from draft defaults. `active` means SIP established;
[the echo test](../../tests/interop/asterisk/README.md) separately verifies media.

`AccountSnapshot` contains required `id`, `name`, `registrar`, `username`,
`registration`, nullable `lastError`, and boolean `removing`. Registration failures
are per account. Missing/inaccessible saved credentials restore a visibly failed
account that can be repaired by supplying a password. No secret is read back.

Answer/hangup capture an internal call generation; delayed actions for call A
cannot affect replacement call B. The opaque `callId` is a session-local generation token, not a SIP Call-ID. It is null while idle and can be null while outgoing queueing is pending. Both call-control commands now require the displayed token (an intentional MVP IPC change); empty/stale tokens reject with `invalidState`. Duplicate answers reject. `answering` means the native answer is pending an ACK; only the native established event reports `active`. Engine lifetime
belongs to the application, rather than either webview.

## Persistence and credential cleanup

`accounts.json` in Tauri's application data directory contains schema version 1,
nonsecret account metadata, opaque credential keys, selected key, audio mode and
`pendingDeletions` (default empty). It contains **no passwords**. Same-directory
file replacement commits metadata. Unsupported/corrupt data is rejected rather
than overwritten. macOS Keychain Services and Windows Credential Manager are used
through keyring; other operating systems have no desktop secret-store backend in
this milestone.

New credential keys are journaled before writing secrets. On replacement/removal,
obsolete keys remain in `pendingDeletions` until deletion succeeds; cleanup retries
at application load and successful account mutations. Interrupted saves therefore
retain a cleanup reference. A cleanup failure returns a snapshot with
`credentialCleanupPending`: the account change succeeded, but a prior secret still
needs deletion. Unlock/allow the credential store and restart to retry cleanup.
Storage access failures reject with `storageFailure`; they are not reported as
successful saves. Metadata is not encrypted: server, username and account names
are ordinary local application data.

## PhoneError

| Code                       | Meaning                                                                                                                     |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `invalidInput`             | Invalid fields, unsupported mode/path, unknown ID, duplicate identity, account limit or changing the running audio backend. |
| `notRegistered`            | The selected account has no successful registration.                                                                        |
| `busy`                     | Call slot occupied, native singleton held, account mutation pending, or native call guard rejected configuration/removal.   |
| `invalidState`             | No engine/incoming call/call to end, or stale call generation.                                                              |
| `engineUnavailable`        | Native support absent/stopped.                                                                                              |
| `nativeFailure`            | Initialization, queueing, SIP/native execution, state or shutdown failure.                                                  |
| `storageFailure`           | Account metadata or system credential-store access failed.                                                                  |
| `credentialCleanupPending` | Account change committed; obsolete secret deletion is durably queued for retry.                                             |

`message` is a safe explanation, sometimes with a numeric native/SIP code; never
raw protocol text or credentials. Codes are application classifications, not SIP
response codes. The frontend translates known codes to German recovery guidance.

## Sound commands (settings window only)

| Command                 | Input                             | Result and errors                                                                                                                                                                                                                                                                                                               |
| ----------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_audio_devices`     | none                              | `AudioDevices {inputs: AudioDevice[], outputs: AudioDevice[]}`. Reads current native metadata without starting SIP, reading credentials or opening capture. `engineUnavailable` without native support; `nativeFailure` on enumeration errors.                                                                                  |
| `save_sound_settings`   | required `{sound: SoundSettings}` | `EngineSnapshot` after durable save and native acknowledgment when an engine exists. Requires idle state; validates live devices even before engine creation. `invalidInput` for invalid/missing/ambiguous devices, `busy` during calls, `storageFailure` on persistence errors, `nativeFailure` for native application errors. |
| `preview_ringtone`      | required `{device: string}`       | `EngineSnapshot`. Plays a two-second native ringtone on that draft output without saving it. Starts the engine if necessary, locking its audio mode for the session. Requires idle state and valid device. Does not open a microphone or send audio to SIP.                                                                     |
| `stop_ringtone_preview` | none                              | `EngineSnapshot`. Stops a preview only; never silences an actual incoming-call alert. Safe before engine initialization.                                                                                                                                                                                                        |

`SoundSettings` is a required-field object: `inputDevice`, `outputDevice`,
`ringDevice` (strings), and `ringtoneEnabled` (boolean). Unknown fields reject.
Empty device IDs select the OS default independently; the native alias `default`
is also accepted. Nonempty IDs are opaque, <=127 UTF-8 bytes without control
characters, and must match a currently selectable endpoint of the right direction.
Microphone/call-output choices apply to subsequent system-audio calls; test-tone
mode continues to use its synthetic source/recording path. Default preferences:
three empty IDs and ringtone enabled.

`AudioDevice` contains required `id`, `name`, and `selectable` fields. macOS
CoreAudio currently routes by device display name; if several physical devices
share that name, the entry is unselectable and the UI offers renaming or the
system default. Windows uses WASAPI endpoint IDs and friendly names. Device
metadata is refreshed on request rather than polled every 500 ms. Unplugged or
ambiguous saved selections remain visible; save and media startup recheck them
rather than silently choosing another device.

`EngineSnapshot.soundSettings` reflects applied routing (or saved preferences
before engine creation); `ringtonePlaying` reports actual native playback,
including incoming alerts. A preview is stopped when a call starts, and an
incoming ringtone stops on answer/reject/close/shutdown. Native call guards also
reject routing changes or previews that race an incoming call.

Version-1 metadata adds optional `sound` with defaults when absent, preserving
older account files. It is ordinary nonsecret local data; keychain handling is
unchanged. Sound preferences are not written to webview storage.

## Automatic audio jitter buffering

The engine uses baresip's adaptive audio jitter buffer with a 40–200 ms delay
range. This is an internal default, not a new setting/DTO field. It applies at
engine startup and can grow under jitter; it does not change device selection,
packet duration, SIP NAT handling or native command behavior.

## Automatic SIP/UDP address adaptation

The native engine learns a translated **local** SIP address from successful,
transaction-matched REGISTER responses and re-registers with that address before
reporting `registered`. The previous Contact is removed in the same update.
This is automatic and adds no command arguments, settings fields or DTOs.
Socket/RTP binding and the PBX's remote dialog target are preserved.

Mappings prefer an exact local-socket and peer-IP+port match. For an incoming PBX request on another source port, an unexpired mapping for the same local socket and peer IP is used only when all candidates agree. Conflicting candidates and unrelated peers retain the original local address. Invalid, failed,
unmatched or nonnumeric observations are ignored. Address instability beyond two
immediate corrections, or exhausted bounded mapping storage, uses the existing
`nativeFailure` registration/error path with normal failure backoff. A subsequent
fresh registration cycle can recover. This does not add encryption, SIP/TCP,
media ICE/TURN or support for every NAT/proxy topology. The internal C contracts
and regression evidence are documented in [the native extension](../../native/patches/README.md).

## Desktop lifecycle, events and preferences

The native process restores accounts and monitors state without webview timers.
Initial macOS keychain restoration suppresses keychain UI; unavailable credentials
produce failed accounts that can be repaired interactively. Closing the phone hides
it; explicit Quit confirms an occupied call and shuts down the engine. An explicit
manual second launch opens the existing phone, whereas `--background` stays hidden.
Login start is user-session startup, not a system daemon or sleep/offline service.

Rust emits `phone-state-changed` with the full password-free `EngineSnapshot` to
`main` and `settings` when observed state changes. Both windows subscribe and retain
initial/visible fallback reads. Native ring audio and registration do not depend on
those listeners. Rust emits `settings-section` (one of the four section strings)
only to the settings window; existing drafts still require an explicit discard.
The optional `open_settings.section` defaults to `accounts`; invalid sections
reject with `invalidInput`. Existing-window focus/build failures use `nativeFailure`.

Settings-only commands:

| Command                 | Required input                                               | Output / errors                                                                                                                                                                      |
| ----------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `get_desktop_settings`  | none                                                         | `DesktopSettings`; `storageFailure` or `nativeFailure` if the store/OS state is unavailable.                                                                                         |
| `save_desktop_settings` | `{autostartEnabled: boolean, notificationsEnabled: boolean}` | Actual `DesktopSettings` after saving. `invalidInput` when enabling login start in unsupported locations; `storageFailure` on persistence errors; `nativeFailure` for OS operations. |

`DesktopSettings` has required boolean `autostartEnabled`, `autostartAvailable`,
`notificationsEnabled`, and nullable boolean `notificationsGranted`. Login-start
registration comes from the official OS integration plugin. On macOS it checks
the LaunchAgent entry; an external OS background-item policy can still disable
execution, which this API cannot detect. Release installations in `/Applications/SIP.app`,
`~/Applications/SIP.app`, or the standard Windows per-user/Program Files `SIP`
locations can register; debug/temporary bundles cannot register or alter an existing
installed app's login entry. First successful account registration enables login
start once. Explicitly disabling it is respected on subsequent starts.

The saved schema remains version 1 with a defaulted `desktop` object containing
`notificationsEnabled` (default true) and `autostartConfigured` (default false).
Existing metadata without this object loads unchanged; credentials remain in the
system store. Preference updates serialize the OS change with metadata commits
and attempt rollback if persistence fails.

`notificationsGranted` is **null on desktop**: the current plugin cannot query real
OS authorization, even though its permission API reports Granted unconditionally.
The UI does not claim authorization. OS notification settings and Focus modes may
suppress banners; native ringing and tray controls remain available. Incoming calls
produce at most one passive notification per observed call, without focusing a
window. There are no stale notification call buttons. Useful native call actions
carry the displayed `callId`, including when a native menu remains open across calls.

Network address/DNS recovery runs on the SIP thread every five seconds. Address
changes are reconciled before a quiet poll rebinds/re-registers transports. All
mutations are deferred while a call occupies the slot; failed resets retry with a
capped 60-second backoff. It does not alter Tailscale or OS network configuration.

## Responsive account saves and protected Settings exit

`save_account` and `configure_account` retain their existing input/output DTOs.
They journal a candidate key under the session lock, perform foreground credential
authorization outside that lock, then revalidate and commit under the lock.
Snapshot delivery, native call controls and shutdown continue while authorization
is pending. A call or account/audio change during authorization can reject the
commit with `busy` or `invalidState`; the existing account is preserved and orphan
cleanup remains durably journaled. Background credential reads/cleanup are
noninteractive and never wait for a foreground authorization prompt.

The Settings window preserves failed-save drafts, including an entered password
for an immediate retry. Closing/discarding clears that draft from the component;
passwords are never returned through IPC or saved in browser storage. Removing an
account externally does not silently convert its dirty edit into a new account.
Desktop preference load failures do not block account or sound settings.

Settings-only commands `settings_ready` and `quit_app` take no arguments and
return null. `settings_ready` marks installed close/quit handlers ready before the
form is editable. Rust routes an explicit app Quit to the ready Settings window
with `settings-quit-requested` (null payload), making it visible even when minimized.
The existing discard panel handles drafts; busy saves must finish first. Only
then does `quit_app` continue the native call confirmation and shutdown path.
It never bypasses active-call confirmation. Clean or explicitly discarded native
window closes use the scoped `core:window:allow-destroy` permission; no other
window gains this permission. Settings destruction clears guard readiness.
