# Desktop IPC API

The settings-only company links and compiled software-inventory commands are
documented in [About-page links and SBOM](about.md).

Native macOS phonebook commands and generated DTOs are documented in
[Contacts](contacts.md). Local call-history commands, queries and callbacks are
documented in [History](history.md). `EngineSnapshot.callerContact` is an authorized
current address-book projection; `historyError` is a nullable sticky `PhoneError`
for history recording failures. Both fields are required nullable output fields.

No HTTP server or HTTP status-code mapping is exposed. Frontend wrappers reject
browser previews. Rust phone-core and ts-rs own the DTOs; `bun run types:generate`
updates the TypeScript contract. All success responses are password-free.

Additional settings contracts: [notifications](notifications.md),
[anonymous diagnostics](diagnostics.md), [permission boundaries](permissions.md),
[local preferences](preferences.md), [native audio diagnostics](audio-diagnostics.md)
and [account activation](account-activation.md).

Native Rust controls and per-dialog DTO additions: [call controls and consultation](call-controls.md).

## Windows and permissions

`main` reads app/engine state, opens settings, selects the outgoing account and
controls calls. `settings` reads app/engine state and saves/removes accounts.
Explicit generated command permissions apply separately. Neither window has
arbitrary window creation, filesystem, shell or keychain access. `open_settings`
is an async Rust command; it creates one `settings` window (900 × 660, minimum
740 × 540), or restores/focuses the existing window. Closing it preserves the
shared Rust session, registrations and any call. The main phone stays compact.

## Commands

| Command               | Input                                                                                                                                         | Result / behavior                                                                                                                                                                                                                        |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_app_info`        | none                                                                                                                                          | `AppInfo {version, telephony: "linked" \| "notLinked"}`. Compiled availability only; does not initialize SIP or read credentials.                                                                                                        |
| `open_settings`       | optional `{section: "accounts" \| "audio" \| "general" \| "contacts" \| "about" \| "notifications" \| "privacy" \| "diagnostics"}`, main only | `null` on success; opens/focuses the singleton settings window.                                                                                                                                                                          |
| `get_engine_snapshot` | none                                                                                                                                          | `EngineSnapshot`. Startup restores saved accounts in Rust independently of windows. Reads share the same managed session and can complete initialization if needed.                                                                      |
| `save_account`        | `{id: number \| null, name: string, account: AccountConfig}`                                                                                  | Null/missing ID creates; existing ID updates. Saves metadata/keychain credentials and waits for native configuration acceptance. Actual registration remains asynchronous.                                                               |
| `reconnect_account`   | required `{id: number}`                                                                                                                       | Settings only: explicitly authorize reading the saved password and reapply the existing account while idle. Preserves metadata, credentials and drafts; registration is asynchronous. See [credential recovery](credential-recovery.md). |
| `select_account`      | `{id: number}`                                                                                                                                | Persists outgoing selection without unregistering other accounts. Requires idle call state.                                                                                                                                              |
| `remove_account`      | `{id: number}`                                                                                                                                | Requires idle state. Waits for native removal, commits metadata deletion and removes the keychain secret. A native incoming-call rejection leaves saved data unchanged.                                                                  |
| `configure_account`   | `{account: AccountConfig}`                                                                                                                    | Compatibility command, settings only: save/replace the selected account, using username as its name. **Now durable**, unlike the earlier ephemeral API. New UI uses `save_account`.                                                      |
| `dial`                | `{destination: string}`                                                                                                                       | Reserves the single call slot on the selected, registered account. Number/extension: 1–64 ASCII digits or `+ * #`; arbitrary SIP URIs are unsupported.                                                                                   |
| `answer`              | required `{callId: string}`                                                                                                                   | Accepts the current incoming call on its own account, independent of outgoing selection.                                                                                                                                                 |
| `reject`              | required `{callId: string}`                                                                                                                   | Rejects only an unanswered incoming call; stale or already answered calls return `invalidState`. Main window only.                                                                                                                       |
| `hangup`              | required `{callId: string}`                                                                                                                   | Rejects/ends the current call; native completion follows through state.                                                                                                                                                                  |

`reject` is a main-window command with required `{callId: string}` input and an
`EngineSnapshot` result. It rejects only an offered incoming call. Empty/stale
tokens and calls already answering or established reject with `invalidState`;
the native queue rechecks the offer. Missing/malformed arguments are rejected by
Tauri. Absent/stopped native support or queue failures use the existing
`engineUnavailable` / `nativeFailure` errors. No optional input fields exist.

