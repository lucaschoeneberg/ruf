# Buildbezogene SBOM: Umsetzung und Prüfung

Stand: 5. November 2026. Drei technische Lücken des vorherigen Projektinventars
wurden für den hier geprüften Build behoben: native OpenSSL-Abdeckung, Abhängigkeitsbeziehungen und Build-Zuordnung.
Das ist keine pauschale Konformitätserklärung für den gesamten CRA.

## Tatsächliches Ergebnis

Der geprüfte macOS-arm64-Native-Build enthält eine CycloneDX-1.6-SBOM mit
**281 Komponenten**, davon **221 mit Runtime-Scope `required`** und **60 mit
Build-Scope `excluded`**. OpenSSL **3.5.9** wird aus dem tatsächlich ausgewählten
SDK übernommen. Der geprüfte portable Build enthält **278 Komponenten** und
keine Einträge für baresip, libre oder OpenSSL. Diese Zahlen gelten für die hier
geprüften Eingaben und sind nicht im Produkt fest einprogrammiert.

- Cargo-Metadaten werden auf Zielplattform und Native-Feature gefiltert und vom
  Desktop-Paket aus traversiert. Build-/Proc-Macro-Abhängigkeiten werden als
  solche gekennzeichnet, nicht als Laufzeitbibliotheken ausgegeben.
- JavaScript-Pakete und deren Importbeziehungen stammen aus dem gerenderten
  Vite-Modulgraphen und den tatsächlich installierten Paketmanifesten. Reine
  Frontend-Build-Werkzeuge und fremde Plattformpakete werden nicht pauschal aus
  dem gesamten Lockfile übernommen.
- Native SDK-Target, Quellpins und angewendete Patchtexte müssen stimmen.
  OpenSSL-Version, native Archiv-Hashes und Patch-Hashes werden erfasst.
  Die ausgewählten baresip-Module werden dem baresip-Eintrag zugeordnet.
- Jeder Eintrag erhält eine Referenz; Kanten müssen auf vorhandene Einträge
  zeigen, und jeder Eintrag muss vom Produkt aus erreichbar sein.
- Der Desktop-Wrapper gibt einen einzigen Target-/Profil-/Native-Kontext an
  Vite und Tauri. Die native CLI läuft in einem Kindprozess, damit Bun/N-API
  bereits beim Start dieselbe OS-Umgebung sieht. Das Rust-Buildskript lehnt
  Abweichungen vom tatsächlichen Cargo-Build ab und bettet die validierte
  JSON-Datei als unveränderlichen Snapshot ein.
- IPC und CLI-Export lesen genau diesen Snapshot. Export und Metadatenausgabe
  starten weder Tauri noch Konten, Kontakte oder Telefonie.
- Neben dem Installer liegen eine target-/variantenbezogene `.cdx.json` und
  eine `.build.json` mit SHA-256-Zuordnung zu SBOM, Executable und Installer.
  Die lokalen Workflow-Änderungen behalten diese Dateien auch für Release-Drafts.

Die Über-Ruf-Seite behält ihren kompakten Aufbau. Softwaredetails bleiben
zugeklappt; die Übersicht zeigt ausschließlich passende zentrale Komponenten.
Die vollständige Datei kommt in der Desktop-App über den settings-exklusiven
`get_software_bill`-Befehl. Entwicklungsvorschauen sind als solche gekennzeichnet;
eine Release-App ersetzt einen Lesefehler nicht durch ein Projektinventar.

## Nachweis der Artefakt-Zuordnung

Geprüft wurden der durch die CLI aus dem Programm exportierte Snapshot, die
beigelegte Datei und der Installer. Die Exportdatei ist bytegleich zur Sidecar-
SBOM; deren Hash, Executable-Hash und Installer-Hash stimmen mit dem Manifest
überein. Die Datei besteht die offizielle CycloneDX-1.6-Schemaprüfung.

Für den abschließend geprüften Native-Build:

| Artefakt                                         | SHA-256                                                            |
| ------------------------------------------------ | ------------------------------------------------------------------ |
| `Ruf_0.1.0_aarch64-apple-darwin_native.cdx.json` | `ab68f3eb179580c9a70ae877c8cd706c87e885fac5ec1821ae62b06fc432cfc4` |
| `Ruf_0.1.0_aarch64.dmg`                          | `f3217da96801945e240a327a7c81c8627dbc7e4ff530e4910f37b075e9839a97` |

Die Dateien befinden sich unter
`target/aarch64-apple-darwin/release/bundle/dmg/` und sind Build-Ausgaben,
keine eingecheckten Projektdateien. Zeitstempel und SBOM-Seriennummer werden
pro Build erzeugt; eine spätere Neuerzeugung erhält deshalb andere Hashes.

## Ausgeführte Checks

