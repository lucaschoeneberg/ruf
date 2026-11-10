# About-page links

`open_about_link` is available only to the settings window. It opens a fixed
company page through the operating system's default HTTPS handler; it does not
load remote content into Ruf's webview, access accounts or initialize telephony.
There are no HTTP response/status-code semantics: this is local Tauri IPC.

Input: required `{ link: AboutLink }`, where the Rust enum is exported by ts-rs:

| Value     | Destination                      | Meaning                        |
| --------- | -------------------------------- | ------------------------------ |
| `company` | `https://www.mardu.de/`          | Mardu GmbH company website     |
| `imprint` | `https://www.mardu.de/publisher` | Company legal notice           |
| `privacy` | `https://www.mardu.de/privacy`   | Company website privacy policy |

No optional fields. No caller-supplied URLs or protocols. Missing/unknown enum
values are rejected by Tauri deserialization; window access is enforced by the
settings capability. Success returns `null` (`Promise<void>`). OS process failure
or blocking-task failure returns the existing `PhoneError` with `nativeFailure`.
Success means the OS handler accepted the request, not that the website loaded.
The frontend keeps the settings page open and displays an error with the company
domain on failure. Browser previews use ordinary external HTTPS links.

## Compiled software inventory

`get_software_bill` is a settings-only command with no arguments. Success returns
one required JSON **string** (`Promise<string>`) containing the exact CycloneDX
1.6 snapshot compiled into this executable. It performs no filesystem reads,
credential access, SIP initialization or network requests. Required top-level
fields are `bomFormat: "CycloneDX"`, `specVersion: "1.6"`, `version: 1`,
`serialNumber` (a UUID URN), `metadata`, `components` and `dependencies`.
`metadata` contains application identity, timestamp, generator and build properties.
Each entry in the top-level `components` array and `metadata.component` has a
required `type`, unique `bom-ref`, `name` and `version`;
`group`, `publisher`, `purl`, `scope`, `licenses`, `hashes` and `properties` are
optional. Generator entries under `metadata.tools.components` contain `type`,
`name` and `version`. Each dependency has required `ref` and `dependsOn` references.

Build properties include `ruf:build:target`, `ruf:build:native` (string
`true`/`false`), `ruf:build:profile` (`debug`/`release`), `ruf:build:kind`
(`desktop`/`preview`) and `ruf:frontend:resolution` (`rendered-modules` or
`development-preview`). Identifiers must resolve within the same document.
`scope: excluded` denotes build-only components, not a runtime dependency.

A plain Cargo debug/dev build without a matching packaged inventory rejects with
existing `PhoneError.engineUnavailable`; denied window permissions are handled by
Tauri. There are no HTTP status codes for the command. The frontend parses only
this local serialized document. In development/browser previews it may fall back
to the local Vite `/software-bill.cdx.json` asset. A release never substitutes a
preview on an IPC error: it shows an inventory-loading error instead.

The CLI `sip-desktop --export-sbom <output-path>` exports the same snapshot and
exits before Tauri, accounts, contacts or SIP start. Exactly one output path is
required. Missing/extra arguments, unavailable inventory or write failure exit
with status 2. It is a local executable option, not exposed as arbitrary file
access to the webview. See [licensing](../licensing.md#about-page-and-project-sbom)
for generation, artifact association and remaining distribution-review limits.
Existing `AppInfo` and generated IPC DTOs are unchanged.

## Consumer changelog

The What's new view selects bundled `CHANGELOG.md` (English) or `CHANGELOG.de.md`
(German) for the effective application language, written as user-facing
release notes rather than commits or implementation details. The supported format
is `## Version heading` followed by one or more single-line `- Change` bullets.
An optional document title (`# ...`) and blank lines are ignored. Remote content
is rendered as escaped text, never HTML. Empty/no-version documents, headings
without bullets, and documents over 100,000 characters are rejected.

Optional build configuration: `VITE_RUF_CHANGELOG_URL` for English and
`VITE_RUF_CHANGELOG_URL_DE` for German, each a public
`https://raw.githubusercontent.com/<owner>/<repo>/<ref>/CHANGELOG.md` URL.
Without a source for the current language, Ruf shows that language's bundled
notes and performs no changelog
requests; a Git remote does not configure this source automatically.
Private repositories and authentication are unsupported. This value is public
build configuration and must not contain credentials or query parameters.

With a configured source, first opening the disclosure sends a GET with no cookies
or referrer and an eight-second timeout; “Aktualisieren” explicitly retries.
Only HTTPS on `raw.githubusercontent.com`, no custom port, userinfo, query or
fragment, and a `.md` path are accepted. CSP permits that host for fetching only.
Non-2xx HTTP responses, network failures, timeout and invalid content retain the
bundled notes with a short status message. Leaving the About page aborts a
pending request. A language change aborts the previous request and resets notes
to the new language's bundled copy. Late responses never replace notes for a
different language, and no language falls back to another language's remote URL.
This is an optional frontend fetch, not a new IPC command or DTO.
