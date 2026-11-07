# Menus, action search and keyboard navigation

Ruf provides window-local shortcuts, a searchable action list and native menus.
The magnifier in the phone header and **Cmd/Ctrl+K** open **Ruf durchsuchen**.
The shared result list groups actions, contacts and local call history. Action
search matches all entered words against German labels and action synonyms.
Contact/history results start after two characters. No permission prompt or
telephone operation is caused by opening, typing into, or choosing a personal
search result. Individual contacts, history and settings searches remain available.

## Combined contacts and history search

- Contacts reuse the authorized in-memory `ContactsSnapshot` and `searchContacts`
  with current favorites, organization-display and German-number matching
  preferences. Names, companies and original phone numbers remain searchable;
  hidden company labels do not disable company search. Ambiguous owners stay
  separate. A result opens that contact's existing detail/number chooser.
- History reuses `get_history` with required `query` fields: `search`,
  `filter: "all"`, `offset: 0`, `limit: 8`, `contactNumbers: []`.
  Input is at most 200 characters.
  Formatted telephone input reuses `normalizeDialNumber`; other input follows
  existing literal native history matching by name, number and account. Requests
  are delayed 200 ms, and stale/unmounted responses are ignored.
- At most eight contacts and eight history rows appear initially. Extra matches
  expose **Alle … anzeigen**, opening the existing full panel with its query
  preserved. History results open the exact record's details and focus its row
  after the matching page arrives. Deleted results cannot authorize a callback.
- Calling always requires the existing contact call or history callback button.
  Native callback account identity, idle/build/audio checks and token validation
  remain unchanged. Choosing a search result never dials automatically.
- `history-changed`, `contacts-changed`, contact-cache changes and window focus
  invalidate the in-memory history preview. Revoked/failed contact reads remove
  current contact projections; outstanding old responses cannot restore them.
  No personal search results are persisted or logged by the frontend.
- Loading, unavailable/denied contacts, empty matches, history read errors with
  retry and `recordingError` are explicit. A failed source does not disable other
  result groups. Browser preview explains that personal results need Desktop.

No new invoke command, DTO field, error code or HTTP status mapping is introduced.
See [contacts](contacts.md) and [history](history.md) for validation, permissions,
query matching and existing `PhoneError` classifications.

## Shortcuts

| Shortcut (Cmd on macOS, Ctrl on Windows) | Action                                                                                                              |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Cmd/Ctrl+1 / +2 / +3                     | Tastatur / Kontakte / Verlauf                                                                                       |
| Cmd/Ctrl+L                               | Focus number entry while idle                                                                                       |
| Cmd/Ctrl+F                               | Focus current contacts/history search; from dialer/start, open contacts search. In Settings, focus settings search. |
| Cmd/Ctrl+K                               | Search actions, contacts and history; discover shortcuts                                                            |
| Cmd/Ctrl+,                               | Open Settings using existing draft guards                                                                           |
| Cmd/Ctrl+Enter                           | Answer the offered incoming call                                                                                    |
| Cmd/Ctrl+.                               | Reject incoming call or hang up current call                                                                        |
| Cmd/Ctrl+Shift+M                         | Toggle microphone mute                                                                                              |
| Cmd/Ctrl+Shift+H                         | Hold / resume selected conversation                                                                                 |
| Cmd/Ctrl+Shift+T                         | Open transfer / consultation destination; never submit it                                                           |
| Cmd/Ctrl+Shift+D                         | Toggle in-call DTMF keypad                                                                                          |
| Enter in number entry                    | Dial the validated number                                                                                           |
| Up / Down in action search               | Select result, including unavailable actions with an explanation                                                    |
| Enter in action search                   | Execute enabled selected action                                                                                     |
| Escape in action search                  | Close and return focus to previous connected control                                                                |

Plain M/H/T letters have no call-control meaning and remain available for typing.
Browser shortcuts ignore repetition, IME composition and extra Alt modifiers.
Desktop shortcuts use Tauri native accelerators; webview fallback handlers run
only in browser preview, preventing duplicate invocation. No system-wide shortcut
registration is introduced. Browser preview cannot place or control calls.

**Ansicht** exposes destinations, focused search and action search. **Gespräch**
exposes state-dependent call controls with visible accelerators. Tray controls
reuse the conversation menu without accelerators. **Hilfe** opens the action/
shortcut overview, first-run help and diagnostics. The Help submenu uses Tauri's
`HELP_SUBMENU_ID`, which registers the native macOS Help menu and its system menu
search. Windows uses the same menu entries and the cross-platform action search.
The full shortcut reference is also searchable in Settings → Diagnose & Hilfe.

## Internal native-to-window events

These events are shell requests, not new invoke commands or DTO objects. Primitive
payloads are required; there are no optional fields or HTTP status codes.

| Event                                                                                               | Target     | Payload                                                 | Behavior                                                                                                  |
| --------------------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| `phone-dialer-requested`, `phone-contacts-requested`, `phone-history-requested`                     | `main`     | `null` (Rust unit)                                      | Select phone destination without removing current call controls.                                          |
| `phone-number-requested`                                                                            | `main`     | `null`                                                  | Focus idle number entry; ignore during a call.                                                            |
| `phone-find-requested`                                                                              | `main`     | `null`                                                  | Request scoped search focus, also after asynchronous contact loading. No contacts access request.         |
| `phone-search-requested`                                                                            | `main`     | `null`                                                  | Open action search.                                                                                       |
| `phone-start-requested`                                                                             | `main`     | `null`                                                  | Open existing getting-started view.                                                                       |
| `phone-mute-requested`, `phone-hold-requested`, `phone-transfer-requested`, `phone-tones-requested` | `main`     | Opaque call-token `string`, captured in the native menu | Ignore unless token matches selected live dialog; then reuse `CallPanel` availability and action handler. |
| `settings-find-requested`                                                                           | `settings` | `null`                                                  | Focus existing settings search when Settings owns Cmd/Ctrl+F.                                             |

The shell restores/focuses the phone before emitting phone requests. Requests
while action search owns focus are ignored. Answer/reject/hangup menu actions
continue using existing captured-token Rust handlers. Action search uses the
same guarded UI handlers as buttons: busy and unknown-audio guards, confirmed
mute acknowledgment, native hold/transfer recovery and one-dialog transfer rules
still apply. Unavailable actions stay visible with a reason and cannot execute.
Pending native acknowledgment never fabricates mute/hold success. Race failures
from invoke commands use existing `PhoneError` rendering and snapshot delivery;
no new error code is added. Menu delivery/focus failures remain best-effort shell
operations; they do not perform a fallback telephone action.
