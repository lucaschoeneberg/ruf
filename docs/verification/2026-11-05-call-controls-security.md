# Call controls and encrypted-account verification

Scope: the approved call-controls/security branch, synthetic loopback lab only,
macOS arm64, pinned Rust 1.98.1 and Asterisk 22.11.0. No installed Ruf app was
restarted or installed, no external contacts/accounts were called, and no OS
trust store, DNS, permissions or production credentials were changed.

## Native lab method

`bun run test:asterisk` retains the strict UDP Echo example and all six original
interop regressions. Eight additional ignored tests exercise the public Engine
API against real Asterisk dialogs. The same command explicitly compiles test
binaries with `native-test-hooks` and separately runs eleven controlled TLS
security/lifecycle tests; `--ignored` alone would omit those negatives. Native engines run
serially under the existing global lease.

The transfer original is an incoming call immediately after registration:
`Local/650@lab/n` calls Ruf/PJSIP1000 while its opposite Local side generates 440 Hz.
The target is a separate bridge through 602/`Local/652@lab/n`, generating 880 Hz.
Both original/consultation PJSIP legs are asserted to belong to actual bridges.
Successful blind/attended tests require the native final-NOTIFY-derived
`transferSucceeded`, empty Ruf dialogs, no remaining Ruf/PJSIP1000 channel, the
same remote peer identities alive, and their opposite Local legs sharing a bridge.

Only after that success do CLI MixMonitor stop/start commands create NEW
receive-direction WAV files on both surviving tone channels. Returned target 880 Hz
at the original and original 440 Hz at the target prove media continuity after
transferor teardown. This excludes stale pre-transfer audio, especially the
440 Hz tone Ruf itself transmitted before transfer. The terminal legacy projection
retains the last selected child's FIFO; attended transfer uses one REFER/final
NOTIFY, rather than requiring a fictional independent notification per dialog.

Mute similarly uses separate closed directional recordings before/during/after
native confirmation, with silence measured at the PBX while incoming 880 Hz stays
available. Live MixMonitor file lengths are buffered and cannot mark mute phases
reliably. Hold/resume and encrypted hold/resume record a new post-resume WAV byte
checkpoint and require more than three seconds of subsequently written PCM;
aufile can reopen/truncate its path on hold, so old total duration is insufficient.
All WAV validation checks PCM16/mono/8 kHz, useful-tone RMS >500 and frequency within
20 Hz of the fixture's 440/880 Hz. Mute requires >=1 s of PCM and RMS <100.

The runner generates fresh one-day CA/server material using the existing Python
certificate constructor in ignored `native/build`. Only synthetic server material
is mounted read-only into Asterisk. Test-only exclusive trust and private loopback
DNS reuse the existing constructors/controlled peer; no production CA override is
introduced. Account 1100 enforces verified TLS plus nonoptimistic mandatory SDES-SRTP.
Returned 440 Hz after resume proves actual encrypted media decrypts and returns.
1101 deliberately refuses required SRTP: Asterisk returns SIP 488, exposed as
`sipNotAcceptable`, with no successful protected call. The controlled peer's
missing/invalid/plaintext crypto answers separately require
`mediaSecurityRequired`, preserving the different native causes.

Additional scenarios cover ringing and established consultation cancellation,
switching with exactly one audible dialog, refusal/retry, stale generations,
remote ending on either side with fresh survivor media, slot reuse, and original
remote ending while consultation rings followed by the surviving child's answer.
Existing controlled native tests retain deterministic glare/refusal/recovery,
malformed SDP, transfer timeout, ACK, authority and lifecycle ownership coverage.

## SDK warning regression

Strict `cc -std=c11 -DUSE_OPENSSL -DUSE_TLS -Werror -fsyntax-only` compilation of
patched ACK and Contact sources initially rejected a prototype-local incomplete
`struct udp_sock` declaration from the private SIP header. The smallest shared
fix forward-declares that type in `re-security.patch`. Both strict compilations
pass after the final SDK rebuild. Exact patch text is regenerated into sdk.json,
verified by the adapter build, and participates in existing native SBOM hashes.
No SIP behavior, public API or DTO changed for this warning fix.

## Earlier task-4 execution record (before final-review fixes)

The earlier checks in this section completed on source freeze `3a1b94fc62bd46ffc89ce73bff1b304fb98b383b`.
The smoke run passed all 14 Asterisk cases in 64.36 s and all 8 controlled TLS cases
in 2.19 s. Those results preceded the final-review fixes recorded below. Their artifact hashes
are historical evidence, not the final branch artifacts.
Rust and desktop-wrapper commands used `PATH=/opt/homebrew/opt/rustup/bin:$PATH`
to select the pinned toolchain. Generated contract stability was checked with
`git diff --exit-code -- apps/desktop/src/lib/generated/AppInfo.ts`.

