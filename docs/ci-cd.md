# Git and CI/CD

Git is initialized locally with the MVP design on `main`. The foundation is on
`chore/project-foundation`. No remote is assumed and no push is performed by setup.
Connect the actual repository before pushing or opening a pull request.

## Workflows

- **CI**: pull requests, pushes to main/the foundation branch and manual runs.
  Portable Linux checks validate formatting, versions, generated DTOs, frontend
  typecheck/build, portable Rust linting and tests.
- **Desktop build**: reusable workflow called by CI and release; macOS arm64 and
  Windows x64 build the pinned native SDK, run native workspace Clippy/tests and
  produce DMG/NSIS artifacts with the native feature. A Linux Docker job verifies
  authenticated Asterisk calling and returned 440 Hz RTP audio. Hosted
  compilation does not qualify audio, hardware integration or SIP compatibility.
- **Draft release**: a pushed `v<version>` tag must match package/Cargo versions.
  Both native jobs and the Asterisk interoperability job must pass before a separate write-enabled job downloads this
  run's two installers and creates a **draft** GitHub release. It does not publish.
  An existing release is not updated; correct/delete an obsolete draft explicitly
  before retrying draft creation. `docs/releases/foundation.md` is the current
  release-notes source and must be updated as real features ship.

The shared setup installs the pinned Bun version from `.bun-version` and runs
`bun install --frozen-lockfile`; frontend tools, native build orchestration and
formatting run through Bun. Dependabot monitors the `bun` ecosystem/text lockfile.

All remote actions are pinned to commits, with Dependabot enabled for updates.
PR workflows have read-only content permissions, do not persist checkout
credentials and do not receive signing secrets. Draft creation alone uses write
permission. Fork PRs never run the tag release path.

## Repository setup after connecting GitHub

Enable Actions. Protect `main` with a ruleset requiring a pull request, review and
the actual check names from the first workflow run. Include Portable checks and
both desktop jobs and Asterisk SIP/RTP echo; prohibit force pushes. The files cannot enforce server-side
rules before a repository exists. Dependabot should be enabled there as well.
Enable private vulnerability reporting and publish a maintained private contact
as described in [SECURITY.md](../SECURITY.md). Community files and issue templates
are versioned here; they do not activate GitHub settings by themselves.

Do not tag/publish until the app identifier, release notes and platform policies
are ready. A version tag produces a development phone build; the complete MVP, hardware
qualification and production distribution policies remain unfinished.

## Signing and updates

macOS uses ad-hoc signing (`-`) for development. Windows installers are unsigned.
Public distribution needs Apple Developer ID signing/notarization and a selected
Windows signing service/certificate. Add credentials to GitHub Secrets only when
implementing those paths; never commit certificate files or passwords. The current
workflows do not claim to implement production signing.

Automatic updates remain disabled. Enabling them requires the updater plugin,
a real endpoint and a protected updater signing key with a public key in the app.
Neither repository URLs nor signing identities are invented in this scaffold.

## Verification limits

Local checks can validate workflow syntax and build the current host's package.
Only execution on GitHub can verify hosted cache, permissions, Windows packaging
and release upload behavior. Record those results after the first connected run.
