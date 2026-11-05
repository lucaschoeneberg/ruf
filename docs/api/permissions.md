# Settings permission boundaries

Every exposed Tauri command is registered in the invocation handler and build-time
manifest and granted separately to named windows. IPC validation/deserialization
rejects missing required inputs, unknown enum values and malformed DTOs; denied
capabilities reject before dispatch. There are no HTTP status codes.

Main may read `get_preferences`, `get_desktop_settings` and
`get_notification_permission` for contact presentation and the notification hint.
It retains outgoing account selection and the eight explicit call-control
capabilities: `set_call_muted`, `set_call_held`, `transfer_call`,
`start_consultation`, `switch_call`, `cancel_consultation`, `complete_transfer`,
`send_dtmf`.
Each is separately registered/granted; settings has none. All require opaque live
call tokens; consultation cancellation/completion require the child token even
when its original is selected. It cannot save
application preferences, run audio/notification tests, copy diagnostics, toggle
saved account activation or request microphone permission.

Settings may read/save preferences; inspect/copy anonymous diagnostics; explicitly
read/request/open OS notification/microphone permissions; run/stop native audio
tests; toggle saved account activation; select the outgoing account; and clear
completed local history after its confirmation UI. It retains existing account,
contacts-permission, sound, desktop and About commands. Settings has no call
answer/reject/dial capability or arbitrary process, URL, filesystem or keychain API.

`get_microphone_permission`, `request_microphone_permission` and
`open_microphone_settings` take no inputs and require explicit settings capability.
The first is read-only and never captures or prompts; the second is an explicit
foreground request and may prompt only while undecided. The third opens a fixed
macOS microphone privacy pane or Windows microphone privacy settings, never a
caller-supplied URL. Unsupported opening returns `engineUnavailable`; failed OS
opening returns `nativeFailure`. Authorization states, native-test inputs and
failure cases are detailed in [audio diagnostics](audio-diagnostics.md).

Notification commands and their independent OS/application settings are described
in [notifications](notifications.md); whitelisted diagnostic fields and clipboard
bounds in [diagnostics](diagnostics.md); persisted fields/retention in
[preferences](preferences.md); acknowledged account activation in
[account activation](account-activation.md).

Ruf-owned login entries now use `--autostart`. Initial and single-instance login
launches read only local `startHidden` metadata; failure conservatively keeps the
phone hidden. Manual opens show it. Explicit `--background` and notification
activation remain hidden. Existing configured enabled login entries are refreshed
to current arguments; configured disabled entries remain disabled. Startup
presentation never requests permission or credentials.