| Check                                                                                                                            | Result                                                                                            |
| -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `bun run native:build` and strict ACK/Contact `cc -Werror -fsyntax-only`                                                         | Passed; final SDK staged, private-header warning fixed                                            |
| `bun run test:asterisk`                                                                                                          | Passed: strict UDP Echo,14/14 Asterisk,8/8 controlled TLS                                         |
| `bun scripts/check-native-security.mjs` / `bun scripts/check-native-trust.mjs`                                                   | Passed: 6 C security units; Linux verifier branch override rejection                              |
| `cargo test --locked --workspace --features sip-desktop/native,baresip-adapter/native-test-hooks -- --test-threads=1`            | Passed: 206 tests, 0 failed, 18 ignored; adapter 100/0/14 in 457.92 s                             |
| Two exact safe ignored desktop tests with native, serial                                                                         | Passed: credential-authorization incoming race 1/1 (0.28s); missing-device validation 1/1 (0.10s) |
| `cargo test --locked --workspace`                                                                                                | Passed: 118 tests, 0 failed, 1 OS-keychain test ignored                                           |
| `cargo clippy --locked --workspace --all-targets -- -D warnings`                                                                 | Passed portable workspace                                                                         |
| `cargo clippy --locked --workspace --all-targets --features sip-desktop/native,baresip-adapter/native-test-hooks -- -D warnings` | Passed native workspace                                                                           |
| `bun run format:check`                                                                                                           | Passed formatter and Cargo formatting                                                             |
| `bun run check` / `bun run build`                                                                                                | Passed; Svelte 0 errors/0 warnings; production Web build                                          |
| `bun run types:generate` and generated-file Git diff                                                                             | Passed; generated contract unchanged                                                              |
| `bun run check:versions`                                                                                                         | Passed: JavaScript/Cargo/Tauri 0.1.0                                                              |
| `bun run desktop:build --features native --bundles app`                                                                          | Passed: aarch64 macOS release Ruf.app 21.09 MiB; SDK/SBOM validation and hash export              |
| Production features, symbols, signatures and hashes                                                                              | Passed: no native-test-hooks or test CA/DNS exports; local codesign verifies; not notarized       |

Observed PCM metrics (all new samples under the phase rules above):

| Scenario                                        | Samples / frequency / RMS                   |
| ----------------------------------------------- | ------------------------------------------- |
| Existing UDP Echo                               | 4.04s /440.1Hz /5811                        |
| Attended target→original after final NOTIFY     | 21,120 /879.9Hz /5106                       |
| Attended original→target after final NOTIFY     | 20,000 /440.0Hz /5099                       |
| Blind target→original after final NOTIFY        | 21,120 /873.1Hz /5087                       |
| Blind original→target after final NOTIFY        | 19,840 /439.9Hz /5100                       |
| UDP post-resume                                 | 32,832 /437.9Hz /5797                       |
| Incoming while muted                            | 16,384 /879.9Hz /5102                       |
| Muted outgoing                                  | >=1s PCM /silence /0.0                      |
| Unmuted outgoing                                | 17,600 /440.0Hz /5811                       |
| Ringing child after original remote end         | 16,672 /880.0Hz /5107                       |
| Surviving child/original after other remote end | 17,888 /880.1Hz /5103;16,832 /440.1Hz /5099 |
| Verified TLS+required SRTP post-resume          | 32,832 /440.1Hz /5811                       |

The native workspace's 18 ignored entries were handled explicitly: all 14 Asterisk
tests ran separately through the smoke runner. These 2 existing safe desktop tests
ran individually with `--features native -- --ignored --exact --test-threads=1`:

- `phone::credential_tests::incoming_during_authorization_preserves_account_and_cleanup_journal`
  uses blocked **in-memory** credentials, temporary metadata, TestTone and the
  synthetic Asterisk account; it does not touch the OS credential store.
- `phone::tests::first_sound_save_rejects_missing_device_without_starting_engine`
  reads native device metadata, without starting an engine or capturing audio.

The exact supplemental invocations were:

```sh
cargo test --locked -p sip-desktop --features native phone::credential_tests::incoming_during_authorization_preserves_account_and_cleanup_journal -- --ignored --exact --test-threads=1
cargo test --locked -p sip-desktop --features native phone::tests::first_sound_save_rejects_missing_device_without_starting_engine -- --ignored --exact --test-threads=1
```

Only `phone::tests::saved_accounts_survive_session_restart_without_password_readback`
and `store::tests::system_keychain_roundtrip_and_delete` remain unexecuted because
they write to the actual OS credential store. The latter is also the portable
workspace's one ignored test. No unexecuted test is counted as passing.

## Earlier task-4 production artifact binding

