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
`apps/desktop/build/**/*.ts` and the icon generator. It enables `strict`,
`noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `isolatedModules` and
`verbatimModuleSyntax`. Bun, Svelte and Vite declarations describe the runtime
and imported sources; Playwright imports are type-only. Browser fixture types
reuse the generated DTOs while describing intentionally partial legacy mocks.
Executing scripts and tests does not replace this separate static check.

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
