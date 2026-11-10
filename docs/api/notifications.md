# Native notifications and OS permission

These Tauri IPC commands use `PhoneError`, not HTTP status codes. Required output
fields remain present even when nullable. The canonical `NotificationPermission`
is `notDetermined`, `denied`, `authorized` or `unsupported`; a failed read rejects
with `nativeFailure` and must remain unknown in the UI.

| Command                           | Input | Result and authorization                                                                                                                                                                                                                                             |
| --------------------------------- | ----- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_notification_permission`     | none  | Canonical permission only. Read-only; independent of account restoration, metadata, keychain and SIP. Available to main/settings. Never prompts.                                                                                                                     |
| `request_notification_permission` | none  | Existing `DesktopSettings` response. Settings only, after an explicit foreground action. May prompt only if undecided; denial is a normal permission result. If subsequent preference retrieval fails, use the independent getter to recover the actual OS decision. |
| `open_notification_settings`      | none  | Settings only, `void`; opens the fixed macOS Ruf notification pane or Windows notification settings. Accepts no URL and never changes permission itself.                                                                                                             |
| `test_notification`               | none  | Settings only, `void`; checks authorization, waits five seconds on a blocking worker, then submits fixed anonymous text with another native authorization check. No implicit permission request.                                                                     |

Opening the OS pane can return `engineUnavailable` on unsupported platforms or
`nativeFailure` when opening fails. A concurrent notification test returns
`busy`; an initially non-authorized test returns `invalidState`. Delivery denial,
service errors and timeouts return `nativeFailure`; unsupported delivery contexts
can return `engineUnavailable`. Success reports OS acceptance, not proof of
visible delivery: Focus and OS display choices still apply.

The test is silent and contains only the German text “Testbenachrichtigung”
(“Test notification”) and “Benachrichtigungen von Ruf sind aktiviert.”
(“Notifications from Ruf are enabled.”). It has no caller/account information,
call token, Answer/Reject buttons or incoming-call category. macOS uses its own
UUID identifier namespace and independent five-second settings/acceptance waits;
it does not wait on the real-call delivery queue or touch its active token or
withdrawal registry. Late callbacks remain safe. Test clicks are ignored by the
call-action delegate. Windows reuses the existing COM worker, a separate tag/group
and a basic toast without call actions; test activations are ignored and the
live-call toast map remains intact. Uninstalled/development Windows contexts
remain unsupported. No SIP or audio state changes.

`DesktopSettings.notificationsEnabled` is the application preference;
`notificationPermission` is OS authorization. Neither implies the other. Native
baresip ringing remains independent. `notificationsGranted` remains a compatible
nullable projection: authorized→true, denied/undecided→false, unsupported→null.

`AppPreferences.showCallerInNotifications` defaults to true. False uses a generic
incoming title/body with no caller name, number, matched contact or account name.
The delivery worker reads this flag without blocking or loading metadata; unknown
or contended session state also uses anonymous text. Changing captions preserves
the opaque token and its still-live call binding. Existing stale-action checks,
withdrawal and bounded delivery retry remain in force.

Authorized current contact names apply to incoming captions as “Name ruft an”. Updated or revoked contact references replace the bound caption without changing its action token. The anonymous notification preference still suppresses both contact name and number.
