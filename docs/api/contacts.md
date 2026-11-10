# Native macOS contacts

The main phone window can read macOS Contacts through explicitly permitted
Tauri commands. No HTTP API, filesystem permission or SIP-engine startup is
required. Native access runs on a blocking worker independently of the serialized
phone session. The system address book is read-only in this iteration.

## Commands

| Command                   | Required input                    | Result             | Behavior                                                                                                                                                                              |
| ------------------------- | --------------------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_contacts`            | none                              | `ContactsSnapshot` | Returns the cached snapshot, rejects a cached read error, or performs an initial read when neither exists. Never requests permission.                                                 |
| `request_contacts_access` | none                              | `ContactsSnapshot` | Explicit user action. Requests system permission only while `notDetermined`, then reads the accessible contacts. Denial is a normal permission result; it does not repeatedly prompt. |
| `set_contact_favorite`    | `{id: string, favorite: boolean}` | `null`             | Adds/removes an app-local favorite by opaque native contact identifier. Never modifies a system contact.                                                                              |

All inputs shown are required and have no defaults. Tauri rejects malformed or
missing command arguments and permission violations. Commands are granted to the
`main` window only, not settings. Favoriting requires a nonempty ID of at most
1024 UTF-8 bytes without control characters; at most 1000 favorite IDs may be
stored. Duplicate additions and removing a missing favorite are idempotent.
An identifier is scoped to the macOS source; favoriting does not validate that a
source contact still exists, and stale identifiers are not assigned to namesakes.

Additional main-window commands:

| Command             | Input                   | Result / errors                                                                                                                                                                                                                                                                                                                                            |
| ------------------- | ----------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `refresh_contacts`  | none                    | `ContactsSnapshot`; explicitly reloads OS permission, records and favorites without prompting. Same errors as `get_contacts`.                                                                                                                                                                                                                              |
| `open_contacts_app` | optional `{id: string}` | `null`; launches the fixed Apple Contacts bundle, targeting the exact source contact when `id` is supplied. Omitted/`null` IDs open the app alone. IDs must contain 1–1024 UTF-8 bytes without control characters (`invalidInput` otherwise). `nativeFailure` on launch failure; `engineUnavailable` elsewhere. No caller-supplied URL or source mutation. |
| `copy_phone_number` | `{number: string}`      | `null`; copies a user-selected original number to the system clipboard. 1–1024 UTF-8 bytes, no control characters; `invalidInput` otherwise. `nativeFailure` for clipboard failure, `engineUnavailable` on unsupported platforms. The frontend first tries the WebView clipboard and uses this native fallback.                                            |

## Permission-only settings commands

These commands are explicitly allowed in both `main` and `settings`. Settings
does not gain access to `get_contacts`, `request_contacts_access` or favorite
mutations. The existing `open_settings` command also accepts `section: "contacts"`.

| Command                       | Input | Result and behavior                                                                                                                                                                                                                |
| ----------------------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_contacts_permission`     | none  | `ContactsPermission`. Reads only OS permission; never prompts, enumerates contacts or reads favorites.                                                                                                                             |
| `request_contacts_permission` | none  | `ContactsPermission`. Explicit user action; requests only undecided access. No contact records are fetched. Denial returns `denied`; a native request error rejects with `nativeFailure`.                                          |
| `open_contacts_settings`      | none  | `null`. Opens the fixed macOS Contacts privacy pane. No URL, path or arguments are accepted; the command does not change permission. Launch failure rejects with `nativeFailure`; other platforms reject with `engineUnavailable`. |

Permission commands use the same contacts worker/mutex as phonebook reads, never
the phone session lock. Worker/lock failures use `storageFailure`. Non-macOS
permission queries return `unsupported`. Tauri rejects malformed arguments and
denied command permissions. No HTTP status codes or credentials are involved.

The Privacy section in Ruf Settings displays the current Contacts permission
and offers an explicit request. `section: "contacts"` opens Contacts & History for
display, matching and history preferences; permission controls are under
`section: "privacy"`. After denial, macOS does not prompt again; use the system privacy
pane and enable Ruf there. The status refreshes when the window regains focus.
Only after an access request does macOS normally list the app in Contacts privacy.
Browser preview reports unsupported access without invoking a native API.

## Generated DTOs

Rust `phone-core` and the existing ts-rs exporter own the frontend declarations.
All fields below are required. Empty strings/arrays and explicit `null` represent
absent values; there are no optional DTO fields.

