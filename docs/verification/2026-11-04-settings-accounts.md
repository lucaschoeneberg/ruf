# Separate settings window and saved SIP accounts

## Implemented scope

The compact phone opens/focuses a singleton native settings window. The new
Svelte settings entry follows the user's Zed reference: near-black searchable
sidebar, account list, restrained blue selection, thin row separators and compact
controls. Telephone supplies the account-management pattern. Existing controls,
validation, Bun tooling, Rust/ts-rs contracts and the baresip thread are reused.
The only new production integration dependency is keyring for OS secret storage;
serde_json was already in the workspace dependency graph.

Up to eight simultaneous SIP registrations, outgoing account selection,
add/edit/remove, per-account state, durable password-free metadata and actual OS
credential storage are implemented. One global call remains the limit. Actual
audio mode is in snapshots, including after webview reloads. Contacts/history
remain honest empty states.

## Native evidence

The Asterisk lab was initially stopped and started through this project's Compose
file; no unrelated containers were changed. Synthetic identities 1001/1002 extend
the existing lab. `bun run test:asterisk` passed with:

- REGISTER_OK and an established echo call on the existing 1000 identity.
- Returned G.711 WAV: 4.04 seconds, RMS 5811, frequency 439.8 Hz.
- Two accounts independently registered at once, calls selected through each ID.
- An incoming Asterisk call to the second account while outgoing selection stayed
  on the first; manual answer, active state and hangup.
- Busy guards on selection/removal during a live call; rejected native account
  update restores the previous identity when Rust's reservation was stale.
- Independent account removal and the existing stale-call-action regression.

The first expanded test invocation exposed parallel use of the process singleton;
the smoke script now runs the native interop tests with `--test-threads=1`.
Incoming-call verification exposed missing `call_accept yes` in the bridge's
baresip configuration. This creates incoming dialog objects without the upstream
menu module; answering remains manual. The final test passes without a SIP proxy
or Docker-specific workaround.

The opt-in `saved_accounts_survive_session_restart_without_password_readback`
desktop test passed against the real macOS credential store and the local lab.
It used isolated temporary metadata and unique synthetic keychain entries. It
verified two-account restoration in a fresh PhoneSession, saved outgoing
selection, empty-password update, deletion of the obsolete credential, absence
of passwords from JSON and recovery/cleanup of a journaled interrupted secret.
The test removes its own accounts, keychain entries and temporary metadata.

A new capture-only macOS process verified actual native window behavior:
`WINDOWS 2 [SIP · Einstellungen, SIP]`, close settings => one window, reopen =>
two windows. Only that PID was automated and terminated. Native screenshot:
`native/build/settings-native.png` (ignored local artifact, empty real app).

## UI evidence and review

The browser interaction script at `native/build/test-settings-ui.mjs` uses
synthetic IPC fixtures, not real SIP. It passed navigation, cross-window command
routing, account selection, add/edit, whitespace/password preservation, inline
validation, password reuse, registration feedback, removal confirmation, search,
unsaved-draft protection and polling. It checked 900 × 638 and minimum client
740 × 518 settings sizes plus a 360 × 532 phone.

The polling regression was observed failing before the fix: a fresh account
snapshot reset the removal confirmation after 500 ms. Removing the unnecessary
effect fixes it; the existing keyed component resets state only on account
changes. The final test waits 750 ms and confirms the dialog remains actionable.

The visual finish reviewer requested one change: reveal/focus the unsaved-edit
notice when navigation starts from a scrolled form. The fixed 740 × 518 capture
shows the complete named group and both actions, preserves the draft and focuses
“Weiter bearbeiten”. The reviewer scored that fix resolved and returned ship at
that scope. Native screenshot and browser fixture captures are kept under the
ignored review/build directories; they are not shipped application data.

A single Impeccable detector pass reported 46 advisory token-catalog mismatches,
no non-advisory findings. Existing main-phone token drift predates this change;
new settings tokens are documented narrowly. No ignores/suppressions were added.
The earlier side-tab accent finding is no longer present in the changed surface.

Independent code review identified native acknowledgment, polling-reset, delayed
removal and credential-cleanup/recovery issues. All five findings were addressed
and scored resolved in bounded re-review. Credential deletion is durably
journaled, and a failed restoration leaves a visible repairable account.

## Final checks

Passed on macOS arm64 with the pinned Rust 1.98.1 toolchain:

- `bun run format:check`, `check:versions`, `test:ui` (11 tests / 45 assertions),
  `check` (zero Svelte/TypeScript errors or warnings), and `build`.
- `bun run types:generate`, then byte-for-byte stability on a second generation.
- Workspace Clippy (`-D warnings`) and tests both portable and with
  `--features sip-desktop/native`.
- `bun run desktop:build --features native --bundles app` (15.29 MiB, ad hoc
  macOS app bundle); expected notarization skip without distribution credentials.
- Actual Asterisk integration and the opt-in real macOS keychain/session
  integration described above; these are separate from normally ignored tests.
- Synthetic browser flow and native two-window open/close/reopen verification.
- Independent code review, bounded visual verdict and `git diff --check`.

## Limits

Windows build definitions and keyring backend are prepared but were not executed
on Windows here. Hosted CI, installer distribution/notarization, hardware
microphone/speaker audio, encrypted transports, NAT traversal beyond the local
lab, eight-provider interoperability and private caller identification were not
verified. The actual media proof uses the explicit synthetic test-tone source.
