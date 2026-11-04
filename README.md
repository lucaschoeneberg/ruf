# SIP

Schlankes SIP-Softphone für macOS und Windows mit Rust, Tauri 2 und Svelte.
Eigener Code steht unter der [MIT-Lizenz](LICENSE).

**Stand: Projektgrundlage.** Die Desktop-App zeigt ihren echten Laufzeitstatus.
baresip ist noch nicht eingebunden; Anrufe, Konten, Kontakte, Verlauf und Kalender
sind noch nicht implementiert. Installer aus dieser Phase sind Entwicklungspakete.

## Lokal starten

Voraussetzungen: Node aus `.node-version`, npm 11, Rustup sowie die
[Tauri-Systemvoraussetzungen](https://v2.tauri.app/start/prerequisites/)
(Xcode auf macOS, MSVC Build Tools und WebView2 auf Windows).

```sh
npm ci
npm run dev
```

Rustup installiert die in `rust-toolchain.toml` festgelegte Version. Wenn Cargo
über Homebrew oder einen anderen Paketmanager statt Rustup aufgerufen wird,
muss dessen Toolchain ausdrücklich auf dieselbe Version eingestellt werden.

```sh
npm run check
npm run build
npm run types:generate
npm run format:check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
npm run desktop:build
```

`npm run dev:web` öffnet nur die Web-Oberfläche; der native Status ist dort
ausdrücklich nicht verfügbar. Cargo nutzt ein gemeinsames `target/` im Root.
Ohne `--workspace` prüfen Cargo-Befehle standardmäßig nur die portablen Pakete.

## Aufbau

| Pfad                     | Verantwortung                                                  |
| ------------------------ | -------------------------------------------------------------- |
| `apps/desktop/src`       | Svelte-Oberfläche                                              |
| `apps/desktop/src-tauri` | Tauri-App, IPC und spätere Plattformanbindungen                |
| `crates/phone-core`      | Plattformunabhängige Rust-Verträge und spätere Anwendungslogik |
| `crates/baresip-adapter` | Grenze zur späteren nativen Telefonie-Engine                   |
| `native`                 | Vorbereitung reproduzierbarer nativer Builds                   |
| `tests/interop`          | Vorbereitung echter Anlagen- und Audiotests                    |
| `.github`                | CI, plattformübergreifende Installer und Release-Entwürfe      |

## Weiterarbeiten

- [Architektur](docs/architecture.md) und [Entwicklungsablauf](docs/development.md)
- [Interne API und generierte Typen](docs/api/README.md)
- [GitHub, CI/CD und Signierung](docs/ci-cd.md)
- [MVP-Entwurf](docs/superpowers/specs/2026-11-04-sip-mvp-design.md)
- [Beitrag leisten](CONTRIBUTING.md)

GitHub-Workflows sind vorbereitet. Ein Remote und Repository-Regeln werden erst
nach Verbindung mit dem tatsächlichen Repository eingerichtet. Es wurde durch
das Anlegen der Dateien nichts zu GitHub übertragen oder veröffentlicht.