```ts
type ContactSource = "macos";
type ContactsPermission =
  "unsupported" | "notDetermined" | "restricted" | "denied" | "authorized";
type ContactPhone = {
  label: string;
  number: string;
  destination: string | null;
};
type Contact = {
  id: string;
  source: ContactSource;
  name: string;
  organization: string;
  phones: ContactPhone[];
  favorite: boolean;
};
type ContactsSnapshot = {
  permission: ContactsPermission;
  contacts: Contact[];
};
```

- `id` is the opaque macOS unified-contact identifier, not a name, phone number or
  SIP account ID. It can change if the source replaces, links or unlinks a record.
  macOS's native contact unification is respected; this app does not merge namesakes.
- `name` uses the native name formatter, falling back to organization, first phone
  number or the unnamed-contact label (`Ohne Namen` in the German UI).
  `organization` is empty when absent. Names are plain text.
- `phones` contains all labeled source phone numbers; contacts without a number
  remain visible. `number` retains its source spelling. `label` is the localized
  native label, or `Telefon` for an unlabeled number.
- `destination` removes visual whitespace, parentheses, hyphens, dots and slashes
  only when the result is unambiguous. It retains national prefixes, extensions,
  star/hash service codes and an initial `+`. A result must contain a digit, fit
  the existing 64-byte ASCII dial syntax and have at most one initial plus sign.
  A number beginning with `+` cannot also contain `*` or `#`.
  An international optional `(0)` prefix, controls, letters, SIP URIs, post-dial
  comma/semicolon sequences and unsupported characters produce `null`; the
  original number remains visible but cannot be dialed. No country is inferred.
- `favorite` is app-local and changes only after durable preference storage succeeds.
- `contacts` is empty for `unsupported`, `notDetermined`, `restricted` or `denied`.

## Errors and state

Commands reject with the existing `PhoneError` shape:

| Code                | Conditions                                                                                                                                                                                   |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `invalidInput`      | Invalid favorite/contact ID, invalid clipboard text or exceeding the 1000-favorite limit.                                                                                                    |
| `nativeFailure`     | Native permission/enumeration fails, borrowed strings cannot be copied without loss, native data is invalid, or an OS launch/clipboard write fails. No partial contact snapshot is returned. |
| `storageFailure`    | Favorite preferences cannot be read/replaced, have an unsupported version or are corrupt; or a blocking operation fails. Existing corrupt preferences are preserved.                         |
| `engineUnavailable` | Favorite mutation, native clipboard access or opening the macOS Contacts app/privacy pane on an unsupported platform. This does not describe SIP linkage.                                    |

Permission denial/restriction is a snapshot state, not an exception. If access is
revoked during enumeration, all partial rows are discarded. Failed reads clear
the displayed contact list and show recovery instructions. Windows reads return
`unsupported` with no contacts; the browser preview explains the macOS requirement
without invoking a native command.

A background cache subscribes to `CNContactStoreDidChangeNotification` via a
revision counter and checks it every 500 ms. It checks permission without prompting
every two seconds; full enumeration occurs only on initial load, source revision,
permission change, recovery after a failed read, or explicit refresh/focus restoration.
`contacts-changed` has a null payload and invalidates contact/history views. The
listener is installed before the panel's initial read. Updates clear old records
on denied/revoked permission or read failure. The cache is memory-only; neither
source names nor phone numbers are written to disk. A serialized contacts mutex
keeps reads/favorite writes ordered without holding the phone-session mutex.
Unmounted panels ignore late responses. A manual refresh remains available.

## Search and calling

Search is an in-memory frontend filter over name, organization and original phone
numbers, ignoring common number separators, case and diacritics. Favorites sort
first by default, then names alphabetically; `contacts.favoritesFirst: false`
uses alphabetical order instead. `contacts.showOrganization` controls company
labels without removing company search. A favorites-only filter is explicit. Initial
rendering is limited to 50 matching contacts with a load-more action.

