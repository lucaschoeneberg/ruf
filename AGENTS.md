# Project conventions

Read the relevant code and docs before editing. Reuse existing APIs, DTOs,
utilities and dependencies. Make a short plan, a minimal patch, then run checks.

- Rust: edition 2024; version is pinned in rust-toolchain.toml. Use workspace
  metadata/dependencies. Keep phone-core independent of Tauri and native handles.
- TypeScript: strict typing. Frontend IPC contracts come from Rust through ts-rs.
- Native: all C/unsafe integration belongs in baresip-adapter. Do not
  implement SIP/audio in the frontend or add a second audio pipeline.
- Portable builds report `notLinked`; native builds report `linked` only for
  compiled linkage. Registration and media success require real native evidence.
  Build the SDK before `--features sip-desktop/native` workspace checks; run
  `bun run test:asterisk` for changes affecting SIP or audio.
- Document changed commands/DTOs, required/optional fields, validation and errors
  in docs/api. Use Rust documentation comments for public Rust APIs.
- Prefer existing patterns; avoid speculative abstractions and unused dependencies.
- No credentials, build output or personal call/contact data in Git.

Checks: `bun run format:check`, `bun run check`, `bun run build`,
`bun run check:versions`, `bun run types:generate`,
`cargo clippy --locked --workspace --all-targets -- -D warnings`,
`cargo test --locked --workspace`, and `bun run desktop:build` for desktop changes.
Verify generated files are unchanged after generation. On Linux, portable Rust
checks may use `-p phone-core -p baresip-adapter`; the desktop requires system deps.

At completion report changed files, behavior, reused patterns, API documentation,
checks actually run and remaining risks. Never imply hosted CI, Windows runtime
or SIP/audio tests passed unless those executions were observed.