The earlier app was built from the task-4 code-freeze commit above with **native only**.
Its documentation-only follow-up did not change that artifact's code revision.
`cargo tree --locked -p sip-desktop --features native -e features` contains no
`native-test-hooks`; `nm -g` on both the raw release executable and bundled app
executable contains neither `sip_engine_set_test_ca` nor
`sip_engine_set_test_dns`. Local `codesign --verify --deep --strict` passes.
Notarization was skipped because no Apple publishing credentials were supplied.

| Artifact / binding                 | SHA-256                                                            |
| ---------------------------------- | ------------------------------------------------------------------ |
| Exported native CycloneDX SBOM     | `560134eea7864dcdb44879c06f869c79eaefbc49d317aea62d3b2525d0d47530` |
| Raw release executable             | `bc1f59d6e742fb830fd58fe8737b88b377152474a9692f26e219cdb464452a44` |
| Bundled, locally signed executable | `a277f079c939666cc3906eddb3944ce07338e912b8517a0b1495f3a9aefe4050` |
| Ordered libre patch set in SBOM    | `d74fe6c52244e2baeef414eba8185ee296da8a864ab67cc8031e824f2f757e67` |
| Ordered baresip patch set in SBOM  | `51b5175a47522a79f2e06a4bdff66b26b457008cd99fafbfe8d28e27e23f73c4` |
| Tracked/staged re-security.patch   | `8b04bdba9f47597e5c753ef1e02a42d5963a21d6b2e093c595d54aee9a81516f` |

The raw and bundled executable hashes differ because bundling signs its Mach-O
copy; both were checked independently. SBOM and raw executable match the exported
build manifest. Ordered patch hashes match exact staged SDK texts; the module
list includes SRTP. An isolated copy of the exact PCM/tone validator accepted the
recorded SRTP 440 Hz waveform and rejected an inverted RMS predicate; no tracked
source was mutated for this probe.

Unchanged UI evidence is reused explicitly from the controller-observed Task 3
fix-round freeze: 85 UI tests, 1,261 assertions, and all six browser flows passed;
Svelte/TypeScript check had 0 errors/warnings. Task 4 changes no UI, command, account
DTO, persistence or desktop authority source. These are earlier observed results,
not a claim of a newly repeated UI/browser execution in Task 4.

## Final-review regression fixes

The final-review fix source freeze is
`8cbcef64d034ed88fa680543c1b5b5bf97caf83f` (base
`8cfddc0e04cc872799ac711b9421e30d40a2ac0c`). This replaces the earlier production
artifact binding above; subsequent verification-document changes do not change
that source revision. Complete tracked source/test hashes, excluding this evidence
document, were frozen before the final suites.

Three new real TLS cases first failed on the reviewed SDK: a local hold answer
without acceptable crypto, a remote established re-INVITE offer without crypto,
and a remote offerless re-INVITE whose ACK answer lacks crypto. All left the
original dialog active with unknown media. With the fix, only the failed dialog
closes and its exact call/account retain `mediaSecurityRequired`. Packet logs bind
the failed re-INVITE's ACK, or the remote offer's 488, to its exact SIP Call-ID and
CSeq before that dialog's BYE. Termination runs after the callback finishes its
transaction work. The remote-offer case also retains a secure, usable consultation
while the original fails; every case preserves registration and establishes a new
secure call afterward. The existing C SRTP tests still prove rejected contexts
are cleared and mandatory RTP/RTCP gates stay closed.

Two new normal-refusal tests first received **zero fresh PCM bytes** after SIP 488
and 491 despite an established receive pipeline. Rollback now uses baresip's
existing media update/start path. The controlled peer sends a known 500 Hz PCMU
waveform paced against absolute 20 ms deadlines. The tests verify the newest full
second of baseline and post-refusal PCM, without another hold/resume; both cases
measured 8,000 samples, RMS 9,852 and 500.0 Hz in both phases. Mute remains enabled
while incoming audio resumes. A third test makes the actual aufile player reopen
fail by replacing its ignored recording path with a directory; it first failed
with an active call and now requires owned terminal `nativeFailure`. Ordinary SIP
refusal retains its classification, including Asterisk 1101's `sipNotAcceptable`.

Focused verification passed: eleven controlled TLS cases, twenty native control
cases, all four matching refused-hold/recovery cases, six C security units, the
Linux verifier branch harness on macOS, native workspace clippy, and strict
ACK/Contact C syntax checks with `-Werror`. No DTO, UI, desktop authorization,
dependency, or separate audio pipeline was introduced. The earlier 85 UI tests and
six browser flows remain explicitly reused evidence; final check/build results
are recorded separately below.

### Final frozen-source execution and artifact binding