Clicking a supported number reuses `dial {destination}` through the main phone's
existing operation handler and selected SIP account. No contact-specific SIP API
or automatic account change is introduced. Registration, single-call occupancy,
build availability and audio-mode checks still apply. Multiple numbers have
separate call actions; unsupported numbers remain disabled with an explanation.
The compact phonebook calls a single supported number directly from its row;
contacts with several numbers first expand an inline number chooser. Only one
contact is expanded at a time. The star beside search toggles favorites-only
filtering; the count is shown beside the heading. Source/privacy information
remains in Settings → Privacy instead of being repeated below the list.
Inline details expose every original number, per-number copying, the fixed Apple
Contacts app action targeting that contact's native ID and a scoped call-history link.
The ID is percent-encoded into an `addressbook://` link and passed directly to
`/usr/bin/open -b com.apple.AddressBook` without a shell. No permission prompt,
name search or contact modification occurs. Success confirms the OS launch request,
not that Contacts displayed the card: macOS handles deleted or inaccessible IDs.
Cmd/Ctrl+F focuses search;
Escape clears search/collapses details, and ArrowDown from search enters the list.
History links back to the exact source identifier, clearing filters and revealing
that contact. Deleted or inaccessible contacts do not open a namesake.

## Caller recognition

`EngineSnapshot.callerContact` is a nullable `ContactReference {id, source, name}`
resolved from the authorized memory cache and selected dialog's native caller
number or outgoing destination. It
never modifies raw `caller` evidence. Matching removes supported visual separators
and equates an explicit international `00` prefix to `+` for external-length
numbers. The explicit `contacts.matchGermanNumbers` preference additionally
equates German national numbers with +49 (see below). It preserves short
extensions and service codes. It never uses suffix matches or matches names; withheld/unsupported/ambiguous
numbers return null. If two contacts own the same number, neither is asserted.
The call panel and system notification prefer the matched current name and retain
the original caller number. This is display convenience, not caller authentication.
Changes/revocation update projections; a changed ringing caption replaces the
notification under the same bound action token. Unchanged captions never cause
repeated notices. Titles are sanitized and bounded to 128 Unicode characters.
Names are not copied into historical facts.
See [call history](history.md) for local persistence, callbacks and filtering.

## Persistence and permissions

macOS owns contact persistence and synchronization of configured system accounts.
The app fetches only identity/name, organization and phone fields, not photos,
notes, birthdays or arbitrary attachments. No external provider login is added.
`NSContactsUsageDescription` explains the request in the app bundle. Hardened
Runtime builds also include the `com.apple.security.personal-information.addressbook`
entitlement in the actual code signature. `bun run test:macos-contacts [bundle]`
verifies both and the bundle signature; checking only source plist files is not
sufficient. Native authorization completion errors are propagated instead of
being mistaken for a normal undecided/denied response.
Explicit contact authorization has a three-minute deadline. A stalled system
request returns `nativeFailure`, releases the contacts operation gate and permits
a later retry. Late completion callbacks retain their own state and never access
the returned caller's stack. This deadline does not cancel an OS dialog or infer
the user's access decision. `bun run test:macos-contact-timeouts` verifies stalled
and late consent callbacks with a fake store; it requests no real permission and
reads no personal contacts.

`contact-favorites.json` in Tauri's app data directory stores schema version 1 and
an array of native identifiers under `favorites`. A same-directory temporary file
is replaced while holding the contacts mutex. It contains no copied names or
numbers and is not encrypted; identifiers are private local application data.
No address-book cache is written to disk or logs. System data is edited in Apple's
Contacts app. There is no vCard import, Windows/Outlook/CardDAV/Graph provider,
contact creation or source writeback in this iteration. Locally observed call facts
are persisted independently as described in [call history](history.md).

## Dialer, conversation and history identity

The dialer loads authorized contacts without prompting, suggests at most three
explicit contact/number choices after three digits, and never calls automatically.
Spaces and visual separators are normalized for lookup. The display preserves
original contact spelling. Ambiguous owners remain separate choices.

`CallSnapshot.contact` is a required nullable `ContactReference`, enriched by the
desktop for each live dialog, incoming or outgoing. The legacy
`EngineSnapshot.callerContact` projects the selected dialog's contact in either
direction. These references are current authorized memory-only values, not SIP
identity authentication or persisted contact names. Renames and late contact
additions are resolved again in current calls and old history rows. Denial,
revocation or read failure clears the projection; withheld numbers are not resolved.
Notifications use the same selected reference and preserve the anonymous setting.

`contacts.matchGermanNumbers` defaults to true and can be disabled in
Contacts & History. With this explicit setting, national numbers beginning with
one 0 and at least seven dial characters match the corresponding +49/0049 form.
Other countries, short extensions and service codes retain conservative exact
matching. Multiple source contacts owning equivalent forms yield no identity.
The conservative `resolve_contact` API remains unchanged; desktop projections use
`resolve_contact_with_preferences` and its matching-key helper.
