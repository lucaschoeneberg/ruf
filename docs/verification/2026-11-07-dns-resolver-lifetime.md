# DNS resolver lifetime correction — 7 November 2026

## Cause and correction

The installed Ruf process crashed at 12:27:58 Europe/Berlin in
`notify_register_check`, reached through libre's `get_resolv_dns` and the
existing idle DNS refresh timer. The macOS diagnostic identified an exceeded
libnotify registration limit for DNS configuration notifications.

The pinned libre resolver initialized temporary state with `res_ninit` but
released only its sockets with `res_nclose`. On macOS, `res_ndestroy` also
releases the notification token and extended resolver allocations. The existing
`re-reliability.patch` now uses it in the shared cleanup path, including errors
after initialization. Other platforms retain their previous cleanup.

The five-second idle poll, active-call deferral, native media pipeline, public
APIs and generated DTOs are unchanged. The existing SDK manifest and desktop
SBOM checks require the exact updated patch; stale SDKs must be rebuilt.

## Regression evidence

The new macOS C fixture calls the installed SDK's real `dns_srv_get` 65,536
times and validates successful, nonempty server lists. This reads system DNS
configuration; it sends no DNS queries and uses no accounts or personal data.
Assertions remain enabled in Release builds. The existing network-roam runner
executes it on macOS and rejects signals, nonzero exits and a two-minute timeout.
Existing desktop CI already invokes that runner after building the SDK.

- Before the fix: the fixture terminated with SIGTRAP. The macOS diagnostic
  confirmed the same exceeded DNS notification registration limit.
- After rebuilding the SDK: all 65,536 refreshes completed successfully,
  together with the six existing network recovery tests and call-action fixture.
- The rebuilt desktop executable imports `res_9_ndestroy` and `res_9_ninit`;
  its SDK manifest matches the exact updated patch. The native app bundle and
  strict code-signature verification completed successfully.

This reproduces and verifies the specific failure under accelerated repetition;
it is not a multi-day desktop soak test or a guarantee against unrelated crashes.

## Files changed for this correction

- `native/patches/re-reliability.patch`: macOS resolver destruction.
- `crates/baresip-adapter/native/tests/dns_refresh.c`: real resolver stress test.
- `crates/baresip-adapter/native/tests/CMakeLists.txt`: macOS fixture target.
- `scripts/check-network-roam.ts`: run the fixture against the installed SDK.
- `native/patches/README.md`, `docs/api/sip-events.md`, `docs/development.md`:
  cleanup behavior, compatibility and repeatable verification instructions.
- This report: observed evidence and remaining limits.

## Checks observed in this session

- `bun run native:build`: passed, including the existing native NAT tests.
- `bun run test:network-roam`: failed with the old SDK, passed after correction.
- `bun run format:check`, `bun run check`, `bun run build`: passed.
- `bun run check:versions`, `bun run types:generate`: passed with pinned
  Rust 1.98.1; generated TypeScript files are unchanged.
- `cargo clippy --locked --workspace --all-targets -- -D warnings`: passed.
- `cargo test --locked --workspace`: passed; one OS device test ignored.
- Native workspace Clippy with `--features sip-desktop/native`: passed.
- `cargo test --locked --workspace --features sip-desktop/native`: passed;
  184 adapter tests and 89 desktop tests, plus core/contract tests. The command
  explicitly ignored 15 adapter lab tests and five desktop OS/credential/lab
  tests; lab coverage was executed separately by the Asterisk runner below.
- `bun run test:asterisk`: passed, including measured 440 Hz RTP echo,
  16 lab cases, 13 TLS/security cases and the saved-account restart case.
- `bun run test:ui`: 97 passed, zero failed.
- `bun run test:ui:flows`: all nine browser flow groups passed.
- `bun run test:macos-login-item`: passed with synthetic service responses and
  no real login-item mutations.
- `bun install --frozen-lockfile`: passed.
- `bun run desktop:build --features native --bundles app`: passed, including
  SBOM export; strict app signature verification passed.

The staged content passed `git diff --cached --check`. The credential/build-data
scan found no matching sensitive material or runtime artifacts. Intentional
Markdown hard breaks in the existing privacy-policy draft use equivalent
backslash syntax so the whitespace check also passes for newly staged files.

## Remaining limits

Checks ran locally on macOS arm64. No hosted CI, Windows runtime, physical
microphone/speaker or multi-day GUI execution is claimed. The built bundle is
under `target/aarch64-apple-darwin/release/bundle/macos/Ruf.app`; the installed
application in `/Applications` was not replaced. Packaging emitted the existing
local `rust-objcopy`/missing-LLVM stripping warning and used ad-hoc signing without
notarization; the build and signature checks exited successfully. No credentials,
personal call/contact data, crash logs or generated build outputs are committed.
