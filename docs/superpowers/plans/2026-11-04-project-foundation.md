# SIP Project Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a runnable Rust/Tauri 2/Svelte foundation, a clear baresip integration boundary, and Git/CI/release preparation without implementing telephone features.

**Architecture:** A Cargo workspace contains phone-core, baresip-adapter, and the Tauri desktop application. npm workspaces own the frontend. A read-only Tauri command exposes honest native engine readiness; TypeScript DTOs are generated from Rust.

**Tech Stack:** Rust 1.98.1 / edition 2024, Tauri 2, Svelte 5, TypeScript, Vite, npm, serde, ts-rs, GitHub Actions.

**Spec:** ../specs/2026-11-04-sip-mvp-design.md. The user's instruction to start authorizes this foundation subset. Full calling, native linking, persistence, contacts and calendar remain later milestones.

## Global Constraints

- macOS Apple Silicon and Windows x64 are the initial desktop targets.
- Work in the existing project directory. Initialize Git locally; do not create a remote or push.
- Own code is MIT. Preserve the original design document and identify scaffold scope in it.
- Keep unsafe/C work inside baresip-adapter when introduced. This foundation does not download/build/link baresip or claim telephony support.
- Reuse Tauri's generated configuration/icon conventions and Cargo workspace inheritance. Choose plain Svelte/Vite; a server or SvelteKit is unnecessary for this shell.
- Commit lockfiles, toolchain pins, generated IPC types and action commit pins. Do not commit credentials, build artifacts or installed dependencies.
- Configuration and generated scaffold get compiler, type, formatting and build checks; do not add tests that merely reproduce their implementation.

## Review Focus

- A successful desktop startup must still show that baresip is not linked.
- Frontend preview outside Tauri must show an explicit unavailable bridge, not a fake native response.
- Shared workspace layout must preserve correct Tauri build commands and artifact paths on both hosts.
- A release tag/version mismatch or failed matrix member must prevent creation of a release draft.
- Fork pull requests must run read-only checks without publishing or accessing signing secrets.

## Task 1: Workspace and typed native boundary

**Files:** Root Cargo.toml/rust-toolchain.toml/.gitignore; crates/phone-core/{Cargo.toml,src/lib.rs,examples/export_types.rs}; crates/baresip-adapter/{Cargo.toml,src/lib.rs,native/README.md}; native/README.md.

**Interfaces:** phone-core exports serializable and TypeScript-exportable AppInfo { version: String, telephony: TelephonyStatus }; TelephonyStatus::NotLinked serializes as "notLinked". baresip-adapter::status() returns TelephonyStatus::NotLinked. The example export_types writes the TypeScript declaration to apps/desktop/src/lib/generated/AppInfo.ts. No start_call interface exists yet.

- [ ] Initialize main with the existing design as baseline; create chore/project-foundation for implementation.
- [ ] Create manifests with inherited package metadata, shared dependency versions and root lockfile. The adapter depends on core; core has no Tauri/native dependency.
- [ ] Implement the small read-only status contract, its exporter and documentation. Export path resolves from CARGO_MANIFEST_DIR rather than caller cwd.
- [ ] Run cargo check/test/clippy for the two portable crates and generate types. Expected: compiler/linter success; generated union includes only the actual unavailable state.
- [ ] Commit the portable foundation.

## Task 2: Runnable desktop shell

**Files:** Root package.json/package-lock.json/.node-version/.npmrc/.editorconfig/Prettier configuration; apps/desktop/package.json/index.html/vite.config.ts/tsconfig.json/svelte.config.js/src/{main.ts,App.svelte,app.css,lib/api.ts}; apps/desktop/src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/main.json,permissions/app.toml,src/main.rs,src/lib.rs,src/commands.rs,icons/*}; scripts/check-versions.mjs.

**Interfaces:** Tauri get_app_info() -> phone_core::AppInfo uses the compiled package version and baresip_adapter::status(). The frontend calls it with invoke<AppInfo>("get_app_info"). Both layers use generated DTOs. The command is read-only with no request fields. Rust command permission is explicitly limited to the main window.

- [ ] Create the frontend and Tauri package using current official conventions, keeping only dependencies required by the shell.
- [ ] Show startup loading, real runtime info or a visible error. A browser-only preview reports that the desktop runtime is required. Exclude product features and fake accounts/calls.
- [ ] Configure loopback-only Vite, strict CSP, window defaults and macOS 14 minimum. Use the official starter's MIT-licensed placeholder icons with provenance.
- [ ] Add version consistency verification for root npm, Cargo workspace members, and Tauri version source; optionally require RELEASE_TAG to match v<version>.
- [ ] Document get_app_info request, response, enum meaning, failure and lack of side effects in docs/api/README.md.
- [ ] Install dependencies; run frontend typecheck/build and full workspace fmt/clippy/test. Check the generated IPC file is reproducible. Run a native Tauri build locally. Expected: a real macOS package, with telephony clearly unavailable.
- [ ] Commit the desktop shell.

## Task 3: GitHub automation and contributor handoff

**Files:** .github/workflows/{ci.yml,desktop-build.yml,release.yml}; .github/actions/setup/action.yml; .github/{dependabot.yml,pull_request_template.md}; LICENSE; README.md; CONTRIBUTING.md; AGENTS.md; docs/{architecture.md,development.md,ci-cd.md,research/2026-11-04-foundation-sources.md,releases/foundation.md}; tests/interop/README.md.

**Interfaces:** A reusable workflow performs checks and builds native installers on macos-15 arm64 and windows-2022 x64. Pull requests/main use read-only permissions and upload build artifacts. Version tags call the same checks/builds; a separate job with contents:write creates only a draft after every prerequisite passes. No auto-publishing or updater endpoint is configured.

- [ ] Add root commands for development, frontend checks/build, DTO generation, formatting and native packaging. Keep workflow commands aligned with documentation.
- [ ] Pin GitHub actions to verified commits; configure caches and immutable dependency installation. Set concurrency and timeouts. Exclude secrets and write permissions from PR builds.
- [ ] Prepare unsigned/ad-hoc development artifacts and a tag-triggered draft release. Verify the tag matches all version sources. Download artifacts only from this workflow run; create the draft only after both platform builds pass. Never overwrite an existing release.
- [ ] Add MIT, ignore rules, contributor/agent guidance, branch-protection instructions, Dependabot, PR template, current limitations and signing/updater prerequisites. Do not invent remote URLs, maintainer identity or test results.
- [ ] Validate workflow syntax and action inputs, version mismatch rejection, lockfiles, generated types, formatting, typecheck and local native packaging. Windows and hosted GitHub executions remain pending until a remote run.
- [ ] Request one independent code review; resolve material findings and rerun affected checks. Commit the verified foundation; leave it on the feature branch without publishing.

## Verification record

Execution evidence and any justified deviations are recorded in this plan's ignored .superpowers workspace during implementation and summarized here at completion. No hosted CI success or working SIP call may be inferred from local compilation.
