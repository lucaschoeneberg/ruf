# Local release checks · 10 November 2026

Fresh local checks of the uncommitted 0.1.0 working tree based on
`1aafead527daee87d1aea0b38e497cf6f4027fcb`. Host: macOS 27.2 (26B123), Apple Silicon;
Rust/Cargo 1.98.1 selected through Rustup, Bun 1.4.4, CMake 4.4.5 and
Apple Clang 21.0.0. Asterisk 22.11.0 ran in the repository's digest-pinned
Linux/arm64 Docker lab. This records local execution, not hosted CI or Windows results.

## Corrections and regression evidence

- Homebrew OpenSSL 3.5.9 archive members required macOS 27 despite Ruf's macOS 14
  target. The existing `OPENSSL_ROOT_DIR` mechanism now selects a checksum-verified
  source build of the same version targeting 14. The SDK probe rejects newer
  member minima before library compilation/staging, resolves changed roots afresh,
  and passes the same headers/libraries to downstream CMake projects. The macOS
  desktop/release workflow builds that compatible source rather than a host bottle.
- Two new regressions failed before their corrections, then passed: real Mach-O
  archive fixtures cover either library and mixed members; a CMake wrapper exercises
  the actual MSVC discovery branch with two roots and one cache. It initially mixed
  replacement headers with old libraries; clearing configuration-specific MSVC
  caches fixed that. This does not constitute Windows compilation/runtime testing.
- The optional saved-session test failed with `StorageFailure`: it mutated the
  expected in-memory state before the durable-state comparison. Its test setup now
  clones, persists and then adopts the journal, matching `prepare_save`. All five
  optional desktop tests subsequently passed, including real synthetic OS-keychain
  writes/readback/deletion and restored accounts. The failed attempt's two isolated
  synthetic credentials and temporary metadata were removed. Production behavior
  and the guard against stale writes were unchanged.
- The pinned `rust-objcopy` aborted because its runtime search path missed the
  toolchain's existing `libLLVM.dylib`. A process-local library path fixes loading.
  Cargo also replayed a host-helper warning cached on 6 November; a package-scoped
  release clean removed it. Both final package logs contain no LLVM stripping or
  newer-macOS linkage warning. The toolchain itself was not modified.

Changed implementation/configuration: [SDK builder](../../native/build.ts),
[OpenSSL probe](../../native/openssl-probe/CMakeLists.txt),
[desktop workflow](../../.github/workflows/desktop-build.yml),
[OpenSSL regressions](../../tests/ui/native-openssl.test.ts) and the test-only
[saved-session fixture](../../apps/desktop/src-tauri/src/phone.rs).
Build instructions are maintained in the [native guide](../../native/README.md)
and [development guide](../development.md); this record is linked from the
[verification index](README.md). Existing dependency selection, CMake probing and
journal patterns were reused. No dependency, API or DTO contract was added;
generated TypeScript contracts remain unchanged.

## Observed checks

| Check                                                             | Result                                                                                                                                                                    |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Frozen Bun install, version pins, formatting, workflow actionlint | Passed; no lockfile changes.                                                                                                                                              |
| Contract generation and generated diff gate                       | Passed; zero generated diff.                                                                                                                                              |
| Svelte/TypeScript and unused-symbol check                         | Passed; zero Svelte errors/warnings.                                                                                                                                      |
| UI unit tests / browser flows / frontend build                    | 111 tests, all nine synthetic-IPC flows and production build passed.                                                                                                      |
| Portable workspace Clippy / tests                                 | Passed; 126 tests, one optional keychain case ignored in this default suite.                                                                                              |
| Native SDK / native workspace Clippy / tests                      | Passed; 296 tests, 20 opt-in cases ignored in the default suite, exercised separately below.                                                                              |
| All-features workspace Clippy                                     | Passed with warnings denied, including native test hooks and App Store feature.                                                                                           |
| App Store desktop unit tests / unsigned preflight                 | 89 passed, five opt-in cases ignored; unsigned configuration preflight passed.                                                                                            |
| Asterisk smoke and optional adapter cases                         | Fresh PCM16 mono 8 kHz echo: 4.04 s, RMS 5811, 440.1 Hz; all 16 cases passed.                                                                                             |
| Asterisk security / desktop startup restoration                   | 13 security cases and the saved-account first-incoming regression passed.                                                                                                 |
| All optional desktop cases                                        | Five passed after the journal-fixture correction; normal desktop suite rerun: 89 passed.                                                                                  |
| SIP diagnostics / GuardMalloc lifetimes                           | 64 allowlisted markers with logging disabled by default; all three protected-allocation regressions passed.                                                               |
| Network / native call-action / DNS harnesses                      | Six network cases, call-action guards and 65,536 real native DNS refreshes passed.                                                                                        |
| Native security / Linux verifier branch on macOS                  | Six C policy harnesses and the generated-chain trust test passed; no Linux runtime qualification.                                                                         |
| Virtual-clock jitter-buffer probe                                 | All four scenarios played 100/100 packets with zero insertion errors; first playout 100 ms fixed vs 40 ms adaptive.                                                       |
| macOS service/notification tests                                  | Contacts/notification timeouts, login-item policy and real isolated notification delivery/withdrawal passed. Action responses were synthetic.                             |
| OpenSSL source / exact CI source-build shell step                 | 4,009 upstream tests passed; the workflow step was extracted verbatim and run locally, not on GitHub.                                                                     |
| Portable and native app/DMG packages                              | Both built, deep/strict ad hoc signatures and signed microphone/Contacts metadata passed.                                                                                 |
| Artifact checks                                                   | Embedded raw/bundle SBOM exports equal sidecars; dependency references, context and hashes checked. Native OpenSSL 3.5.9 included; production test-CA/DNS symbols absent. |
| Read-only DMG checks                                              | Images verified and mounted; contained executables match the archived signed bundles. Both mounts detached.                                                               |
| Lab cleanup                                                       | Container/network removed; the lab had not been running before these checks.                                                                                              |

