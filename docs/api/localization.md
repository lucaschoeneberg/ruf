# Application language

Ruf uses i18next in the frontend and rust-i18n for native text lookup/interpolation.
The two engines share German (`de`) and English (`en`) JSON catalogs for UI, native menus, app-owned
dialogs and notifications. All translations are local assets. Names, numbers,
device identifiers, protocol values, diagnostics and original license texts are
not translated. German phone-number matching remains independent of UI language.

## Preferences and detection

`AppPreferences.language` accepts exactly `system`, `de` or `en`. It may be omitted
in legacy input and defaults to `system`; it is always present in responses.
Other strings, null and wrong types fail deserialization. The settings window
saves the full object through the existing `save_preferences` command. Existing
validation, history confirmation, persistence errors and permissions still apply.
Selecting a draft does not change the displayed language; successful saving does.
Failure preserves the previously committed language.

Manual `de`/`en` overrides system preferences. In `system` mode the desktop reads
the preferred OS language list, choosing the first supported base language (for
example `de-CH` → `de`, `en-GB` → `en`); no match or an empty list falls back to
English. Resolution runs at startup and when changing back to `system`. Later OS
language changes take effect on the next app start. The browser preview uses
`navigator.languages` and does not create another persisted preference.

The additive field keeps existing version-1/version-2 metadata readable without
rewriting it at startup. Schema version remains 2. **Downgrades are not supported
after saving:** older binaries with strict unknown-field rejection may report
`storageFailure` when reading the new field. Corrupt metadata is preserved;
localization falls back to system language for display while the existing
preference reader continues to report the storage error.

## Read command and event

`get_localization` takes no arguments and is allowed in the main and settings
windows. It reads process-local state initialized from password-free metadata;
it does not initialize SIP, read credentials or request OS permissions.

Its result and the `localization-changed` event share the generated
`LocalizationState` contract. Every output field is required:

| Field        | Type                 | Meaning                                                                                                                                                        |
| ------------ | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `preference` | `system \| de \| en` | Committed preference, including automatic mode.                                                                                                                |
| `locale`     | `de \| en`           | Effective language for UI and native presentation.                                                                                                             |
| `revision`   | `number`             | Monotonic process-local revision, initially zero. A changed committed preference increments it. It is not persisted or related to the phone snapshot revision. |

After successful preference persistence, Rust updates localization in save order,
refreshes native menus/window titles independently of phone-state changes and
emits the state to both windows. Windows subscribe before their initial read and
ignore responses/events whose revision is not newer. A focused or newly opened
window reads the current state again. The settings save also reads it after the
save response. Switching translates existing message references without
remounting the application or discarding drafts/calls.

There is no separate language-write command and no HTTP status mapping. Tauri
rejects malformed commands or denied permissions; save errors use the existing
`PhoneError` contract. Event-delivery failure does not undo durable preferences;
subsequent reads converge on the saved state.

## Native boundaries and packaging

Menu IDs, keyboard actions and opaque call-notification tokens remain unchanged.
Existing call banners are updated with the same token when captions change.
Adapter APIs receive localized plain text from the desktop:
`show_call_notification(token, title, body, answer, reject)` and
`show_test_notification(title, body)`. Title/body bounds remain 512/1024 UTF-8
bytes; answer/reject labels are required and bounded to 128 bytes. Empty strings,
control/bidirectional formatting characters and invalid tokens return
`invalidInput`. Platform authorization/service failures retain their previous
semantics. Updating labels does not request authorization or add notification
audio; native token ownership and caller privacy rules remain authoritative.

macOS bundles contain German/English `InfoPlist.strings` for the app-owned Contacts
and microphone purpose descriptions. OS-owned dialog content follows the OS/app
language selected by the operating system, not a forced runtime override.
Windows NSIS supports English/German and automatically selects from the OS
language, falling back to English. This does not persist the in-app preference.

## Catalog checks

`bun run i18n:check` takes no arguments and validates identical flat keys, named
placeholders and paired i18next JSON V4 `_one`/`_other` plural keys across the two shipped
catalogs. It exits zero on success, nonzero for malformed/mismatched catalogs,
and writes no files. `bun run check` includes it before type checking.
Resources are flat string maps. `%{name}` interpolation is rust-i18n's standard
syntax; i18next uses its built-in configurable prefix/suffix to consume the same
files. Frontend numeric counts use `%{count, number}` and i18next's built-in
number formatter; i18next resolves plural suffixes and falls back to English.
Missing frontend keys/parameters fail explicitly, and injected values cannot invoke
nested translations. Native message keys are typed and checked in both catalogs;
rust-i18n leaves an omitted native variable visible as `%{name}`. Messages use
normal Svelte escaping, never HTML rendering.

The Svelte store/message-reference and native IPC/revision bridge remain project
code; they do not parse or replace translation templates. Native rust-i18n
compiles the shared resources into the executable and receives an explicit locale
per lookup, never a separate global language setting. Cargo tracks resource-file
changes through the shared locale directory. `Intl.DateTimeFormat` and collation
continue to format presentation in the effective language and local time zone.
The catalog checker validates authoring consistency at build time, not runtime
translation. No HTTP resource loader, browser detector plugin, cookies or second
language preference store is introduced.
