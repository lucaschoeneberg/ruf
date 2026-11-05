# Local application preferences

These local Tauri IPC commands use the existing `PhoneError` contract;
there are no HTTP status codes. No passwords, audio data or contact records are
contained in the preference object.

`get_preferences → AppPreferences` has no arguments and is allowed in the phone
and settings windows. Only the settings window may save. `save_preferences` requires
`{ preferences: AppPreferences }` and accepts optional
`confirmHistoryDeletion: boolean` (default false). It returns the complete saved
`AppPreferences`. The read command accesses metadata directly without account
restoration, credential access, permission requests or SIP initialization. A
metadata failure does not prevent the separate permission commands or authorized
contacts from loading. Preference changes are explicit saves; unknown fields and
malformed/missing required boolean or nested object fields fail deserialization.
`history.retentionDays` may be omitted in input, which means `null`; it is always
present and nullable in output. The UI sends the complete generated contract.

| Output field                  | Default | Meaning                                                                                                                                                                      |
| ----------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `startHidden`                 | `true`  | Keep the phone hidden on login launches only. Manual launches stay visible.                                                                                                  |
| `showCallerInNotifications`   | `true`  | Include caller name/number in native notifications; false uses anonymous text. Does not change OS authorization or native ringing.                                           |
| `contacts.favoritesFirst`     | `true`  | Sort existing favorite contacts first.                                                                                                                                       |
| `contacts.matchGermanNumbers` | `true`  | Match German national 0… with +49…/0049… for contact identity, search and history scope. Does not rewrite dialing. May be omitted in legacy input; always present in output. |
| `contacts.showOrganization`   | `true`  | Show organization labels in contact lists. Does not fetch or change contacts.                                                                                                |
| `history.recordingEnabled`    | `true`  | Collect metadata for future calls. This is not audio recording. Changes require an idle phone and preserve existing rows.                                                    |
| `history.retentionDays`       | `null`  | Unlimited, or exactly `30`, `90`, `365` days. Completed records are removed by start timestamp strictly before the UTC cutoff; in-progress records are always preserved.     |

Invalid retention values and shortening retention that would delete existing
completed records without `confirmHistoryDeletion: true` return `invalidInput`.
Changing history collection during a call returns `busy`. Shutdown returns
`invalidState`. Metadata/SQLite failures return `storageFailure`. Failure does
not stop SIP. These preferences remain separate from the compatible
`DesktopSettings` notification-enabled and actual OS autostart/permission fields.

The history worker serializes policy changes with native call lifecycle events.
A call's collection choice is latched at its first start event: enabling later
never reconstructs a previously ignored call. Existing recorded calls can finish
normally. Retention runs at startup, on a saved history change, and daily.
Deletion and metadata persistence must both succeed; a metadata failure rolls
back deletion, and a failed SQLite commit restores the prior preferences. History
storage warnings remain independently visible in the existing history error DTOs.

The `accounts.json` metadata schema is version 2. Version 1 loads losslessly with
current-behavior defaults, unchanged credential keys, pending credential deletion
references and selected account. Each legacy account defaults to `enabled: true`.
Before the first v2 replacement, the exact original bytes are saved durably to
`accounts.v1.backup.json`; an existing different backup prevents migration rather
than being overwritten. The preference persistence operation clones existing metadata and does not
rotate keys, read passwords or write to the credential store. Initial session
restoration uses the existing noninteractive credential policy. Replacement uses a
synced temporary file in the same directory. Corrupt/unsupported metadata is
reported, never silently replaced. The backup contains account metadata and must
remain local user data.