Expected negative controls (incompatible SDK, pre-fix regressions and broken LLVM
helper) failed as intended. The actual optional-session failure was corrected and
rerun successfully. Focused independent review found no unresolved issues after
corrections. API/DTO generation, lockfiles and frozen inputs were checked again.

## Frozen inputs and artifact binding

Before the final portable/native packages, 373 existing tracked or unignored
regular files were frozen as SHA-256 entries. The verification index and this
self-referencing report are excluded. The digest of sorted UTF-8 entries
`<sha256>  <repository-relative path>\n` is:

`650272c984ca330b81de03ec5c861125c6b9ab622a57291875730101409a5580`

The complete manifest, 245 staged SDK file hashes, command results/logs,
SBOM/build manifests and both separate bundle/DMG variants remain locally under
`artifacts/verification/release-checks-20261110T000657Z/` (ignored by Git).
The frozen source and SDK files remained unchanged through final packaging.
SDK manifest and linked archives:

| SDK input          | SHA-256                                                            |
| ------------------ | ------------------------------------------------------------------ |
| `sdk.json`         | `8d7e3b9ccdc1554298a3e0dc09b4083744367c5f5be8253aad0e59b6967fe3af` |
| `lib/libre.a`      | `82d0df451ed351ffa155f9d49d7f6d74dfe9ba05656b697357d78e72300cb91f` |
| `lib/libbaresip.a` | `59b0d8a1509218d7af673b80c2e97a4915a55f35d69ce84f714ac582097d0ca9` |
| `lib/libssl.a`     | `c6f7dd59617311e385e188db1127e01da97ac8877798e87361bcfc0c5814997e` |
| `lib/libcrypto.a`  | `130299949f09ce52398336f05d00003d7850080b8b19723b7205d74b35134970` |

All 1,409 inspected members across the four arm64 archives declare macOS 14.0.
This is deployment metadata evidence, not execution on macOS 14.
Both variants use the installer name `Ruf_0.1.0_aarch64.dmg`; their separate
`portable/` and `native/` archive directories prevent replacement of evidence.
Only these final hashes qualify the packages in this record:

| Variant  | Artifact                 | SHA-256                                                            |
| -------- | ------------------------ | ------------------------------------------------------------------ |
| portable | Raw executable           | `0d66a6456f56ff9d63604ce944bda19e317d77896d328c7c8c9cce695746b3e1` |
| portable | Signed bundle executable | `cecbf7b1c7ee7588ea934122d1835a690fdcc7a64b11b22a7e556da31a540e45` |
| portable | SBOM                     | `b138cfbf4588f18b41c087652a3eaa118735efd44cab1ab8df2749d6b63fc2b0` |
| portable | DMG                      | `ef711a71d9af9c81b93cc4d6dd01afb6df7ea0ffe7c48028d932f1b2964b15ae` |
| native   | Raw executable           | `4aff56d122ccd06e7c36089474d318866b54c492cb37e57f784800d2d25d1d6f` |
| native   | Signed bundle executable | `7f9e87429eac87bc59dc3c62456b01008972043aa65dc6f1707cfb6566eddf02` |
| native   | SBOM                     | `01dcd28d17df883ee74d4889d9308596bc74be1e9ea07440b1841cade9cfb7cd` |
| native   | DMG                      | `6055df9e49708f5fce0cf70fd90c74a186a80724e1624758fcf5cf8c00036c18` |

## Remaining limits

Windows compilation/runtime, hosted GitHub Actions, macOS 14 runtime, installed-app
upgrade/permission acceptance, physical microphone/headset media, FreeSWITCH/provider
coverage and a multi-day soak were not executed. The builds are ad hoc signed;
notarization and production/App Store signing were not performed. No package was
installed, uploaded or released. The upstream libre test link still emits a harmless
duplicate-library warning; macOS 27 deprecates the `hdiutil` mount syntax used only
by local verification. No warning was suppressed.

Synthetic PCM does not establish physical audio quality or end-to-end latency.
Notification tests do not establish interactive installed-app clicks or Focus
behavior. Complete license inventory and the other platform/PBX acceptance gates
remain in the [release checklist](../release-checklist.md).
