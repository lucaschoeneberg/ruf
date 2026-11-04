# Native baresip and Asterisk Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Link real baresip into the desktop application, prepare macOS/Windows SDK builds, and prove an authenticated SIP call with echoed RTP audio against an isolated local Asterisk.

**Architecture:** Keep the approved library/C-shim approach. Rust owns a safe engine handle; baresip/libre run on one dedicated thread. C operations are marshalled through libre's message queue. serde/ts-rs contracts and the existing Tauri IPC boundary are reused. One ephemeral account and one call are sufficient for this milestone.

**Tech Stack:** Existing Rust/Tauri/Svelte workspace; baresip and libre 4.12.0 pinned to full source commits; CMake, OpenSSL static libraries, a minimal C shim and the Rust cc build helper; Docker Compose for Asterisk.

**Spec:** ../specs/2026-11-04-sip-mvp-design.md. The user explicitly authorizes the native milestone and first local Asterisk call. Follow-on production features remain outside this milestone.

## Global constraints

- Reuse the current checkout and implementation branch; preserve existing user work and existing Docker containers.
- Native support is a Cargo feature. Portable builds continue to report notLinked; a linked library is not the same as a registered account.
- Initial account transport is explicitly SIP/UDP in an isolated test network, with G.711 audio. No TLS/SRTP support claim in this milestone and no silent transport downgrade.
- Credentials stay in process memory, never in account URIs, disk configuration, API responses, Debug output or logs. Account password is supplied through the dedicated upstream setter after allocation.
- Raw pointers and unsafe Rust remain private to baresip-adapter. All C handles/callbacks remain valid until the engine thread joins; prevent Rust unwinding across C.
- Use actual pinned upstream headers rather than copying the old wiki example's event signatures.
- Desktop calling controls use existing styles and generated contracts. Calendar, contact/history storage, multiple accounts, transfers and production distribution stay deferred.
- Headless echo-tone audio proves an actual network/media path. Physical microphone/headset validation remains a distinct manual test.

## Review focus

- Invalid registrar/user/password or destination must not inject SIP account parameters or config lines.
- Two overlapping calls and account replacement during a call must be rejected without dropping the existing call.
- Startup failure, shutdown and callback ownership must not leak a thread or use freed memory.
- A SIP 200 OK alone is insufficient: the Asterisk echo test must receive non-silent audio matching the sent tone.
- Windows SDK linking/static OpenSSL selection and the macOS system-framework links must be explicit; unexecuted hosted Windows tests must be reported as such.

## Task 1: Native SDK and authenticated local lab

**Files:** native/{sources.json,build.mjs,vcpkg.json,README.md}; crates/baresip-adapter/{Cargo.toml,build.rs}; tests/interop/asterisk/*; package.json; ignore rules.

**Interfaces:** SDK at native/install/<Rust target>/ with include/re, include/baresip.h, lib and license resources. Baresip source commit 9cfab81550acc4bd38d6580d79b45cc6eea65cd6; libre commit 65555dad439de9d6a81b3ee6092b9c096585249b. Configure static builds with only G.711, format/resampling, platform audio and deterministic tone/file audio modules. Windows OpenSSL uses an explicitly baselined vcpkg manifest; macOS uses the explicitly resolved local OpenSSL prefix and records it.

- [x] Implement a cross-platform Node build recipe that fetches exact source revisions into ignored native/build, configures CMake and stages only required development files/static libraries/notices. Abort on command or source identity mismatch.
- [x] Build the SDK locally; preserve a build manifest with versions, target and selected modules. Prepare Windows MSVC x64 paths without claiming that they ran locally.
- [x] Add a separately named Docker Compose Asterisk lab, pinned image digest, loopback-only published ports and synthetic local-only credentials. Add an authenticated extension and Echo dialplan. Do not stop or alter unrelated containers.

## Task 2: Safe engine and observable state

**Files:** phone-core contracts/validation/tests/exporter; baresip-adapter native bridge/private FFI/safe Engine and tests/examples.

**Interfaces:** AccountConfig input with registrar, username, password and audio mode; password-free EngineSnapshot with registration/call states and safe error information. Engine::start, configure_account, dial, answer, hangup, snapshot and shutdown. Only one native engine instance per process. C ABI has explicit create/run/request/destroy ownership and numeric owned events.

- [x] Write failing tests for invalid input/injection rejection, credential redaction, call-state preconditions and unavailable-feature behavior. Implement only the required behavior, then run the suite.
- [x] Add cc build integration and a small C adapter against actual headers. Initialize in-memory config, static modules and libre's queue on the engine thread; use account_set_auth_user/pass and explicit registration.
- [x] Implement safe event/state mapping, singleton lifetime and teardown. Add real feature-gated lifecycle tests; exercise repeated startup/shutdown and failure cleanup.
- [x] Add an example that reads the synthetic test password from environment, registers, dials the echo extension, waits for connection, records incoming tone audio, hangs up and exits with failure on timeout or native error. Never print the password.
- [x] Run the real example against Asterisk and verify the resulting WAV duration/amplitude/frequency; distinguish this from hardware audio verification.

## Task 3: Desktop API and native CI builds

**Files:** Tauri Cargo features/state/commands/permissions/platform microphone description; Svelte call controls/api; DTO exporter; .github workflows; API/native/lab/development/licensing documentation.

**Interfaces:** get_app_info retains its existing contract with an added linked state. New commands configure_account, get_engine_snapshot, dial, answer and hangup use generated Rust DTOs. The main-window capability explicitly permits only these commands. Errors return stable safe codes/messages; no HTTP status codes are invented.

- [x] Wire a managed engine into Tauri, keeping blocking native operations off the UI thread. Release the engine on process exit.
- [x] Add simple ephemeral account, number, answer/hangup and real status controls. Show the limited UDP/test-network scope and explicit unavailable states in browser/portable builds.
- [x] Build native SDK before macOS/Windows workspace checks and packaging; turn on native feature for those jobs. Preserve the separate portable Linux checks. Stage native notices into bundles and document signing/microphone permissions.
- [x] Update API contracts, generated TypeScript, lab instructions, actual compatibility evidence and scope limitations.
- [x] Run formatting, generated-contract drift check, types/build, portable/native Rust tests, Clippy, workflow validation, native app packaging and the Asterisk media test.
- [x] Request independent whole-change review, fix material findings with regression evidence, commit locally and report unverified platforms/hardware honestly.
