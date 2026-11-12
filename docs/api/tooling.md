# Workspace tooling commands

The workspace uses the pinned Bun runtime to execute TypeScript sources directly.
The former `.mjs` entry points now use `.ts`; callers should prefer the stable
`bun run <script>` commands in the root `package.json`. Existing command arguments,
environment variables, native linkage requirements and output formats remain
unchanged. No desktop IPC commands or generated Rust DTOs change.

## Cleanup

`bun run clean` requires no arguments. It recursively removes only these
repository-local generated directories: `target`, `apps/desktop/dist`,
`apps/desktop/src-tauri/gen`, `artifacts`, `.playwright-cli` and
`output/playwright`. This includes local installers, screenshots and test reports.
Run it after stopping development servers, builds and tests.

Optional flags (combinable):

- `--dry-run`: print the selected paths without deleting anything.
- `--all`: also remove `native/build`, `native/install`, root `node_modules`
  and `apps/desktop/node_modules`. Afterwards run `bun install --frozen-lockfile`
  and, before native builds/checks, `bun run native:build`.

Paths are resolved from the script location, independently of the working
directory. Missing directories are accepted, so repeated cleanup succeeds.
Source files, lockfiles, `.env` files outside these output directories and
installed application data are preserved. Custom/external Cargo target directories
and global tool caches are outside this command's scope. No arbitrary paths are
accepted. Unknown arguments fail before deletion; filesystem errors stop cleanup
with a nonzero exit code and may leave earlier paths already removed. Success
exits zero and prints each selected path; there is no structured response or DTO.
Symlinked parents of selected paths are rejected before any deletion, including
in dry-run mode, so nested outputs cannot redirect cleanup outside the repository.
A symlink at a selected final path is removed as a link without following it.

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
retain its logs. Bun process spawning forwards both output streams to each
partition's log and waits for every child before reporting results. These tooling
commands expose process exit codes, not HTTP status codes.
