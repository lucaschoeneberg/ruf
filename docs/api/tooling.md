# Workspace tooling commands

The workspace uses the pinned Bun runtime to execute TypeScript sources directly.
The former `.mjs` entry points now use `.ts`; callers should prefer the stable
`bun run <script>` commands in the root `package.json`. Existing command arguments,
environment variables, native linkage requirements and output formats remain
unchanged. No desktop IPC commands or generated Rust DTOs change.

## Type checking

| Command                 | Input                              | Output and failure behavior                                                                                                                                                                                  |
| ----------------------- | ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bun run check`         | No required or optional arguments. | Runs the tooling check followed by the existing Svelte/desktop TypeScript check. Exits nonzero on either failure; the desktop check runs only if tooling passes.                                             |
| `bun run check:tooling` | No required or optional arguments. | Runs the existing TypeScript compiler with `tsconfig.tooling.json` and `noEmit`. Reports file/line diagnostics; exits zero when valid and nonzero on type or configuration errors. Writes no compiled files. |

The tooling check includes `scripts/**/*.ts`, `native/*.ts`, `tests/ui/**/*.ts`,
`apps/desktop/build/**/*.ts`, the icon generator and the DMG artwork generator. It enables `strict`,
`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `isolatedModules` and
`verbatimModuleSyntax`. Bun, Svelte and Vite declarations describe the runtime
and imported sources. Browser fixture Playwright imports are type-only; the DMG
artwork generator uses the existing Playwright runtime. Browser fixture types
reuse the generated DTOs while describing intentionally partial legacy mocks.
Executing scripts and tests does not replace this separate static check.

`bun run check` also runs `bun run i18n:check` first to validate the shared German
and English catalogs. See [localization contracts](localization.md).

`bun run test:ui:flows` runs the nine existing German regression flows, nine
English scenarios using the same synthetic native fixtures, and a multiwindow
language-switch flow. Optional environment variables: `SIP_UI_LANGUAGE=de|en`
selects a language; `SIP_UI_FLOW` retains its existing individual-flow filter,
with `localization` selecting only the language-switch/changelog-race flow.
An unsupported `SIP_UI_LANGUAGE` fails the command. These are test-runner
controls, not application preference or language overrides.

## Native network and media regressions

`bun run test:network-roam` requires the pinned Rust toolchain, a C compiler,
CMake and the matching staged native SDK. It uses no new arguments or environment
variables. In addition to the existing network state-machine, call-action and
macOS DNS-lifetime checks, it builds and runs the native jitter-buffer probe.
That probe emits six JSON lines comparing fixed 100–200 ms and adaptive 40–200 ms
profiles at 0, 12 and 40 ms synthetic arrival variation. Each line includes
`profile`, `minimum_ms`, `input_jitter_ms`, `first_playout_ms`, `packets_sent`,
`packets_played` and `put_errors`. Every scenario must play 100 packets without
insertion errors. Missing SDK/tools, compilation or assertion/probe failures
exit nonzero; temporary harness build directories are removed on exit. No
microphone, speaker, real call or end-to-end latency measurement is involved.

## DMG artwork

`bun apps/desktop/src-tauri/dmg/generate.ts` requires macOS and an existing
Chromium browser. No arguments are required; optional
`SIP_UI_BROWSER_EXECUTABLE` selects the browser executable. It reads the authored
SVG, canonical Ruf mark and bundled IBM Plex Sans faces, then replaces only
`apps/desktop/src-tauri/dmg/background.png` with a 1440×960 render at 144 DPI.
Missing inputs/browser, render failures and unsupported platforms exit nonzero.
It performs no app installation, signing or notarization. Normal desktop builds
use the checked-in PNG without running the generator. See the
[artwork guide](../../apps/desktop/src-tauri/dmg/README.md) for Finder/CI requirements.

## Repository media

`bun run media:repository` accepts no arguments and requires the existing
Playwright/Chromium installation plus `ffmpeg` on PATH. Optional
`SIP_UI_BROWSER_EXECUTABLE` selects a browser executable. It captures the actual
English UI through the existing synthetic App Store fixtures, taking the version
from the root package manifest, then overwrites
only the documented media in `docs/screenshots`. PNG frames and a capture report
are written to ignored `output/playwright/repository`; a 30-second MP4 goes to
ignored `artifacts/release-preparation/media`. Off-origin browser traffic is
blocked. No SIP, audio, OS account/contact store, installer or publication is used.
Missing tools, unsupported arguments, server/capture errors or failed media
conversion exit nonzero. See [media provenance](../screenshots/README.md).

## Migrated entry points

- Build/development and packaging wrappers: `scripts/build-desktop.ts`,
  `scripts/tauri-run.ts`, `scripts/prepare-app-store.ts`,
  `scripts/capture-app-store.ts` and `apps/desktop/src-tauri/icons/generate.ts`.
- Native SDK build/cache helpers: `native/build.ts`, `native/cache-key.ts`,
  `native/vcpkg.ts` and `scripts/check-native-sdk.ts`. CI recipe hashes include
  the new TypeScript paths.
- Regression/version commands: the existing `scripts/check-*.ts` entry points
  and `scripts/asterisk-smoke.ts`; package script names are unchanged.
- Browser scenarios/installers in `tests/ui/browser/*.ts` are imported by the
  regression and screenshot wrappers. They are not standalone CLI commands.

Native workspace tests still return failure when any Cargo partition fails and
retain its logs. `bun run test:native` accepts only the optional `--adapter` flag
for headless adapter checks; other arguments fail before building. It builds the
selected suite once, discovers native libtest names and requires both the NAT and
SIP modules. Discovery/build errors exit nonzero rather than silently running an
empty native suite.

At most six test processes run together. Each NAT test and SIP test whose name
identifies a timeout/deadline, missing ACK, refused compensation, bounded failure
or long local hold runs separately with an exact filter. Ordinary SIP tests and
remaining workspace/integration/doc tests stay covered without repeating those
isolated tests. Existing ignored-test behavior and timer durations are unchanged.

Both stdout/stderr streams appear immediately with `[native:<partition>]` prefixes,
including partial lines; matching raw bytes are kept in each partition's `.log`.
Start, pass/fail, elapsed-time and exit-status messages are streamed as work ends.
Every selected child is awaited even if another fails. Any failure exits nonzero
and prints `Native checks failed; logs: <directory>`; success removes those
temporary logs and retains `NATIVE_CHECKS_OK`. These tooling commands expose
process exit codes, not HTTP status codes. See [CI cache behavior](../ci-cd.md).