All following checks completed on the fix source freeze above, with the pinned
Rustup PATH. Native engines ran serially. The two safe ignored desktop tests
listed in the earlier record were **not repeated** because no desktop source
changed; their earlier evidence is reused. The two actual OS credential-store
writing tests remain unexecuted. No ignored test is counted as passed.

| Check                                                                              | Observed final result                                                                                    |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Native workspace tests with `sip-desktop/native,baresip-adapter/native-test-hooks` | 212 passed, 0 failed, 18 ignored; adapter 106/0/14 in 471.37 s; command 481.18 s                         |
| `bun run test:asterisk`                                                            | Strict UDP Echo plus 14/14 Asterisk (64.08 s) and 11/11 controlled TLS (3.30 s); command 82.92 s         |
| Portable `cargo test --locked --workspace`                                         | 118 passed, 0 failed, 1 OS-keychain test ignored                                                         |
| Native and portable workspace clippy, all targets, `-D warnings`                   | Both passed                                                                                              |
| `bun run format:check`, `check`, `build`, `check:versions`                         | Passed; Svelte 0 errors/0 warnings, production Web build, version 0.1.0 consistent                       |
| `bun run types:generate` and generated-directory Git diff                          | Passed; generated contracts unchanged                                                                    |
| SDK rebuild, six C security units, Linux verifier branch harness                   | Passed on macOS; no Linux runtime claim                                                                  |
| Strict ACK/Contact C syntax with `-Werror`                                         | Passed                                                                                                   |
| `bun run desktop:build --features native --bundles app`                            | Passed; native aarch64 Ruf.app 21.09 MiB, 16.55 s                                                        |
| Production feature graph, both symbol tables, signature and source/SBOM bindings   | Passed; no native-test-hooks or CA/DNS setter exports; local codesign valid; all 391 frozen hashes match |

Final actual media samples include UDP Echo 4.04 s / 440.1 Hz / RMS 5811;
TLS+required-SRTP post-resume 32,672 samples / 440.0 Hz / RMS 5811;
attended target→original 21,440 / 880.2 Hz / RMS 5106 and original→target
20,160 / 436.5 Hz / RMS 5079; blind target→original 21,280 / 873.3 Hz / RMS 5087
and original→target 20,000 / 436.4 Hz / RMS 5079. Muted transmission measured
RMS 0.0 while incoming audio remained 879.9 Hz / RMS 5102. These use the fresh
phase-specific recording rules above.

The production build enables **native only**. Both raw and bundled executables
were independently checked for absent test CA/DNS setter exports. All six exact
staged SDK patch texts match tracked files; ordered libre/baresip patch digests
match the exported SBOM, which includes SRTP. The raw executable and exported
SBOM match the build manifest; the signed bundle has its own hash. Local
`codesign --verify --deep --strict` passed; notarization was not performed.

| Final artifact / binding                   | SHA-256                                                            |
| ------------------------------------------ | ------------------------------------------------------------------ |
| Exported native CycloneDX SBOM             | `e58981ed9725cd66939e70392c087b82e83775b2440aea115b1f351de1bc2b2a` |
| Raw release executable                     | `3610f791e2c5cab2c86a0a03947e9c1472bb16ee22442be67f675f145de6352b` |
| Bundled, locally signed executable         | `ba95518b6152ad14d00c9fa7c6e2ed958d99773f64a99f432348d2917865f52d` |
| Ordered libre patch set                    | `bf786f607308b6dcd5cb17de659dc2cb94484878b03991f53f498337cd3c95b3` |
| Ordered baresip patch set                  | `3ac048920a7a72c78e36033d0d3f1437d9a1633bc640e48e8ce5f939f544f3f6` |
| Tracked/staged re-security.patch           | `5f5de10d7b2f20673af7b225e65e87a60be5e8ca88eaa3c230742479f60ee66c` |
| Tracked/staged baresip-call-controls.patch | `9c11b90649b4aebbcad3ca3236d3572157d848516da8056e16f1c796965fad2d` |

## Limits

This establishes the exercised macOS loopback Asterisk and controlled-peer cases.
It does not establish hosted CI results, Windows runtime/trust behavior, Linux
runtime OS trust, older macOS runtime compatibility, hardware microphone/speaker
capture, or interoperability with every provider/PBX. The Linux verifier C branch
harness run on macOS is a branch-level trust regression, not a Linux runtime test.
TLS/SRTP protect the PBX hop and do not imply end-to-end encryption.

The local Homebrew OpenSSL 3.5.9 static objects report macOS 27 while the application
minimum is 14; duplicate-library and newer-object linker warnings predate this
work. Local compilation/bundling cannot prove older-macOS execution. The private
UDP declaration warning is fixed separately; no unrelated crypto/toolchain
redesign is included. Synthetic WAVs/keys/build output remain ignored by Git.
