# GitHub Actions verification — 10 November 2026

Verified source: [`3fa7fc0d76a1fefa7e260f62e74ebe37391bfe0f`](https://github.com/mardu-systems/ruf/commit/3fa7fc0d76a1fefa7e260f62e74ebe37391bfe0f).
All five jobs in [CI run 42904931181](https://github.com/mardu-systems/ruf/actions/runs/42904931181)
completed successfully under `mardu-systems/ruf`.

## Hosted results

| Job                                     | Runner                  | Result | Duration   |
| --------------------------------------- | ----------------------- | ------ | ---------- |
| Portable checks                         | Ubuntu 24.04            | Passed | 58 s       |
| Desktop UI state regressions            | Ubuntu 24.04            | Passed | 1 min 31 s |
| Asterisk SIP and RTP echo               | Ubuntu 24.04            | Passed | 7 min 55 s |
| Native macOS build and DMG              | macos-15, Apple Silicon | Passed | 9 min 11 s |
| Native Windows build and NSIS installer | windows-2022, x64       | Passed | 10 min 4 s |

Durations include job setup and cleanup. Desktop jobs validated restored native
SDKs, ran the native workspace Clippy/test checks, packaged installers and uploaded
their software inventories and hash manifests. The macOS job also passed signed
microphone/Contacts bundle-metadata checks. Portable checks covered workflow
syntax, versions, generated-contract consistency, formatting, UI unit tests,
TypeScript/Svelte, frontend build and portable Rust Clippy/tests. The separate
browser job and real native Asterisk SIP/RTP scenarios also passed.

## Corrections and controls

- [The first fresh main run](https://github.com/mardu-systems/ruf/actions/runs/42897457594)
  reproduced sparse synthetic PCM on macOS. A background process could coalesce
  the fixture's 20 ms sleeps to about 120 ms, starving its RTP stream. The macOS
  test sender now uses `EVFILT_TIMER` with `NOTE_CRITICAL`. The existing one-second
  PCM requirement, RMS/frequency bounds and packet pacing remain enforced.
  `libc` 0.2.191 was already locked transitively; it is now an explicit macOS-only
  development dependency. No runtime dependency version changed.
- Subsequent hosted runs exposed fixed-delay teardown assertions in the
  [481 hold case](https://github.com/mardu-systems/ruf/actions/runs/42900597125)
  and [malformed SDP ACK case](https://github.com/mardu-systems/ruf/actions/runs/42902709396).
  A complete local background run also exposed the transfer-result case.
  These cases and the sibling unsupported-codec teardown now use the existing
  five-second bounded `Lab::wait` rather than treating 150 ms of peer servicing
  as proof that native audio workers have finished closing. Error classification
  and the exact-one transfer-success-event assertion remain mandatory.
- With Rust 1.98.1, all five matching PCM tests passed under `taskpolicy -b`.
  Removing `NOTE_CRITICAL` temporarily reproduced the baseline-tone failure;
  restoring it passed the tests again. Each of the four corrected terminal-state
  cases passed 25 consecutive background repetitions.
- The complete final local SIP suite passed **127 tests, zero failures and zero
  ignored tests** under forced background scheduling with Rust 1.98.1.
  The final native macOS app/DMG also built locally and passed microphone/Contacts
  bundle checks and installer/SBOM hash validation.
- Independent reviews found no blocking issues. No desktop API, DTO or production
  SIP/audio behavior changed.

## Downloaded artifact validation

Both downloaded installer hashes matched their `.build.json` manifests. Each
manifest's SBOM hash matched the actual `.cdx.json` file; embedded build properties
reported the expected native release target and Rust 1.98.1. Cargo/Bun lockfile
hashes matched the frozen source revision above.

| Artifact                                                                                       | Installer                 | SHA-256                                                            |
| ---------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------------------------------------------ |
| [macOS](https://github.com/mardu-systems/ruf/actions/runs/42904931181/artifacts/13804414006)   | `Ruf_0.1.0_aarch64.dmg`   | `d2616345a7d5fd868132a8146c190a06fe36f13e30a0a387883eb61de069b5f4` |
| [Windows](https://github.com/mardu-systems/ruf/actions/runs/42904931181/artifacts/13805496428) | `Ruf_0.1.0_x64-setup.exe` | `3c10db2d976ed4215a3e7d4c46134bc7edcdf0e3b73f48074bbde98eb0628144` |

## Boundaries

These results qualify the recorded source and artifacts for the automated checks
listed above. Installed Windows 11 behavior, physical audio devices, provider
compatibility, macOS 14 hardware and interactive OS permissions still require
their own acceptance tests. macOS signing remains ad hoc; Windows installers are
unsigned. Production signing, notarization, App Store approval and the separate
draft-release upload workflow were not qualified by this CI run. Checksums verify
downloaded bytes and do not authenticate the publisher.
