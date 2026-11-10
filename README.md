<div align="center">

<img src="apps/desktop/src-tauri/icons/128x128.png" alt="Ruf-Appsymbol" width="80">

# Ruf

**Ein kompaktes SIP-Softphone für macOS und Windows.**

[![CI](https://github.com/lucaschoeneberg/ruf/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/lucaschoeneberg/ruf/actions/workflows/ci.yml)
[![Version: 0.1.0 Vorschau](https://img.shields.io/badge/Version-0.1.0%20Vorschau-3b82f6)](docs/releases/0.1.0.md)
[![Zielplattformen: macOS und Windows](https://img.shields.io/badge/Zielplattformen-macOS%20%7C%20Windows-64748b)](docs/development.md)
[![Lizenz: MIT](https://img.shields.io/badge/Lizenz-MIT-22c55e)](LICENSE)

[Screenshots](#screenshots) · [Funktionen](#funktionen) · [Lokal starten](#lokal-starten) · [Dokumentation](#weiterarbeiten)

</div>

Ruf verbindet deinen Desktop mit einer SIP-Telefonanlage. Wähle Nummern oder
Nebenstellen, verwalte mehrere Konten und behalte Kontakte und vergangene Anrufe
in einer übersichtlichen Oberfläche. Im Gespräch sind Stummschaltung, Halten,
DTMF-Tastatur und Vermittlung direkt erreichbar.

**Version 0.1.0 ist eine Vorschau für isolierte Testnetze.** Die Pakete sind noch
nicht für eine öffentliche Produktionsfreigabe qualifiziert. Den aktuellen Umfang
beschreiben die [Release-Notizen](docs/releases/0.1.0.md).

## Screenshots

<table>
  <tr>
    <th>Wählen</th>
    <th>Im Gespräch</th>
    <th>Kontakte</th>
  </tr>
  <tr>
    <td><img src="docs/screenshots/dialer.png" alt="Ruf mit Kontoauswahl und kompakter Wähltastatur" width="260"></td>
    <td><img src="docs/screenshots/call.png" alt="Gespräch mit Halten, Stummschaltung und Vermittlungsaktionen" width="260"></td>
    <td><img src="docs/screenshots/contacts.png" alt="macOS-Kontaktansicht mit Suche und Favoriten" width="260"></td>
  </tr>
</table>

<details>
<summary>SIP-Kontoeinstellungen ansehen</summary>

<img src="docs/screenshots/settings.png" alt="Übersicht der SIP-Konten und Kontoeinstellungen" width="900">

</details>

Die Abbildungen zeigen die aktuelle Oberfläche mit fiktiven Demo-Konten und
Kontakten. Sie sind keine Nachweise für reale Gespräche oder Betriebssystem-Freigaben.

## Funktionen

| Bereich                | Was Ruf bietet                                                                                                          |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Mehrere Konten**     | Bis zu acht gespeicherte SIP-Konten, unabhängig aktivierbar und mit auswählbarer ausgehender Identität.                 |
| **Gespräche**          | Annehmen, Ablehnen, Auflegen, Stummschaltung, Halten und native DTMF-Töne für Ziffern, Stern und Raute.                 |
| **Vermittlung**        | Direkt vermitteln oder eine Rückfrage starten, zwischen den Gesprächen wechseln und die Vermittlung abschließen.        |
| **Kontakte auf macOS** | Nach ausdrücklicher Freigabe suchen, Favoriten nutzen und passende Kontakte beim Wählen oder im Gespräch erkennen.      |
| **Lokaler Verlauf**    | Anrufergebnis und Dauer, Rückruf über das ursprüngliche Konto sowie einstellbare Speicherung und Aufbewahrung.          |
| **Audio**              | Mikrofon, Gesprächslautsprecher und Klingelton getrennt auswählen und mit kurzen Audiotests prüfen.                     |
| **Desktop**            | Tray-Menü, Hintergrundbetrieb, Tastenkürzel und Suche mit ⌘K beziehungsweise Strg+K; eingehende Mitteilungen auf macOS. |

Rust und Tauri 2 bilden die Desktop-Anwendung, Svelte die Oberfläche. SIP und
Audio laufen nativ über baresip. Der eigene Code steht unter der
[MIT-Lizenz](LICENSE); Abhängigkeiten behalten ihre jeweiligen Lizenzen.

## Stand und Grenzen

- Zielplattformen sind **macOS 14+ auf Apple Silicon** und **Windows 11 x64**.
  Ein erfolgreicher Build ersetzt keine Prüfung auf physischer Audiohardware
  oder gegen die eigene Telefonanlage. Ergebnisse und Grenzen stehen in den
  [Prüfnachweisen](docs/verification/README.md).
- Die macOS-Pakete sind derzeit ad hoc signiert, Windows-Installer unsigniert.
  Die [Release-Checkliste](docs/release-checklist.md) enthält die offenen
  Freigabeschritte.
- Konten unterstützen UDP, TCP und TLS sowie erforderliches SDES-SRTP mit TLS.
  UDP/TCP und gewöhnliches RTP sind unverschlüsselt. TLS prüft Zertifikat und
  Host über das Betriebssystem; ein automatischer Klartext-Rückfall ist
  ausgeschlossen. Die Gesprächsanzeige folgt dem tatsächlich beobachteten
  Schutz. Schutz bis zur Telefonanlage ist keine Ende-zu-Ende-Verschlüsselung.
- Höchstens zwei zusammengehörige Dialoge teilen sich ein hörbares Gespräch.
  Passwörter liegen im macOS-Schlüsselbund beziehungsweise Windows Credential Manager.
- Windows-Kontakte, Kalenderintegration und automatische Updates sind noch nicht verfügbar.

## Lokal starten

Voraussetzungen: Bun aus `.bun-version`, Rustup sowie die
[Tauri-Systemvoraussetzungen](https://v2.tauri.app/start/prerequisites/)
(Xcode auf macOS, MSVC Build Tools und WebView2 auf Windows). Für die native
Telefonie werden außerdem CMake und OpenSSL benötigt; die plattformspezifische
Einrichtung steht in [native/README.md](native/README.md).

```sh
bun install --frozen-lockfile
bun run native:build
bun run dev:native
```

Eine frische Installation startet ohne SIP-Konto. Trage die Zugangsdaten deiner
Telefonanlage in den Kontoeinstellungen ein; das Labor ist dafür nicht erforderlich.
Die Vorschau bleibt auf isolierte Testnetze beschränkt.
`bun run dev` startet Tauri ohne native Telefonie, `bun run dev:web` nur die
Web-Oberfläche ohne native API.

Für das optionale [Asterisk-Labor](tests/interop/asterisk/README.md) werden zusätzlich
Docker mit Compose, Python 3 und das `openssl`-Kommando benötigt:

```sh
bun run test:asterisk
# Nach Abschluss der Labortests:
bun run lab:down
```

`test:asterisk` erzeugt die synthetischen TLS-Testzertifikate, startet beziehungsweise
erstellt die isolierte Testanlage neu und lässt sie anschließend laufen.
Ein vorheriges `lab:up` ist auf einem frischen Checkout nicht erforderlich und kann
ohne die erzeugten Zertifikate scheitern. Testkonto und Nebenstellen stehen im
Laborleitfaden; die Test-CA wird nicht in den System-Vertrauensspeicher importiert.

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
```

Nach dem SDK-Build prüfen diese Befehle zusätzlich die native Variante:

```sh
cargo clippy --locked --workspace --all-targets --features sip-desktop/native -- -D warnings
cargo test --locked --workspace --features sip-desktop/native
bun run desktop:build --features native --bundles app,dmg
```

Der gezeigte Paketbefehl baut auf macOS App und DMG; auf Windows ist
`--bundles nsis` zu verwenden. Für portable Buildprüfungen entfällt
`--features native`; diese Variante meldet `notLinked` und enthält keine native
Telefonie. `linked` bestätigt ausschließlich die kompilierte Anbindung.

Cargo nutzt ein gemeinsames `target/` im Root.
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
- [Prüfnachweise und Grenzen](docs/verification/README.md)
- [Beitrag leisten](CONTRIBUTING.md)
- [Verhaltensregeln](CODE_OF_CONDUCT.md), [Sicherheitsmeldungen](SECURITY.md) und
  [Lizenzierung und Attribution](docs/licensing.md)

GitHub-Workflows prüfen Quellcode, Browserzustände, native Builds und das
Asterisk-Labor. Ein passender Versionstag erstellt nach erfolgreichen Checks einen
Entwurf als Vorabversion mit Installern und SHA-256-Prüfsummen. Die Einrichtung
von Repository-Regeln und die Produktionsfreigabe beschreibt der
[CI/CD-Leitfaden](docs/ci-cd.md).
