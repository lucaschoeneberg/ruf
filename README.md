# Ruf

Schlankes SIP-Softphone für macOS und Windows mit Rust, Tauri 2 und Svelte.
Eigener Code steht unter der [MIT-Lizenz](LICENSE).

**Stand: 0.1.0-Vorschau für isolierte Testnetze.** Bis zu acht gespeicherte
Konten können unabhängig registriert sein. Gespräche bieten Stummschaltung,
lokales Halten, direkte Vermittlung und eine Rückfrage mit Wechsel, Rückkehr und
Vermittlungsabschluss; höchstens zwei zugehörige Dialoge und ein hörbares Gespräch.
Konten unterstützen UDP/TCP/TLS sowie erforderliches SDES-SRTP mit TLS.
UDP/TCP und gewöhnliches RTP sind unverschlüsselt. TLS prüft Serverzertifikat und
Host über die Betriebssystem-Vertrauensregeln; es gibt keinen automatischen
Klartext-Rückfall. Die Gesprächsanzeige folgt beobachtetem Schutz, nicht der
Kontoeinstellung. Schutz bis zur Telefonanlage garantiert keine Ende-zu-Ende-Verschlüsselung.
Passwörter liegen im macOS-Schlüsselbund bzw. Windows Credential Manager.
Die [lokale macOS-Verifikation](docs/verification/2026-11-05-call-controls-security.md)
belegt Anmeldung, Vermittlung, Stummschaltung, Halten und zurückkehrendes Audio
sowie TLS-/SRTP-Erfolg und Fehlerfälle mit isoliertem Asterisk und kontrollierten
Loopback-Gegenstellen. Provider-Interoperabilität, physische Audiohardware und
Windows-Laufzeit bleiben gesondert zu verifizieren.

Die dunkle Desktop-Oberfläche startet mit 340 × 500 Pixeln. Sie bietet Wähltastatur,
macOS-Kontakte nach ausdrücklicher Freigabe, Favoriten und einen lokalen
SQLite-Anrufverlauf mit Rückruf.
Formatierte Rufnummern einschließlich führendem Plus werden vor dem Wählen validiert.
Freigegebene Kontakte erscheinen als Nummernvorschläge, in Gesprächen, Mitteilungen
und rückwirkend im Verlauf; die Zuordnung deutscher Vorwahlen ist abschaltbar.
Während aktiver Gespräche lässt sich eine Tastatur für native DTMF-Töne einblenden.
Acht Einstellungsbereiche bieten Feldsuche, Kontenaktivierung, Audiotests,
Klingeltöne, Hintergrundstart und Berechtigungschecks.
Anruferdetails in Mitteilungen lassen sich ausblenden; der Diagnosebericht enthält
keine persönlichen Daten. Kontaktanzeige und Verlaufsspeicherung lassen sich anpassen; abgeschlossene
Verlaufseinträge können nach einer gewählten Frist gelöscht werden. [Produktumfang](apps/desktop/PRODUCT.md),
[Designregeln](apps/desktop/DESIGN.md) und [Release-Notizen](docs/releases/0.1.0.md)
beschreiben die Vorschau. Windows-Kontakte, Kalender und automatische
Updates sind noch nicht verfügbar.

Release-Ziele sind macOS 14+ auf Apple Silicon und Windows 11 x64. Die Pakete
nutzen derzeit Ad-hoc-Signierung auf macOS und sind unter Windows unsigniert.
Für eine öffentliche Produktionsfreigabe sind die offenen Punkte der
[Release-Checkliste](docs/release-checklist.md) zu erfüllen.

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
bun run check:versions
bun run format:check
bun run test:ui
bun run test:ui:flows
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
bun run desktop:build --features native
```

Für portable Buildprüfungen kann `--features native` entfallen; dieser Build
meldet `notLinked` und enthält keine native Telefonie.

`bun run dev:web` öffnet nur die Web-Oberfläche; der native Status ist dort
ausdrücklich nicht verfügbar. Cargo nutzt ein gemeinsames `target/` im Root.
Ohne `--workspace` prüfen Cargo-Befehle standardmäßig nur die portablen Pakete.

## Aufbau

| Pfad                     | Verantwortung                                                      |
| ------------------------ | ------------------------------------------------------------------ |
| `apps/desktop/src`       | Svelte-Oberfläche                                                  |
| `apps/desktop/src-tauri` | Tauri-App, IPC, Kontenspeicherung, Verlauf und Desktop-Integration |
| `crates/phone-core`      | Plattformunabhängige Rust-Verträge und Validierung                 |
| `crates/baresip-adapter` | Sichere Anbindung der nativen Telefonie-Engine                     |
| `native`                 | Gepinnte native Builds und Lizenztexte                             |
| `tests/interop`          | Lokale Asterisk-Anlage und echter Audiotest                        |
| `.github`                | CI, plattformübergreifende Installer und Release-Entwürfe          |

## Weiterarbeiten

- [Architektur](docs/architecture.md) und [Entwicklungsablauf](docs/development.md)
- [Interne API und generierte Typen](docs/api/README.md)
- [GitHub, CI/CD und Signierung](docs/ci-cd.md)
- [MVP-Entwurf](docs/superpowers/specs/2026-11-04-sip-mvp-design.md)
- [Beitrag leisten](CONTRIBUTING.md)
- [Verhaltensregeln](CODE_OF_CONDUCT.md), [Sicherheitsmeldungen](SECURITY.md) und
  [Lizenzierung und Attribution](docs/licensing.md)

GitHub-Workflows prüfen Quellcode, Browserzustände, native Builds und das
Asterisk-Labor. Ein passender Versionstag erstellt nach erfolgreichen Checks einen
Entwurf als Vorabversion mit Installern und SHA-256-Prüfsummen. Ein Remote und
Repository-Regeln müssen am tatsächlichen Repository eingerichtet werden.