All except `get_app_info` can reject with `PhoneError`. Tauri separately rejects
malformed arguments, missing required fields, unknown account fields/enum values,
runtime failures or denied permissions. Queue acceptance does not imply SIP/media
success: poll real registration/call state. Account save/remove wait for native
application/rejection, rather than abandoning a delayed mutation after a timeout.
Native shutdown causes those waits to reject.
Native removal/replacement acceptance does not wait for the registrar to confirm
unregistration. The retired account finishes its pending REGISTER (including a
possible timeout) before unregistering a confirmed binding. Late responses from
that retired account cannot alter a replacement account's snapshot. Command
inputs, output DTOs and error codes are unchanged.

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
  revision: number;
  registration: "unconfigured" | "registering" | "registered" | "failed";
  call: "idle" | "calling" | "ringing" | "earlyMedia" | "incoming" | "answering" | "active" | "held";
  lastError: PhoneError | null;
  accounts: AccountSnapshot[];
  selectedAccountId: number | null;
  callAccountId: number | null;
  callId: string | null;
  callConnectedAtMs: number | null;
  callEvents: CallEvent[];
  callEventsDropped: number;
  caller: { name: string | null; number: string | null } | null;
  callerContact: ContactReference | null;
  historyError: PhoneError | null;
  audioMode: "system" | "testTone" | null;
  soundSettings: SoundSettings;
  ringtonePlaying: boolean;
  audioTest: AudioTestSnapshot | null;
  audioTestError: PhoneError | null;
}
```

`registration` projects the selected outgoing account; other accounts retain their
own status. `callAccountId` identifies the UA actually owning the call, including
incoming calls to another account. `audioMode` is the real shared native mode,
null before engine initialization. Webview reloads/settings windows read this
value and do not infer it from draft defaults. `active` means SIP established;
[the echo test](../../tests/interop/asterisk/README.md) separately verifies media.

Required ordering/event/timing fields, early media/hold, SIP errors and explicit
REFER rejection are detailed in [SIP events and state](sip-events.md). These new
output fields and enum values intentionally extend the generated IPC contract.

`caller` is a required nullable `CallerIdentity`. Its required nullable `name` and
`number` fields are the bounded, plain-text display name and URI user from the
incoming SIP From header, not a verified identity. Only incoming calls supply it;
it remains through `answering`/`active`/`held` and clears on idle/new outgoing calls.
Display names fit 128 UTF-8 bytes, URI users 80 bytes, truncated at UTF-8 boundaries;
malformed or control/bidi-containing fields are omitted. SIP URI hosts/passwords and
P-Asserted-Identity are never used as the displayed number. Anonymous/withheld
From values or Privacy `id` suppress the number. The UI shows name and number,
number alone, or “Unbekannter Anrufer”. No caller data is added to diagnostics
or account metadata. The authorized `callerContact` address-book projection is
documented in [contacts](contacts.md); `historyError` remains separate from SIP
and registration errors as described in [history](history.md).

`AccountSnapshot` contains required `id`, `name`, `registrar`, `username`,
`registration`, nullable `lastError`, boolean `removing`, boolean `enabled`,
boolean `enabledChanging` and nullable `registeredAtMs`. Activation and confirmed
registration timestamps are documented in [account activation](account-activation.md).
Registration failures
are per account. Missing/inaccessible saved credentials restore a visibly failed
account that can be repaired by supplying a password. No secret is read back.

Answer/reject/hangup capture an internal call generation; delayed actions for call A
cannot affect replacement call B. The opaque `callId` is a session-local generation token, not a SIP Call-ID. It is null while idle and reserved before a successful outgoing attempt is queued. All three call-control commands require the displayed token; empty/stale tokens reject with `invalidState`. Duplicate answers reject. `answering` means the native answer is pending an ACK; only the native established event reports `active` (or `held` when already remotely held). Engine lifetime
belongs to the application, rather than either webview.

All incoming “Ablehnen” controls in the main window, tray/application menu and
native notification use the same guarded rejection operation. A delayed reject
cannot end a call already accepted by another surface. “Auflegen” uses the same
general termination operation in every non-idle state, including `answering`.
Duplicate hangups already accepted into the native queue for the same closed
generation are harmless; fresh calls still require their own token. An answer
and a hangup can race: native execution order determines whether an answer is
sent before termination, and termination releases the slot without restarting.
A queued answer that runs after the same call already closed becomes an
`invalidState` stale-action warning, rather than a misleading `nativeFailure`.

## Persistence and credential cleanup

`accounts.json` in Tauri's application data directory contains schema version 2,
nonsecret account metadata, activation flags, local preferences, opaque credential
keys, selected key, audio mode and `pendingDeletions` (default empty). Version 1
loads with compatibility defaults and is backed up before the first replacement;
see [preference migration](preferences.md). It contains **no passwords**. Same-directory
file replacement commits metadata. Unsupported/corrupt data is rejected rather
than overwritten. macOS Keychain Services and Windows Credential Manager are used
through keyring; other operating systems have no desktop secret-store backend in
this milestone.

Persisted account names, registrar/username syntax, duplicate identities and
credential keys are validated before engine initialization or credential access.
A non-null selected key must refer to a saved account; missing legacy optional
fields retain their existing defaults. Invalid metadata returns `storageFailure`
and remains unchanged. Session writes and cleanup also check the durable metadata
against the state they are replacing. A mismatch after a failed rollback returns
`storageFailure` and preserves the newer file and its referenced credentials;
stale in-memory preferences cannot silently overwrite a newer saved account.

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

Saved-password access failures now use `credentialAccessRequired`,
`credentialMissing` or `credentialFailure`, independently of SIP errors and
application-data `storageFailure`. Full command semantics and recovery guidance
are documented in [saved credential recovery](credential-recovery.md).

| Code                       | Meaning                                                                                                                     |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `invalidInput`             | Invalid fields, unsupported mode/path, unknown ID, duplicate identity, account limit or changing the running audio backend. |
| `notRegistered`            | The selected account has no successful registration.                                                                        |
| `busy`                     | Call slot occupied, native singleton held, account mutation pending, or native call guard rejected configuration/removal.   |
| `invalidState`             | No engine/incoming call/call to end, or stale call generation.                                                              |
| `engineUnavailable`        | Native support absent/stopped.                                                                                              |
| `nativeFailure`            | Initialization, queueing, SIP/native execution, state or shutdown failure.                                                  |
| `storageFailure`           | Settings or application-data storage access failed.                                                                         |
| `credentialCleanupPending` | Account change committed; obsolete secret deletion is durably queued for retry.                                             |

`message` is a safe explanation, sometimes with a numeric native/SIP code; never
raw protocol text or credentials. Codes are application classifications, not SIP
response codes. Required nullable `sipStatus` contains an actual final SIP response
only; native errors leave it null. Additional `sip*` classifications are defined
in [SIP events and state](sip-events.md#phoneerror). The frontend translates known
codes to German recovery guidance.

## Sound commands (settings window only)

| Command                 | Input                             | Result and errors                                                                                                                                                                                                                                                                                                               |
| ----------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_audio_devices`     | none                              | `AudioDevices {inputs: AudioDevice[], outputs: AudioDevice[]}`. Reads current native metadata without starting SIP, reading credentials or opening capture. `engineUnavailable` without native support; `nativeFailure` on enumeration errors.                                                                                  |
| `save_sound_settings`   | required `{sound: SoundSettings}` | `EngineSnapshot` after durable save and native acknowledgment when an engine exists. Requires idle state; validates live devices even before engine creation. `invalidInput` for invalid/missing/ambiguous devices, `busy` during calls, `storageFailure` on persistence errors, `nativeFailure` for native application errors. |
| `preview_ringtone`      | required `{device: string}`       | `EngineSnapshot`. Plays a two-second native ringtone on that draft output without saving it. Starts the engine if necessary, locking its audio mode for the session. Requires idle state and valid device. Does not open a microphone or send audio to SIP.                                                                     |
| `stop_ringtone_preview` | none                              | `EngineSnapshot`. Stops a preview only; never silences an actual incoming-call alert. Safe before engine initialization.                                                                                                                                                                                                        |

