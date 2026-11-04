# SIP

Schlankes SIP-Softphone für macOS und Windows mit Rust, Tauri 2 und Svelte.
Eigener Code steht unter der [MIT-Lizenz](LICENSE).

**Stand: erste native Telefonie-Anbindung.** Ein flüchtiges Konto, SIP/UDP,
G.711, Anrufsteuerung und echte Statusanzeigen sind eingebunden. Der lokale
Asterisk-Echotest prüft Anmeldung, Gesprächsaufbau und zurückkehrendes Audio.
Kontakte, Verlauf, Kalender und mehrere Konten folgen später. Entwicklungspakete
sind noch keine produktionsreifen Telefone.

Die dunkle Desktop-Oberfläche startet kompakt mit 360 × 560 Pixeln. Sie bietet
eine Wähltastatur sowie Navigation für Kontakte und Verlauf. Das Konto wird über
das Zahnrad eingerichtet; Kontakte und Verlauf zeigen vorerst ehrliche
Leerzustände ohne angebundene Daten. [Designregeln](apps/desktop/DESIGN.md) und
[UI-Prüfung](docs/verification/2026-11-04-compact-ui.md) beschreiben den Stand.

## Lokal starten

Voraussetzungen: Bun aus `.bun-version`, Rustup sowie die
[Tauri-Systemvoraussetzungen](https://v2.tauri.app/start/prerequisites/)
(Xcode auf macOS, MSVC Build Tools und WebView2 auf Windows).

```sh
bun install --frozen-lockfile
bun run native:build
bun run lab:up
bun run test:asterisk
bun run dev:native
```

Native Voraussetzungen und Windows-vcpkg-Konfiguration stehen in
[native/README.md](native/README.md). Das öffentliche Testkonto und die
Nebenstelle stehen im [Asterisk-Labor](tests/interop/asterisk/README.md).
`bun run dev` startet weiterhin ohne native Telefonie, `bun run lab:down` beendet
nur die eigene Testanlage. SIP/UDP ist vorerst auf isolierte Testnetze beschränkt.

Rustup installiert die in `rust-toolchain.toml` festgelegte Version. Wenn Cargo
über Homebrew oder einen anderen Paketmanager statt Rustup aufgerufen wird,
muss dessen Toolchain ausdrücklich auf dieselbe Version eingestellt werden.

```sh
bun run check
bun run build
bun run types:generate
bun run format:check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
bun run desktop:build
```

`bun run dev:web` öffnet nur die Web-Oberfläche; der native Status ist dort
ausdrücklich nicht verfügbar. Cargo nutzt ein gemeinsames `target/` im Root.
Ohne `--workspace` prüfen Cargo-Befehle standardmäßig nur die portablen Pakete.

## Aufbau

| Pfad                     | Verantwortung                                                  |
| ------------------------ | -------------------------------------------------------------- |
| `apps/desktop/src`       | Svelte-Oberfläche                                              |
| `apps/desktop/src-tauri` | Tauri-App, IPC und spätere Plattformanbindungen                |
| `crates/phone-core`      | Plattformunabhängige Rust-Verträge und spätere Anwendungslogik |
| `crates/baresip-adapter` | Sichere Anbindung der nativen Telefonie-Engine                 |
| `native`                 | Gepinnte native Builds und Lizenztexte                         |
| `tests/interop`          | Lokale Asterisk-Anlage und echter Audiotest                    |
| `.github`                | CI, plattformübergreifende Installer und Release-Entwürfe      |

## Weiterarbeiten

- [Architektur](docs/architecture.md) und [Entwicklungsablauf](docs/development.md)
- [Interne API und generierte Typen](docs/api/README.md)
- [GitHub, CI/CD und Signierung](docs/ci-cd.md)
- [MVP-Entwurf](docs/superpowers/specs/2026-11-04-sip-mvp-design.md)
- [Beitrag leisten](CONTRIBUTING.md)
- [Verhaltensregeln](CODE_OF_CONDUCT.md), [Sicherheitsmeldungen](SECURITY.md) und
  [Lizenzierung und Attribution](docs/licensing.md)

GitHub-Workflows sind vorbereitet. Ein Remote und Repository-Regeln werden erst
nach Verbindung mit dem tatsächlichen Repository eingerichtet. Es wurde durch
das Anlegen der Dateien nichts zu GitHub übertragen oder veröffentlicht.