- `bun run check`: keine Fehler/Warnungen; `bun run build` erfolgreich.
- `bun run test:ui`: **54 Tests bestanden**. Native-SDK-Verträge werden auch mit
  synthetischen Eingaben ohne installiertes SDK geprüft; zusätzlich lief hier
  der Test gegen das echte macOS-SDK. Der Windows-Cargo-Graph wurde geprüft.
- `bun run test:ui:flows`: alle fünf Gruppen bestanden, einschließlich
  kompakter Softwareübersicht, lokalem SBOM-Lesen und bestehender Firmenlinks.
- Screenshots bei 900 × 660 und 740 × 540 geprüft; keine horizontale Überbreite.
- `bun run check:versions` mit dem gepinnten Rust/Cargo **1.98.1** und Bun 1.4.4.
  Wiederholte DTO-Generierung blieb bytegleich.
- Workspace-Clippy mit `-D warnings` und Workspace-Rust-Tests jeweils ohne
  und mit `sip-desktop/native`: bestanden; vorhandene ignorierte Tests bleiben
  als solche ausgewiesen. Die Native-Suite enthielt echte lokale SIP-Timeouts.
- `bun run native:build` erfolgreich vor den Native-Checks ausgeführt.
- `bun run desktop:build --features native --bundles dmg` erfolgreich; portable
  App-Build und erneuter portabler Release-Build mit `--no-bundle` erfolgreich.
- Negative Kontrolle: OpenSSL wurde nur in einer temporär veränderten Build-
  Datei entfernt. `cargo check --release --features native` verweigerte den
  Build mit `Native SBOM must contain the actual SDK OpenSSL version`.
  Die Originaldatei wurde unmittelbar wiederhergestellt.
- Native und portable CLI-Exporte mit Sidecars verglichen und gegen das
  offizielle JSON-Schema geprüft. Fehlende/zusätzliche Exportargumente enden
  mit Exit-Code 2, ohne die App zu starten.
- GitHub-Workflow-Syntax mit actionlint 1.7.12 geprüft; Format und Diff geprüft.

## Geänderte Dateien und Wiederverwendung

- `apps/desktop/build/software-bill.ts`, `software-bill-frontend.ts`,
  `sbom-runtime.ts`: vorhandener Vite-Plugin-Weg, Cargo-Metadaten, Bun und SDK.
- `apps/desktop/src/lib/software-bill.ts`, `components/AboutPanel.svelte`,
  `api.ts`: Standardformat, bestehende Settings-Oberfläche und IPC-Wrapper.
- `apps/desktop/src-tauri/build.rs`, `sbom_build.rs`, `Cargo.toml`,
  `src/about.rs`, `src/lib.rs`, `src/main.rs`, `capabilities/settings.json`,
  `permissions/autogenerated/get_software_bill.toml`: Buildprüfung, Snapshot und
  Export; serde_json wird zusätzlich als Build-Abhängigkeit wiederverwendet.
  Es wurde kein zusätzliches Fremdpaket eingeführt; Cargo.lock blieb unverändert.
- `scripts/build-desktop.mjs`, `desktop-context.mjs`, `tauri-run.mjs`,
  `package.json`: bestehende Desktop-Build- und Dev-Einstiege.
- `.github/workflows/desktop-build.yml`, `release.yml`: Sidecars erhalten.
- `tests/ui/software-bill.test.ts`, `software-bill-frontend.test.ts`,
  `desktop-context.test.ts`, `browser/settings.mjs`: inhaltliche Regressionen.
- `docs/api/about.md`, `docs/api/README.md`, `docs/licensing.md`,
  `docs/ci-cd.md` und dieser Prüfbericht: IPC-/CLI-Verträge und Build-Ablauf.

Die API-Dokumentation beschreibt Pflicht-/optionale Felder, Bedeutung der
Build-Properties, Zugriffsbeschränkung, Entwicklungs-Fallback, Fehler und CLI-
Exit-Codes. Bestehende AppInfo-/ts-rs-DTOs und die SIP-/Audio-Pipeline bleiben
unverändert. Vorhandene fremde Working-Tree-Änderungen wurden beibehalten.

## Grenzen

Windows-Laufzeit, Windows-Installer und gehostete GitHub-Artefakt-/Release-Läufe
wurden nicht ausgeführt. Ein neuer Asterisk-Laborlauf gehörte nicht zu dieser
Build-/Inventuränderung. Das macOS-Paket ist lokal signiert, nicht notarisiert.
Die lokale Rust-Installation meldete beim Strippen einzelner Release-Artefakte
eine `rust-objcopy`/`libLLVM.dylib`-Warnung; die Builds und Exporte waren erfolgreich.
Die anfänglich falsche Homebrew-Rust-Version wurde durch explizite Auswahl der
bereits installierten gepinnten Toolchain für die abschließenden Checks vermieden.

Die Inventur erfasst die ausgewählten Paket-/SDK-Komponenten und Schriften;
eine vollständige Lizenz-/Quellfileinventur sämtlicher eingebetteter Upstream-
Teilkomponenten und weitere CRA-Verpflichtungen bleiben eigene Prüfaufgaben.