`SoundSettings` requires `inputDevice`, `outputDevice`, `ringDevice` (strings)
and `ringtoneEnabled` (boolean). It also returns `ringtone` (`classic`, `soft`,
`short`) and integer `ringtoneVolume` (0–100); input omission defaults them to
`classic`/`100` for compatibility. Unknown fields reject. See
[audio diagnostics and ringtones](audio-diagnostics.md) for validation and tests.
Empty device IDs select the OS default independently; the native alias `default`
is also accepted. Nonempty IDs are opaque, <=127 UTF-8 bytes without control
characters, and must match a currently selectable endpoint of the right direction.
Microphone/call-output choices apply to subsequent system-audio calls; test-tone
mode continues to use its synthetic source/recording path. Default preferences:
three empty IDs, ringtone enabled, classic tone and volume 100.

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

The optional saved `sound` object retains defaults when absent in legacy or
schema-v2 metadata. It is ordinary nonsecret local data; keychain handling is
unchanged. Sound preferences are not written to webview storage. Ringtone preview
uses the applied tone/volume and the independently supplied draft output device.

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
initial/visible fallback reads. Main-thread delivery keeps one pending snapshot,
replacing it with the latest observation while the UI queue is blocked; this event
is a current-state projection, not a complete transition log. If a read fails after
a successful observation, the observer retains the last known call, account and
notification preference state and attaches the read error as `lastError`; it does
not infer call termination or disabled notifications. A later successful read
replaces that retained state. Native ring audio and registration do not depend on
those listeners. Rust emits `settings-section` with one of the eight IDs:
`general`, `accounts`, `audio`, `notifications`, `contacts`, `privacy`,
`diagnostics` or `about`. Only the settings window receives this event;
existing drafts still require an explicit discard.
The optional `open_settings.section` defaults to `accounts`; invalid sections
reject with `invalidInput`. Existing-window focus/build failures use `nativeFailure`.

