# Development

Use the Bun version in `.bun-version` and Rustup with `rust-toolchain.toml`.
The exact Bun version is also pinned in `package.json`; install with `bun install --frozen-lockfile` from the root.
Commit `bun.lock` and the root `Cargo.lock`. Cargo defaults to the two
portable crates; use `--workspace` to include Tauri.

On macOS install Xcode/Command Line Tools. On Windows install MSVC C++ Build Tools,
the Windows SDK and WebView2 as described in the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
Current release targets are macOS 14+ arm64 and Windows 11 x64. Windows hosted
builds use Windows Server 2022; that is a build environment, not a runtime test
on Windows 11. Intel macOS, ARM Windows and Linux are not release targets yet.

`bun run native:build` builds the pinned SDK; `bun run dev:native` starts a
real native phone. See [native prerequisites](../native/README.md) and the
[Asterisk lab](../tests/interop/asterisk/README.md). `bun run dev` starts Tauri and a loopback-only Vite server. `bun run dev:web` is
for layout work only. Native API access is unavailable in that browser mode.

Before a change is complete:

```sh
bun run types:generate
git diff --exit-code -- apps/desktop/src/lib/generated
bun run check:versions
bun run format:check
bun run test:ui
bun run check
bun run build
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
bun run desktop:build
```

After intentional DTO changes, review and commit the generated diff before using
the clean-diff command as a verification gate. `bun run format` uses the built-in
Bun formatter and `cargo fmt` for source and
documentation. Native artifacts are under the root `target/release/bundle`, or
`target/<triple>/release/bundle` when passing `--target`.

For native changes, also run workspace Clippy/tests with
`--features sip-desktop/native`, `bun run test:asterisk`, and
`bun run desktop:build --features native`. Test-tone mode needs no audio
hardware; microphone/speaker and Windows runtime require separate platform tests.

Version changes update the root JavaScript package, frontend package and Cargo workspace
version together. Tauri reads the root JavaScript package version directly. Run
`bun run check:versions`; release tags must be exactly `v<version>`.

## Bun tooling

Bun owns installation, workspaces, JavaScript execution and formatting. Run
`bun install --frozen-lockfile` from the repository root after checkout. Commit
`bun.lock`; do not generate a second package-manager lockfile. Both `.bun-version`
and `package.json` pin Bun 1.4.4; `bun run check:versions` verifies the pins and
actual runtime as well as application/release versions.

Workspace tools explicitly run with `bun --bun` so their Node shebangs do not
select a separately installed Node runtime. Existing `node:` standard-library
imports in build scripts are supported by Bun and remain unchanged. GitHub-hosted
actions may use their own bundled runtime; project commands use Bun.

`bun run format` runs the built-in `bun fmt` for Svelte, TypeScript, JavaScript,
CSS, JSON, YAML, TOML and Markdown, followed by `cargo fmt` for Rust. There is no
Prettier or Svelte formatter plugin dependency. Configuration and generated-file
exclusions live in `bunfig.toml`; `.editorconfig` remains authoritative for basic
whitespace. Native C/CMake formatting is unchanged.

Pass arguments directly: `bun run desktop:build --features native --bundles dmg`
on macOS, or `--bundles nsis` on Windows. Do not insert npm's argument separator.

References: [Bun formatter](https://bun.com/docs/format),
[lockfile migration](https://bun.sh/docs/pm/lockfile),
[GitHub Actions setup](https://github.com/oven-sh/setup-bun), and
[Dependabot Bun support](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference#package-ecosystem).

## Compact desktop UI

The native window defaults to 360 × 560 pixels (minimum 320 × 500). UI components
live in `apps/desktop/src/lib/components`, with shared styles in `src/app.css`.
The desktop [product record](../apps/desktop/PRODUCT.md) and
[design system](../apps/desktop/DESIGN.md) describe the confirmed dark direction.

`bun run test:ui` covers keypad insertion/deletion, selection replacement and the
64-character native destination limit using Bun's existing test runner. CI runs
it before the Svelte/TypeScript check. No browser/test dependency is added to the
application. Browser preview allows keypad/navigation/layout work but never
claims native readiness or enables registration/calling. Real calls still use the
existing Tauri API and the independent Asterisk echo regression.
