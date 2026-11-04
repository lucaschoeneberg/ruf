# Foundation verification

Host: macOS arm64. Rust checks/builds used Rustup's pinned 1.98.1 toolchain;
Node 24.16.0 and npm 11.13.0. This record describes local evidence only.

- `npm ci`: lockfile installation succeeds.
- `npm run check`: Svelte/TypeScript report no errors or warnings.
- `npm run build`: Vite production build succeeds.
- `npm run format:check`: Prettier and Rust formatting succeed.
- `npm run check:versions`: npm, all Cargo members and Tauri agree on 0.1.0.
- Release tag check: `v0.1.0` is accepted, `v9.9.9` is rejected.
- `npm run types:generate`: TypeScript generated from Rust successfully.
- Workspace and portable-only Clippy runs succeed with `-D warnings`.
- `cargo test --locked --workspace` succeeds; the scaffold currently contains
  **zero behavior tests**. This is compiler/doc-test verification, not SIP coverage.
- `go run github.com/rhysd/actionlint/cmd/actionlint@v1.7.12 -color`: succeeds.
- macOS application bundle and aarch64 DMG builds succeed using locked Cargo
  dependencies. The app declares version 0.1.0 and macOS 14.0 minimum.
- Ad-hoc code signature verifies. Production signing/notarization is not configured.
- The final DMG was mounted read-only; bundled LICENSE/THIRD_PARTY_NOTICES.md and
  the packaged application's ad-hoc signature were verified. The image was detached.
- `npm audit --omit=dev`: zero findings. The full audit reports two low-severity
  development-dependency findings via deepmerge in vite-plugin-svelte, without
  an available upstream fix at verification time. No dependency override is added.

Windows/hosted workflows, actual desktop rendering and native IPC interaction,
release draft upload, signing/notarization, microphone/audio and SIP calls have
not been executed by this local verification. The next native milestone must
establish real engine behavior before any phone functionality is advertised.

An independent read-only review of the whole foundation approved it without
Critical, Important or Minor findings. The reviewer separately reran formatting,
frontend typecheck, version consistency and mismatched-tag rejection and inspected
the CLI API, generated command ACL and workflow prerequisites. No native rendering,
Windows execution or hosted release upload was inferred from those results.