Desktop preference commands (`get_desktop_settings` is available to main and
settings; the request/save commands are settings-only):

| Command                           | Required input                                               | Output / errors                                                                                                                                                                                                                                                                                                    |
| --------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `get_desktop_settings`            | none                                                         | `DesktopSettings`; `storageFailure` or `nativeFailure` if the store/OS state is unavailable.                                                                                                                                                                                                                       |
| `request_notification_permission` | none                                                         | `DesktopSettings` after an explicit foreground authorization action. Only `notDetermined` can present the first OS permission prompt; denial/unsupported contexts return their canonical status. Native bridge failures use `nativeFailure`; persistence read failures use `storageFailure`. Settings window only. |
| `save_desktop_settings`           | `{autostartEnabled: boolean, notificationsEnabled: boolean}` | Actual `DesktopSettings` after saving. `invalidInput` when enabling login start in unsupported locations; `storageFailure` on persistence errors; `nativeFailure` for OS operations.                                                                                                                               |

`DesktopSettings` has required booleans `autostartEnabled`, `autostartAvailable`,
`notificationsEnabled`, required enum `notificationPermission`, and required nullable
boolean `notificationsGranted`. The enum preserves the actual OS decision:

| `notificationPermission` | `notificationsGranted` | Meaning / foreground recovery                                                                                                                          |
| ------------------------ | ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `notDetermined`          | false                  | No decision yet. “Mitteilungen erlauben” explicitly requests permission even when notices are already enabled by default and no preferences are dirty. |
| `denied`                 | false                  | OS/user policy disabled notices. Change the permission in system settings; another request cannot promise a new prompt.                                |
| `authorized`             | true                   | Delivery is authorized, subject to OS display/Focus settings.                                                                                          |
| `unsupported`            | null                   | This platform or launch context cannot deliver native notices; do not offer a permission prompt.                                                       |

`notificationsGranted` remains a compatibility projection; clients must use
`notificationPermission` to choose permission actions. Query failures reject rather
than inventing a denied/unsupported state. Settings re-read authorization when the
window regains focus, without prompting or discarding unsaved preference drafts.
Login-start registration comes from the official OS integration plugin. On macOS it checks
the LaunchAgent entry; an external OS background-item policy can still disable
execution, which this API cannot detect. Release installations in `/Applications/Ruf.app`,
`~/Applications/Ruf.app`, or the standard Windows per-user/Program Files `Ruf`
locations can register; debug/temporary bundles cannot register or alter an existing
installed app's login entry. First successful account registration enables login
start once. Explicitly disabling it is respected on subsequent starts. Automatic
first-registration login-start setup defers while a foreground preference or
notification authorization operation is pending; the observer continues delivering
phone state and retries the deferred setup afterward.

Login launches use `--autostart` and the saved `startHidden` preference (default
true); manual launches show the phone. Explicit `--background` and notification
activation remain hidden. Existing configured/enabled login entries have their
arguments refreshed independently of account registration; disabled entries stay
disabled. This preference does not keep an offline or sleeping system reachable.

The saved schema is version 2; the existing defaulted `desktop` object retains
`notificationsEnabled` (default true) and `autostartConfigured` (default false).
The separate `preferences` object owns login presentation, caller privacy,
contact display and history policy. Legacy metadata loads with defaults and an
original-byte backup precedes its first v2 replacement; see [migration](preferences.md).
Credentials remain in the system store. Preference updates serialize the OS
change with metadata commits and attempt rollback if persistence fails.

