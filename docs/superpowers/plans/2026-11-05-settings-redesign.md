# Ruf Settings Redesign Implementation Plan

> Agentic workers: execute with superpowers:executing-plans; TDD and independent final review. User approved this full plan and a separate managed worktree on 2026-11-05.

**Goal:** Eight clear settings areas with reliable permission checks, self-service diagnostics, account activation, audio tests and privacy-preserving local preferences.
**Architecture:** Existing Svelte 5 panels and guarded drafts; Tauri 2 typed IPC; phone-core Rust contracts generated with ts-rs; all native audio/SIP in baresip-adapter.
**Spec:** The approved plan below is the implementation specification.

## Global Constraints

- Preserve the separate always-visible Deine Konten block below settings navigation, add button, direct account links and max eight accounts. No collapsible account hierarchy or second sidebar.
- Preserve dark palette/IBM Plex Sans/native windows, explicit saves, draft protection and identities; support 900x660 and 740x540 with independent account scrolling.
- Existing routes general/accounts/audio/contacts/about remain compatible; add notifications/privacy/diagnostics.
- Never auto-prompt OS permission; read on start/open/focus. Explicit action may request undecided permission. Denial opens OS settings instead. Failed reads are unknown, never invented denial.
- No new dependencies or second audio pipeline. Never export passwords, SIP addresses, personal calls or contacts. No external user-assisted test calls, permission resets or production credential changes.
- Migrate metadata v1 losslessly to v2 with backup and atomic replacement before new preference persistence; preserve keys and selected account. Defaults retain current behavior.
- Keep worktree for review; do not merge/push/install or commit automatically.

## Review Focus

1. OS permission query must work despite broken account metadata/keychain and preserve all drafts.
2. Late permission/device results and account activation callbacks cannot overwrite newer intent.
3. Incoming calls preempt native audio tests; no test callback outlives owned audio resources.
4. Disabled accounts retain credentials/selection and cannot dial or silently switch identity.
5. Retention never removes in-progress records, and newly shortened retention requires confirmation.

### Task 1: Navigation and settings structure

- [x] RED/GREEN tests: eight routes; separate account block; field search; draft protection; minimum sizes.
- [x] Split domain panels while retaining existing form/guard APIs; central typed searchable field registry; select results opens route, scrolls/focuses field; protect dirty navigation.
- [x] Stable full width and persistent save/discard actions; per-domain loading/errors. Keep existing SIP-account form unchanged until Task 4.
- [x] Verify UI tests, typecheck and browser settings/layout flows.

### Task 2: Notification permission and test notification

- [x] RED/GREEN tests for all canonical permission states, failed reads, off preference, focus refresh, stale reads and preservation of drafts.
- [x] Add get_notification_permission independent of PhoneSession/store, using existing native permission(false) and NotificationPermission. Preserve request_notification_permission response contract; re-read canonical permission even if preference retrieval after authorization fails.
- [x] Dedicated notification panel: application preference and OS status always separate; explicit check/request/open-settings actions. A foreground hint links to this panel only when app notices are enabled but OS access is missing.
- [x] Add fixed-platform open_notification_settings and explicit test_notification: five-second delay, fixed anonymous text, distinct identifier, no call actions/CallState change; check authorization, do not claim banner delivery from enqueue success.
- [x] Caller-details preference defaults true; false removes caller name/number from all notification text. Add v2 persisted defaults and document new IPC/capabilities.
- [x] Verify UI, native notification boundary tests, desktop checks and bundle.

### Task 3: Privacy and diagnostics

- [x] RED/GREEN for independent permission errors and privacy-safe diagnostics.
- [x] Reuse ContactsSettings permission actions inside privacy; add native microphone permission metadata/query/explicit request. No contact fetch or microphone capture merely opening settings.
- [x] Diagnostic view distinguishes compiled linkage, confirmed account registration, keychain access, device availability and notification authorization. Read-only help remains usable during unrelated operations.
- [x] Offer explicit reconnect for registration failures without resaving unchanged credentials. Expose confirmed-registration timestamp; never promise incoming reachability from registration alone.
- [x] Whitelisted diagnostic preview/copy: version/platform/linkage/counts/error codes only; no full snapshots, contacts, phone numbers, credentials or raw addresses. Add existing keyboard-shortcut help; move storage descriptions here/privacy, retain compact About/SBOM/Mardu links.
- [x] Verify sanitization, errors, UI flows and typed documentation.

### Task 4: Account activation and startup

- [x] RED/GREEN native tests for disabled restoration, queued/incoming-call races, late events and selected identity.
- [x] Add default-true saved/snapshot account enabled state and native acknowledged toggle, preserving account ID/credential metadata. No removal path for disable, no credential read for disabled restoration; active calls reject toggles. Roll back durable intent on native rejection.
- [x] Overview/default-account selection uses existing select API; disabling the selected account retains selection and blocks dial until re-enabled or manually changed. No automatic caller-identity switch.
- [x] Startup preference applies only to login launches; default remains hidden. Manual open shows phone; explicit --background and notification activation keep their semantics. Update only Ruf-owned enabled login entries and respect disabled autostart.
- [x] Verify native loopback/Asterisk, startup policies and v1 migration.

### Task 5: Native audio diagnostics and ringtone

- [x] RED/GREEN native tests for bounded test ownership/preemption and tone generation.
- [x] Microphone meter: explicit start, RAM-only normalized level at <=10Hz, max ten seconds; native baresip ausrc on existing event loop, never browser capture/recording. Stop on close/navigation/new incoming call/engine shutdown.
- [x] Speaker test: two seconds on selected call output; separate existing ringtone preview. Exclusive test audio ownership; real incoming call always wins.
- [x] Add classic/soft/short built-in PCM ringtones and ring volume 0..100; default classic/100 reproduces existing tone amplitude. Do not change OS/call volume.
- [x] Verify synthetic input levels, PCM amplitudes, resources and SIP interop; document unobserved hardware/Windows limits honestly.

### Task 6: Contacts and history preferences

- [x] RED/GREEN for default-preserving contact display and retention boundaries.
- [x] Persist favorites-first/company-visible display preferences with current defaults; reuse existing contact model/provider and favorites.
- [x] Expose existing clear_history in settings with confirmation; retain current active-call protection.
- [x] History recording defaults enabled; changes allowed while idle and apply only to future calls. Existing history is retained when recording is disabled.
- [x] Retention: unlimited/30/90/365 days, default unlimited. Explicitly confirm shorter retention before saving and immediate cleanup; existing history worker prunes completed records by start timestamp on startup/config change and daily, never inProgress records. Preserve event ordering and surface failures separately from SIP.
- [x] Verify migration, failure rollback, recording/retention and UI flows.

## Completion

Run format:check, check, build, check:versions, types:generate twice (second stable), portable/native clippy/tests, native SDK build before native checks, Asterisk for SIP/audio, desktop native build. Review the feature delta independently against the captured source baseline. Impeccable clarify/harden/adapt/polish: bounded visual check at both shipped sizes, keyboard/focus/zoom; at most two batched visual correction passes. Keep all six tasks tracked; do not stop after permission changes.
