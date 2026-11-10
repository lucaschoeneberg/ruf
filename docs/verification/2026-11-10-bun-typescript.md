# Bun and TypeScript tooling migration, 10 November 2026

These observations apply to the local, uncommitted migration working tree on
macOS arm64 with Bun 1.4.4 and Rust 1.98.1. They do not establish hosted CI,
Windows runtime, App Store signing, notarization or hardware audio results.

## Change

All 36 tracked MJS sources moved to TypeScript. Package commands, imports,
documentation and native CI cache paths use the new extensions. The framework's
`svelte.config.js` remains JavaScript. `bun run check` now includes strict tooling
checking; [command contracts](../api/tooling.md) describe its scope and failures.
Existing generated Rust DTOs supply the browser-fixture and test types.

File reads/writes, hashing, module paths and the native test runner's child
processes use Bun APIs where applicable. Directory operations and subprocess
calls with established timeout/stdio behavior retain their existing Node APIs.

## Observed checks

| Check                                                            | Observed result                                                                                                                                                             |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun install --frozen-lockfile`                                  | Passed; no lockfile changes on frozen installation.                                                                                                                         |
| `bun run check`                                                  | Passed tooling TypeScript, Svelte (zero errors/warnings) and desktop TypeScript checks.                                                                                     |
| Typecheck negative control                                       | A temporary script assigning a number to a string failed with TS2322 and a nonzero exit; the probe was removed.                                                             |
| `bun run format:check`                                           | Passed Bun and Cargo formatting.                                                                                                                                            |
| `bun run check:versions`                                         | Passed with the pinned Rust toolchain first on PATH.                                                                                                                        |
| `bun run types:generate` and generated-contract diff             | Passed; generated contracts unchanged.                                                                                                                                      |
| `bun run test:ui`                                                | 112 passed, zero failed; 1,359 assertions.                                                                                                                                  |
| `bun run test:ui:flows`                                          | All nine browser flows passed.                                                                                                                                              |
| Serialized browser installers                                    | All 11 installers executed after serialization/transpilation in an isolated VM; two projection unit tests passed.                                                           |
| `bun run build`                                                  | Passed frontend production build.                                                                                                                                           |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed.                                                                                                                                                                     |
| `cargo test --locked --workspace`                                | Passed; 126 tests, one ignored, including doc tests.                                                                                                                        |
| `bun run native:build`                                           | Passed with the existing macOS-14-compatible static OpenSSL installation.                                                                                                   |
| Native workspace Clippy                                          | Passed with `--features sip-desktop/native`.                                                                                                                                |
| `bun run test:native`                                            | Passed all three partitions; 300 tests passed, zero failed, 20 ignored.                                                                                                     |
| Native runner failure controls                                   | A synthetic Cargo executable verified successful aggregation and nonzero partition failure, both output streams from all three children, and retained failure logs.         |
| `bun run test:asterisk`                                          | Passed registration/call and returned-audio checks, 16 opt-in lab tests, 13 security tests and the saved-account restart regression. PCM: 4.04 seconds, RMS 5811, 440.1 Hz. |
| `bun run test:network-roam`                                      | Passed six network cases, call-action ownership checks and 65,536 DNS refreshes.                                                                                            |
| `bun run test:sip-diagnostics`                                   | Passed 64 allowlisted diagnostic markers and opt-in behavior.                                                                                                               |
| macOS timeout/login helpers                                      | Contacts and notification timeout scenarios and login-item policy tests passed without real login-item mutations.                                                           |
| `bun run desktop:build`                                          | Produced the portable App and DMG; SBOM/build-context validation passed.                                                                                                    |
| `bun run desktop:build --features native --bundles app`          | Produced the native App; SBOM/build-context validation passed.                                                                                                              |
| Bundle metadata checks                                           | Signed microphone entitlement/purpose string and Contacts entitlement/purpose/signature checks passed against the native App. No microphone was opened.                     |
| App Store helpers                                                | Unsigned `--check` preflight and five synthetic browser screenshot captures passed. No upload/submission occurred.                                                          |
| Native cache fingerprint                                         | Old and migrated scripts produced byte-identical output on macOS with identical synthetic hosted-image metadata.                                                            |
| Independent code review                                          | Reviewed all 36 migrations, strict-check coverage, serialized closures, subprocess options, dependency metadata and CI/docs references; no unresolved findings.             |

## Corrections and limits

The browser run exposed an existing stale German changelog-heading expectation;
the bundled changelog uses `0.1.0 · Preview`. The assertion now matches that
existing content; all nine flows subsequently passed.

The default Homebrew Rust executable did not match the project pin. Checks used
Rustup's installed 1.98.1 toolchain. Homebrew's OpenSSL archives require a newer
macOS deployment target and were correctly rejected; the successful SDK rebuild
used the compatible installation already documented in `native/README.md`.

The portable desktop build completed with the existing local `rust-objcopy`
`libLLVM.dylib` warning. Bundles were locally ad-hoc signed and not notarized.
Windows execution and hosted CI for this patch remain unobserved.