Reading settings, receiving calls and saving the app notification switch never
request permission. Only the explicit “Mitteilungen erlauben” action can present
the first OS prompt while the decision is `notDetermined`. The independent
`get_notification_permission` command remains usable despite account metadata or
keychain errors; see [notification commands](notifications.md). A prior
OS denial must be changed in system settings. Unsupported contexts include unbundled
macOS binaries and temporary Windows builds. Focus/display policy can still suppress
authorized banners; native ringing remains independent.

Incoming calls produce a silent native banner with caller name/number and the
receiving account, plus **Annehmen** / **Ablehnen**. No window is focused by
arrival or either button, including on errors. Clicking the banner body opens
the phone without answering. OS presentation can place actions under an options
menu. The native APIs are UserNotifications.framework on macOS and WinRT toasts
with a COM activator on Windows; the Tauri notification plugin is not used.

Native delivery runs on a separate worker with a bounded latest-state handoff;
OS latency cannot stall phone-state events. Show/withdraw/reset effects serialize
separately from live call authorization, preventing an in-flight show from
overtaking a cancellation, permission refresh or shutdown.
Failed delivery retries at most twice for the same still-ringing offer, after
two and four seconds following each failure. Success, withdrawal or call replacement
cancels pending retries. These retries never request permission. On macOS, settings
queries and add-request acknowledgements have five-second deadlines; explicit
foreground authorization has a separate three-minute deadline. A timed-out native
operation returns `nativeFailure`. A late acceptance removes the notice if its
offer was withdrawn, while preserving a live retry with the same token.
When a foreground settings read observes authorization granted in the OS settings,
a failed still-ringing offer becomes eligible for a new bounded delivery attempt.
This preserves its token, does not repeat an already successful banner, and never
resumes a claimed or withdrawn offer. Reading permission still never prompts.

`bun run test:macos-notification-timeouts` exercises stalled services, late
callbacks, withdrawal and valid retries without accessing the OS notification
service. `bun run test:macos-call-notifications` checks isolated native OS delivery,
action categories, synthetic action responses and withdrawal. The latter uses its
own test bundle identity, not Ruf's installed authorization.

Every offer uses a fresh random 16-hex-character token, mapped to the current
session-local `callId`. Tokens carry no caller data and are not exposed via IPC.
Actions recheck token, call ID, Incoming state and one-time claim under the phone
session lock; the native SIP thread repeats its generation/unanswered check.
A stale notice or duplicate callback cannot answer a replacement call or reject
an active call. Answer enters `answering`; only the native ACK/established event
sets `active`. Errors remain passive; a still-ringing failed attempt can be
offered again. Answering, rejection, cancellation, preference disable and shutdown
withdraw pending and delivered notices. No per-call persistent record is created.

Windows installed releases register a per-user COM activator and AppUserModelId;
NSIS uninstall removes those exact keys. `--notification-activation` keeps COM
launches hidden and cannot bypass live-call validation. Development/temporary
Windows builds do not register or overwrite installed-app activation entries.
Windows Notification Center actions require runtime acceptance on Windows;
cross-compilation alone does not confirm delivery or installer behavior.

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

## Ruf desktop identity

The display name is **Ruf** and the bundle identifier is `org.rufapp.desktop`.
Commands, DTO fields, validation and error codes are unchanged by the rename.
The release bundle is `Ruf.app` on macOS and installs under `Ruf` on Windows;
the internal executable remains `sip-desktop`.

Account metadata, desktop preferences and contact favorites use this bundle's
own application-data directory (`org.rufapp.desktop` for Ruf). OS passwords use
`org.rufapp.desktop` as their credential service. The former compatibility
routing to `org.sipsoftphone.desktop` has been removed at the user's request.
There is no fallback or automatic import of old SIP accounts, favorites or
settings. A clean Ruf installation starts without accounts; add credentials
again through the existing account settings. Other bundle identifiers use their
own application-data directories.

Incoming notifications use the Ruf bundle identity and its platform icon.
Caller titles and notification actions belong to the existing call-notification
implementation; the rename does not change their contracts.
macOS may require fresh microphone, contacts and notification consent for the new
identity. Its menu-bar symbol is a transparent monochrome r/handset template; the
OS applies light/dark tint, including after every status-image update: light
on a dark menu bar and dark on a light menu bar. Windows uses the same blue mark. A small badge means
a registered account is available; a larger badge means a non-idle call. No badge
means neither condition holds. Tooltip and menu text describe the actual state.

OS login registrations created under SIP remain separate. Remove a stale SIP
login item and enable login start in Ruf after installing the renamed bundle.
A saved explicit opt-out remains respected.
