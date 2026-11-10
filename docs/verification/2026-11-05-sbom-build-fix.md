# Build-specific SBOM: implementation and verification

As of November 5, 2026. Three technical gaps in the previous project inventory
were closed for the build verified here: native OpenSSL coverage, dependency
relationships, and build association. This is not a blanket declaration of
conformity with the entire CRA.

## Actual result

The verified macOS-arm64 native build contains a CycloneDX 1.6 SBOM with
**281 components**, including **221 with runtime scope `required`** and **60 with
build scope `excluded`**. OpenSSL **3.5.9** comes from the SDK actually selected.
The verified portable build contains **278 components** and no entries for
baresip, libre, or OpenSSL. These counts apply to the inputs verified here and
are not hardcoded in the product.

- Cargo metadata is filtered by target platform and native feature, then
  traversed from the desktop package. Build/proc-macro dependencies are marked
  accordingly rather than reported as runtime libraries.
- JavaScript packages and their import relationships come from the rendered
  Vite module graph and the package manifests actually installed. Frontend-only
  build tools and packages for other platforms are not indiscriminately copied
  from the entire lockfile.
- The native SDK target, source pins, and applied patch texts must match.
  The OpenSSL version, native archive hashes, and patch hashes are recorded.
  The selected baresip modules are associated with the baresip entry.
- Every entry receives a reference; edges must point to existing entries,
  and every entry must be reachable from the product.
- The desktop wrapper passes a single target/profile/native context to
  Vite and Tauri. The native CLI runs in a child process so that Bun/N-API
  sees the same OS environment from startup. The Rust build script rejects
  mismatches with the actual Cargo build and embeds the validated JSON file
  as an immutable snapshot.
- IPC and CLI export read exactly this snapshot. Export and metadata output
  do not start Tauri, accounts, contacts, or telephony.
- Alongside the installer are a target/variant-specific `.cdx.json` and
  a `.build.json` associating SHA-256 hashes with the SBOM, executable, and
  installer. The local workflow changes retain these files for release drafts too.

The About Ruf page retains its compact layout. Software details remain
collapsed; the overview shows only relevant core components.
The desktop app retrieves the complete file through the settings-only
`get_software_bill` command. Development previews are marked accordingly;
a release app does not replace a read error with a project inventory.

## Evidence of artifact association

The snapshot exported from the program through the CLI, the accompanying file,
and the installer were verified. The exported file is byte-identical to the
sidecar SBOM; its hash, the executable hash, and the installer hash match the
manifest. The file passes the official CycloneDX 1.6 schema validation.

For the native build verified in the final checks:

| Artifact                                         | SHA-256                                                            |
| ------------------------------------------------ | ------------------------------------------------------------------ |
| `Ruf_0.1.0_aarch64-apple-darwin_native.cdx.json` | `ab68f3eb179580c9a70ae877c8cd706c87e885fac5ec1821ae62b06fc432cfc4` |
| `Ruf_0.1.0_aarch64.dmg`                          | `f3217da96801945e240a327a7c81c8627dbc7e4ff530e4910f37b075e9839a97` |

The files are located under
`target/aarch64-apple-darwin/release/bundle/dmg/` and are build outputs,
not checked-in project files. The timestamp and SBOM serial number are generated
for each build; regenerating them later therefore produces different hashes.

## Checks run

- `bun run check`: no errors/warnings; `bun run build` succeeded.
- `bun run test:ui`: **54 tests passed**. Native SDK contracts are also tested
  with synthetic inputs without an installed SDK; the test against the real
  macOS SDK was also run here. The Windows Cargo graph was checked.
- `bun run test:ui:flows`: all five groups passed, including the compact
  software overview, local SBOM reading, and existing company links.
- Screenshots at 900 × 660 and 740 × 540 were checked; no horizontal overflow.
- `bun run check:versions` with the pinned Rust/Cargo **1.98.1** and Bun 1.4.4.
  Repeated DTO generation remained byte-identical.
- Workspace Clippy with `-D warnings` and workspace Rust tests, both without
  and with `sip-desktop/native`: passed; existing ignored tests remain
  identified as such. The native suite included real local SIP timeouts.
- `bun run native:build` ran successfully before the native checks.
- `bun run desktop:build --features native --bundles dmg` succeeded; the portable
  app build and another portable release build with `--no-bundle` succeeded.
- Negative control: OpenSSL was removed only from a temporarily modified build
  file. `cargo check --release --features native` rejected the build with
  `Native SBOM must contain the actual SDK OpenSSL version`.
  The original file was restored immediately.
- Native and portable CLI exports were compared with sidecars and validated
  against the official JSON schema. Missing/additional export arguments result
  in exit code 2 without starting the app.
- GitHub workflow syntax was checked with actionlint 1.7.12; formatting and diff
  were checked.

## Changed files and reuse

- `apps/desktop/build/software-bill.ts`, `software-bill-frontend.ts`,
  `sbom-runtime.ts`: existing Vite plugin approach, Cargo metadata, Bun, and SDK.
- `apps/desktop/src/lib/software-bill.ts`, `components/AboutPanel.svelte`,
  `api.ts`: standard format, existing settings interface, and IPC wrapper.
- `apps/desktop/src-tauri/build.rs`, `sbom_build.rs`, `Cargo.toml`,
  `src/about.rs`, `src/lib.rs`, `src/main.rs`, `capabilities/settings.json`,
  `permissions/autogenerated/get_software_bill.toml`: build validation, snapshot,
  and export; serde_json is also reused as a build dependency.
  No additional third-party package was introduced; Cargo.lock was unchanged.
- `scripts/build-desktop.ts`, `desktop-context.ts`, `tauri-run.ts`,
  `package.json`: existing desktop build and development entry points.
- `.github/workflows/desktop-build.yml`, `release.yml`: retain sidecars.
- `tests/ui/software-bill.test.ts`, `software-bill-frontend.test.ts`,
  `desktop-context.test.ts`, `browser/settings.ts`: behavioral regressions.
- `docs/api/about.md`, `docs/api/README.md`, `docs/licensing.md`,
  `docs/ci-cd.md`, and this verification report: IPC/CLI contracts and build process.

The API documentation describes required/optional fields, the meaning of build
properties, access restrictions, the development fallback, errors, and CLI exit
codes. Existing AppInfo/ts-rs DTOs and the SIP/audio pipeline remain unchanged.
Pre-existing unrelated working-tree changes were preserved.

## Limitations

Windows runtime, Windows installers, and hosted GitHub artifact/release runs
were not executed. A new Asterisk lab run was outside the scope of this
build/inventory change. The macOS package is locally signed, not notarized.
The local Rust installation reported a `rust-objcopy`/`libLLVM.dylib` warning
when stripping individual release artifacts; the builds and exports succeeded.
The initially incorrect Homebrew Rust version was avoided in the final checks
by explicitly selecting the pinned toolchain that was already installed.

The inventory covers the selected package/SDK components and fonts;
a complete license/source-file inventory of all embedded upstream
subcomponents and other CRA obligations remain separate verification tasks.
