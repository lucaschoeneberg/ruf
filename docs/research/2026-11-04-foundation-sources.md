# Foundation references

Reviewed on 4 November 2026. Implementation decisions:

- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html):
  shared root lockfile/target directory and inherited metadata/dependencies.
- [Tauri project structure](https://v2.tauri.app/start/project-structure/):
  keep the native shell beside its frontend, within a root Cargo workspace.
- [Tauri frontend setup](https://v2.tauri.app/start/create-project/):
  inspect the official Svelte/TypeScript scaffold; retain standard config/icon
  conventions. Plain Vite is enough for the initial shell; no server rendering.
- [Tauri commands](https://v2.tauri.app/develop/calling-rust/) and
  [capabilities](https://v2.tauri.app/security/capabilities/):
  explicit main-window permission for the registered read-only application command.
- [Tauri GitHub pipelines](https://v2.tauri.app/distribute/pipelines/github/):
  native runner matrix, target-specific bundles, development signing limitations.
- [GitHub hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners):
  macos-15 is arm64; Windows x64 builds use windows-2022.
- [GitHub secure workflow use](https://docs.github.com/en/actions/reference/security/secure-use):
  immutable action references and narrow permissions; separate draft creation.
- [baresip](https://github.com/baresip/baresip#building) and
  [library integration](https://github.com/baresip/baresip/wiki/Using-baresip-as-a-Library):
  CMake, libre/OpenSSL, explicit modules and a native event-loop message queue.
- [ts-rs](https://github.com/Aleph-Alpha/ts-rs): generate TypeScript from Rust DTOs.

Native baresip/libre source pins and builds belong to the next milestone. Current
package versions were resolved from their registries; lockfiles record resolution.
Action references were verified with upstream Git tag lookups.
