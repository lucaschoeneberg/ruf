# Development

Use the Node version in `.node-version` and Rustup with `rust-toolchain.toml`.
The npm version is recorded in `package.json`; install with `npm ci` from the root.
Commit `package-lock.json` and the root `Cargo.lock`. Cargo defaults to the two
portable crates; use `--workspace` to include Tauri.

On macOS install Xcode/Command Line Tools. On Windows install MSVC C++ Build Tools,
the Windows SDK and WebView2 as described in the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).
Current release targets are macOS 14+ arm64 and Windows 11 x64. Windows hosted
builds use Windows Server 2022; that is a build environment, not a runtime test
on Windows 11. Intel macOS, ARM Windows and Linux are not release targets yet.

`npm run native:build` builds the pinned SDK; `npm run dev:native` starts a
real native phone. See [native prerequisites](../native/README.md) and the
[Asterisk lab](../tests/interop/asterisk/README.md). `npm run dev` starts Tauri and a loopback-only Vite server. `npm run dev:web` is
for layout work only. Native API access is unavailable in that browser mode.

Before a change is complete:

```sh
npm run types:generate
git diff --exit-code -- apps/desktop/src/lib/generated
npm run check:versions
npm run format:check
npm run check
npm run build
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm run desktop:build
```

After intentional DTO changes, review and commit the generated diff before using
the clean-diff command as a verification gate. `npm run format` formats source and
documentation. Native artifacts are under the root `target/release/bundle`, or
`target/<triple>/release/bundle` when passing `--target`.

For native changes, also run workspace Clippy/tests with
`--features sip-desktop/native`, `npm run test:asterisk`, and
`npm run desktop:build -- --features native`. Test-tone mode needs no audio
hardware; microphone/speaker and Windows runtime require separate platform tests.

Version changes update the root npm package, frontend package and Cargo workspace
version together. Tauri reads the root npm version directly. Run
`npm run check:versions`; release tags must be exactly `v<version>`.
