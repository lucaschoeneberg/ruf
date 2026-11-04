# Architecture

The Cargo workspace has three packages. `sip-desktop` composes `phone-core` and
`baresip-adapter`; the adapter uses core's typed status. Core has no Tauri, native
or network dependency. npm workspaces contain the plain Svelte/Vite frontend.

Tauri's native commands are the application boundary. The only command today is
`get_app_info`. The main window has explicit permission for that command; there is
no shell/filesystem/network plugin permission and no separate HTTP API server.
The frontend uses generated TypeScript DTOs. Browser preview shows an explicit
unavailable-native-runtime state rather than inventing native data.

baresip is not linked. The planned library integration uses a dedicated native
event-loop thread and a narrow C shim, following upstream's message-queue pattern.
Audio remains with baresip. See [native preparation](../native/README.md).

Future account/call state belongs in Rust; frontend state is its projection.
SQLite, keyring, Tokio and platform bindings will be added when a feature needs
them. They are intentionally not installed as unused foundation dependencies.

The temporary identifier is `org.sipsoftphone.desktop`; select the permanent
identifier before distributing a user-facing build, since it affects app identity,
permissions and future upgrades. The icons are official starter placeholders.
