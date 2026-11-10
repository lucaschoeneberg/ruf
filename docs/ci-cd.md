# Git and CI/CD

The 0.1.0 preview is hosted in the private
[lucaschoeneberg/ruf repository](https://github.com/lucaschoeneberg/ruf).
The initial release-preparation snapshot was pushed to `main` on 10 November 2026.
Setup does not push, tag or publish a release.

## Workflows

- **CI**: pull requests, pushes to main/the foundation branch and manual runs.
  The reusable `quality.yml` runs portable Linux checks for formatting, workflow
  syntax, versions, generated DTOs, frontend typecheck/build and portable Rust
  linting/tests, plus a separate browser-state regression job.
- **Desktop build**: reusable workflow called by CI and release; macOS arm64 and
  Windows x64 build the pinned native SDK, run native workspace Clippy/tests and
  produce DMG/NSIS artifacts with the native feature, plus each target’s compiled
  SBOM and build-hash manifest. A Linux Docker job verifies
  authenticated Asterisk calling and returned 440 Hz RTP audio. Hosted
  compilation does not qualify audio, hardware integration or SIP compatibility.
- **Draft release**: a pushed `v<version>` tag must match package/Cargo versions.
  Versioned notes at `docs/releases/<version>.md` must exist. The same reusable
  quality checks as branch CI, both native jobs and the Asterisk interoperability
  job must pass before a separate write-enabled job downloads this run's two
  installers and creates a **draft prerelease**. It does not publish.
  An existing release is not updated; correct/delete an obsolete draft explicitly
  before retrying draft creation. [0.1.0 notes](releases/0.1.0.md) describe this
  preview; add notes matching each new version before tagging. The draft includes
  `SHA256SUMS` with plain installer filenames, verifiable after downloading all
  installers and the checksum file into the same directory. Matching per-target
  SBOM and build-hash manifests are also attached. Checksums do not authenticate a publisher.

macOS builds request both `app,dmg`: Tauri retains Ruf.app so the signature and
permission checks run against the finished bundle before uploading the DMG.
DMG-only packaging removes its temporary app and cannot support those checks.
Desktop jobs allow 60 minutes: the first complete cold Windows run took almost
45 minutes, including native SDK compilation, tests, packaging and cache upload.
This is a cold-build budget. The installed native SDK is cached separately with
an exact compiler/image/input key and saved immediately after qualification.
Cache hits reuse the existing manifest/archive checks and rerun its two NAT
regressions; application compilation and tests remain unconditional. Rust
dependency caches are also saved after failed non-PR runs so a later test failure
does not force the same dependency compilation again.

Native test execution builds once and runs the existing NAT, SIP-event and
remaining tests in separate processes through `bun run test:native`. Linux uses
`--adapter`. All partitions must pass; workspace integration/doc tests remain in
the remaining partition, and the Asterisk job still runs its opt-in scenarios.

Native workspace tests use the job's temporary directory. On failure, the
desktop jobs retain only the three explicitly named early-media/refused-hold
WAV recordings for seven days. These fixtures contain generated test tones;
the recordings help distinguish sender scheduling from native media failures.

The shared setup installs the pinned Bun version from `.bun-version` and runs
`bun install --frozen-lockfile`. It explicitly installs the Rust toolchain from
`rust-toolchain.toml`, without relying on Rustup's deprecated auto-installation.
Frontend tools, native build orchestration and
formatting run through Bun. Dependabot monitors the `bun` ecosystem/text lockfile.

All remote actions are pinned to commits, with Dependabot enabled for updates.
PR workflows have read-only content permissions, do not persist checkout
credentials and do not receive signing secrets. Draft creation alone uses write
permission. Fork PRs never run the tag release path.

## Repository settings

Enable Actions. Protect `main` with a ruleset requiring a pull request, review and
the actual check names from the first workflow run. Include Portable checks and
both desktop jobs and Asterisk SIP/RTP echo; prohibit force pushes. The files do
not activate server-side rules themselves. Dependabot should be enabled there as well.
Enable private vulnerability reporting and publish a maintained private contact
as described in [SECURITY.md](../SECURITY.md). Community files and issue templates
are versioned here; they do not activate GitHub settings by themselves.

Follow the [release checklist](release-checklist.md) before tagging or publishing.
A version tag produces a preview draft; hardware qualification and production
distribution policies remain unfinished.

## Signing and updates

macOS uses ad-hoc signing (`-`) for development. Windows installers are unsigned.
Public distribution needs Apple Developer ID signing/notarization and a selected
Windows signing service/certificate. Add credentials to GitHub Secrets only when
implementing those paths; never commit certificate files or passwords. The current
workflows do not claim to implement production signing.

Automatic updates remain disabled. Enabling them requires the updater plugin,
a real endpoint and a protected updater signing key with a public key in the app.
Neither repository URLs nor production signing identities are invented.

## Verification limits

Local checks can validate workflow syntax and build the current host's package.
Only execution on GitHub can verify hosted cache, permissions, Windows packaging
and release upload behavior. Record those results after the first connected run.

## Build-specific software inventories

The supported entrypoint is `bun run desktop:build`; it checks pinned versions
and starts the native CLI in a child process whose OS environment already contains
the selected target/profile/native context. This also avoids Bun/N-API environment
snapshot differences. No shell interpolation or arbitrary URLs are involved.
The compiled Cargo build validates that context before embedding the frontend.
After success, a CLI-only export reads the SBOM from the executable itself. A
hash manifest associates that JSON with the executable and current installers.
Both artifact collection and draft creation include one SBOM and one hash manifest
per target. Hosted execution is still required to verify Windows packaging and
GitHub artifact/upload behavior.
