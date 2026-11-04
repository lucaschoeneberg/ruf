# Native macOS Contacts Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans in the existing shared checkout. Other agents are fixing bugs and design here; preserve their changes and keep integration edits small.

**Goal:** Make the existing Kontakte tab a useful, read-only macOS phonebook with explicit permission, search, multiple numbers, persistent favorites and calls through the existing dial command.

**Architecture:** Contacts.framework is isolated behind a safe Rust API in baresip-adapter, following the repository's native/unsafe boundary. phone-core defines contact DTOs and safe dial-target conversion; ts-rs generates TypeScript. A separate Tauri contacts module owns favorite IDs in the app data directory; no contact names or numbers are cached on disk. A self-contained Svelte panel inherits existing design tokens and calls the existing selected-account dial flow.

**Tech Stack:** Rust 2024, existing cc/serde/serde_json/ts-rs dependencies, Objective-C Contacts/Foundation, Tauri 2, Svelte 5, Bun.

**Spec:** User-approved direction in this chat and the Contacts section of ../specs/2026-11-04-sip-mvp-design.md. This iteration selects native macOS only; vCard, Windows and cloud sources remain deferred.

## Global Constraints

- Work in the current checkout explicitly shared with two other agents; no broad refactor, reset, automatic commit or global formatting mutations.
- Keep all functionality inside Kontakte. Preserve the header, dialer, settings, history and call controls.
- Read macOS contacts only. Do not request permission until the user presses the access button.
- Preserve source IDs and original number spelling; never merge by name, guess a country code, remove an optional trunk prefix or dial unsupported SIP/post-dial strings.
- Reuse dial and its registration/busy checks. SIP/audio implementation stays unchanged.
- Favoriting stores identifiers only; do not log or commit personal contact data.
- Document commands, DTOs, validation, errors, privacy and platform limits in docs/api.

## Review Focus

- Denied/revoked access clears displayed contacts and never prompts repeatedly.
- Several numbers, namesakes and contacts without numbers retain their identities and explicit actions.
- Formatted international numbers work; ambiguous trunk prefixes, post-dial codes and SIP URIs remain unavailable.
- Native enumeration stays off the UI thread and copies all borrowed C data before returning.
- Concurrent reload/favorite requests and component disposal cannot display stale contacts or discard favorites silently.

## Task 1: Typed phonebook contracts and safe telephone targets

**Files:** phone-core/src/contacts.rs, phone-core/src/lib.rs, phone-core/tests/contacts_contract.rs, phone-core/examples/export_types.rs, generated/AppInfo.ts.

- [x] Write and run failing contract tests for formatted numbers, extensions, service codes and unsupported/ambiguous input.
- [x] Implement ContactSource, ContactsPermission, ContactPhone, Contact, ContactsSnapshot and contact_dial_target.
- [x] Export DTOs using existing ts-rs generation and rerun the tests.

## Task 2: Native permission/read bridge and favorite persistence

**Files:** baresip-adapter/native/contacts.m, native/contacts.h, src/contacts.rs, build.rs, Cargo.toml, src/lib.rs; desktop/src-tauri/src/contacts.rs, lib.rs, build.rs, capabilities/main.json, Info.plist.

- [x] Test favorite round trips, corrupt storage rejection, invalid IDs and concurrent-safe mutation; test copying synthetic native contact values.
- [x] Implement read/request access, explicit permission states, source identifiers and multiple labeled numbers. Build the macOS bridge independently of SIP linkage.
- [x] Add get_contacts, request_contacts_access and set_contact_favorite, main-window only, with blocking work on Tauri's worker runtime.
- [x] Add NSContactsUsageDescription and preserve existing capabilities/configuration.
- [x] Run Rust tests and build checks without accessing personal contacts or answering a real permission prompt.

## Task 3: Contacts panel and existing call integration

**Files:** desktop/src/lib/contacts.ts, contacts-api.ts, components/ContactsPanel.svelte; minimal App.svelte import/branch/call helper; tests/ui/contacts.test.ts.

- [x] Write and run failing search tests for names, organization, formatted numbers, favorites, Unicode and namesakes.
- [x] Add permission/unsupported/loading/error/empty states; contact list, search, favorites filter and explicit phone-number buttons.
- [x] Refresh on entering the panel, returning focus and a bounded visible interval; never silently keep results after a failed/revoked read.
- [x] Use an oncall callback into existing perform/dial; keep panel-local styles and preserve concurrent UI edits.
- [x] Verify browser fixtures at 340×500 and 320×440, keyboard behavior, multiple numbers, favorite changes, denied access and busy/unregistered states.

## Task 4: Documentation, checks and review

- [x] Document actual API and limitations; record verification with synthetic data only.
- [x] Run UI tests, typecheck, build, versions, format check, type-generation stability, workspace clippy/tests and desktop build. Build native SDK before native feature checks; run Asterisk regression if call-path changes affect SIP/audio.
- [x] Obtain an independent review of contacts changes, fix material issues and rerun affected checks.
- [x] Report observed results, concurrent unrelated failures and the remaining manual macOS permission/device validation.

## Completion evidence

See ../../verification/2026-11-04-macos-contacts.md for observed checks, changed-file ownership, browser fixtures and the remaining real macOS permission test. No shared work was committed or reverted. Unknown native permission values are restricted; limited access is not exposed because the installed macOS SDK marks that iOS-only status unavailable.
