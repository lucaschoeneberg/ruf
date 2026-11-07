# Ruf: Suche, Menüs und Tastenkombinationen

Stand: 6. November 2026. Umsetzung im bestehenden Checkout; vorher vorhandene
Änderungen wurden beibehalten.

## Verhalten und Wiederverwendung

- Lupe im Telefonkopf und Cmd/Strg+K öffnen die Aktionssuche mit deutschen
  Suchbegriffen, sichtbaren Kürzeln und begründeten, gesperrten Gesprächsaktionen.
- Pfeiltasten wählen aus, Enter führt aus, Escape schließt und stellt den Fokus
  wieder her. Darstellung im Browser bei 340 × 500 und 320 × 440 geprüft.
- Cmd/Strg+1/2/3 wechseln die Ansicht; +L fokussiert das Nummernfeld, +F die
  passende Suche. Kontakte erhalten den Fokus auch nach asynchronem Laden.
- Ansicht-, Gespräch- und Hilfe-Menüs machen Navigation, Anrufaktionen und
  Kürzel auffindbar. Tauri registriert das macOS-Hilfe-Menü für die Systemsuche.
- Stummschaltung, Halten, Vermittlungsformular und Gesprächstastatur verwenden
  die vorhandenen `CallPanel`-Handler. Veraltete native Menü-Token werden
  ignoriert; Bestätigungssperren bleiben erhalten. Vermitteln öffnet zunächst
  das vorhandene Formular und löst keinen Transfer aus.
- Bestehende Tauri-Menüs, `listen`/`emit_to`, `perform`, `open_settings`,
  generierte Gesprächsmodelle, Einstellungsnavigation, Icons und Design-Tokens
  werden wiederverwendet. Keine neue Dependency oder zweite Audiopipeline.

## Geänderte Dateien dieses Auftrags

Neue Dateien:

- `apps/desktop/src/lib/shortcuts.ts`
- `apps/desktop/src/lib/components/ActionSearch.svelte`
- `tests/ui/shortcuts.test.ts`
- `tests/ui/browser/shortcuts.mjs`
- `docs/api/keyboard-navigation.md`
- `docs/verification/2026-11-06-keyboard-navigation.md`

Erweiterte Dateien:

- `apps/desktop/src/App.svelte`
- `apps/desktop/src/Settings.svelte`
- `apps/desktop/src/lib/components/CallPanel.svelte`
- `apps/desktop/src/lib/components/ContactsPanel.svelte`
- `apps/desktop/src/lib/components/HistoryPanel.svelte`
- `apps/desktop/src/lib/components/DtmfPad.svelte`
- `apps/desktop/src/lib/components/DiagnosticsSettings.svelte`
- `apps/desktop/src/lib/settings-navigation.ts`
- `apps/desktop/src-tauri/src/shell.rs`
- `apps/desktop/DESIGN.md`
- `docs/api/README.md`
- `tests/ui/browser/call-controls.mjs`
- `scripts/check-ui-flows.mjs`

## API-Dokumentation

[Keyboard navigation](../api/keyboard-navigation.md) dokumentiert Kürzel,
Menüverhalten, interne Fensterereignisse, erforderliche primitive Payloads,
Tokenprüfung, Zustandsvalidierung und Fehlerverhalten. Bestehende Invoke-Befehle
und Rust-DTOs wurden wiederverwendet. TypeScript-Generierung erzeugte keinen Diff.

## Ausgeführte Prüfungen

Rust-Prüfungen liefen mit der festgelegten Toolchain 1.98.1; dazu wurde
`/opt/homebrew/opt/rustup/bin` vor Homebrew-Rust in PATH gesetzt. Der erste
Versions-/Desktop-Versuch erkannte korrekt die abweichende Systemversion 1.99.1.

Erfolgreich:

- `bun run format:check`
- `bun run check` — keine Fehler oder Warnungen
- `bun run build`
- `bun run check:versions`
- `bun run types:generate` und Diff-Prüfung der generierten Dateien
- `bun run test:ui` — 93 Tests bestanden
- `cargo clippy --locked --workspace --all-targets -- -D warnings`
- `cargo test --locked --workspace` — einschließlich des vorhandenen explizit
  ignorierten Tests für echte Schlüsselbund-Schreibzugriffe
- `bun run native:build`
- `cargo clippy --locked --workspace --all-targets --features sip-desktop/native -- -D warnings`
- `bun run test:asterisk` — Registrierung und Medien-Smoke-Test, 16
  Asterisk-Szenarien, 13 Sicherheitsprüfungen und ein Neustart-/Incoming-Test
- `bun run desktop:build` — portables macOS-App- und DMG-Bundle
- `git diff --check`

Der Shortcut-Test erkennt experimentell das Entfernen der Wiederholungssperre.
Die vorübergehende Mutation wurde unmittelbar zurückgenommen. Die Browserprüfung
wurde zunächst ohne Suchschaltfläche rot beobachtet und danach grün ausgeführt.
Eine zusätzliche Fokusprüfung erforderte das Zurücksetzen ihrer synthetischen
Kontaktfreigabe, damit nachfolgende Berechtigungsszenarien unabhängig bleiben.

Ebenfalls erfolgreich abgeschlossen:

- `bun run test:ui:flows` — alle acht Abläufe: Kontakte, Verlauf, Telefon,
  Gesprächssteuerung, Tastenkürzel, Wählvorschläge, Einstellungen und Layout.
- `bun run desktop:build --features native` — natives macOS-App- und DMG-Bundle
  mit bestätigtem nativen SBOM-Buildkontext. Der erste Versuch scheiterte am
  lokalen LLVM-Suchpfad/Prozesslimit. Die Wiederholung mit
  `DYLD_FALLBACK_LIBRARY_PATH` auf das Toolchain-`lib`-Verzeichnis und
  `CARGO_BUILD_JOBS=2` war erfolgreich; Cargo gab dabei noch die gespeicherte
  Strip-Warnung des vorigen Buildscript-Builds aus.

## Verbleibende Grenzen

Die native macOS-Menüsuche wurde über die Tauri-Integration eingebunden und
kompiliert, aber nicht manuell im Systemmenü bedient. Windows-Laufzeit, externe
SIP-Anbieter und physische Audiogeräte wurden in diesem Auftrag nicht geprüft.
Lokale Bundles sind ad hoc signiert und nicht notarisiert. Kein Hosted-CI-Ergebnis
wird behauptet.

## Erweiterung: Kontakte und Verlauf gemeinsam durchsuchen

Auf ausdrücklichen Wunsch wurden Kontakte und Verlauf in „Ruf durchsuchen“
integriert. Ab zwei Zeichen erscheinen bis zu acht Treffer pro Quelle; weitere
Treffer öffnen die vorhandene Einzelansicht mit übernommenem Suchtext.
Kontakttreffer öffnen die Kontaktkarte, Verlaufstreffer die genauen Anrufdetails.
Ein Rückruf bleibt eine getrennte explizite Aktion. Telefonnummern verwenden die
bestehende Normalisierung. Die Kontakt-Anzeigepräferenzen werden berücksichtigt.

Zusätzlich zur bisherigen Änderungsliste wurde
`tests/ui/browser/global-search.mjs` angelegt; angepasst wurden `ActionSearch`,
`App`, `ContactsPanel`, `HistoryPanel`, `shortcuts.ts`, die Menübeschriftung in
`shell.rs`, der Browser-Runner, seine vorhandenen Shortcut-/Call-Control-Prüfungen,
`DESIGN.md` und die oben verlinkte API-Dokumentation.

Wiederverwendet: `getHistory`, `searchContacts`, generierte `Contact`-/
`HistoryEntry`-Modelle, `historyTitle`, `historyOutcomes`, `normalizeDialNumber`,
Kontakt-ID-Fokus und bestehende Detail-/Anrufsteuerung. Keine neue Dependency,
kein neuer IPC-Befehl und keine DTO-Änderung.

Neue Browserfälle prüfen gemeinsame Treffer, explizites Öffnen ohne Anruf,
formatierte Telefonnummern, Kontaktfreigabeentzug, verspätete Antworten,
Quellfehler, beide „Alle Treffer“-Übergaben, Suchtext-/Fokusübernahme und
IME-Bestätigung. Der neue Browserfall wurde vor der Implementierung rot beobachtet.
Abschließende Prüfungen dieser Erweiterung:

- `bun run check`: keine Fehler oder Warnungen.
- `bun run test:ui`: 96 Tests bestanden.
- `bun run test:ui:flows`: alle neun Browserabläufe bestanden, einschließlich
  neuer globaler Suche und abschließend wiederholtem Gesamtlauf.
- `bun run format:check`, `bun run check:versions`, `git diff --check`: bestanden.
- `cargo clippy --locked --workspace --all-targets -- -D warnings` und
  `cargo test --locked --workspace`: bestanden; der bestehende echte
  Schlüsselbund-Schreibtest bleibt absichtlich ignoriert.
- `bun run types:generate`: erfolgreich, generierte Dateien ohne Diff.
- Web-Produktionsbuild und `bun run desktop:build --features native`: bestanden;
  aktuelles natives macOS-App- und DMG-Bundle mit passendem SBOM erstellt.
  Zwei erste Paketversuche scheiterten an `Resource temporarily unavailable`
  (OS-Fehler 35). Mit `CARGO_BUILD_JOBS=1`, `TOKIO_WORKER_THREADS=1`,
  `RAYON_NUM_THREADS=1` und dem beschriebenen LLVM-Bibliothekspfad gelang der
  vollständige Build. Die ältere gecachte Strip-Warnung bleibt sichtbar.

Die zusätzliche Suche wurde mit synthetischen Kontakten/Anrufdaten und bei
320 × 440 Pixeln visuell geprüft. Die SIP-/Audioimplementierung blieb unverändert;
der oben dokumentierte Asterisk-Lauf stammt aus der ersten Erweiterung und wurde
für die reine gemeinsame Suche nicht wiederholt. Windows und die manuelle native
Menübedienung bleiben ungeprüft.

## Lokale Installation und Signatur

Am 6. November 2026 wurde auf Wunsch die ältere laufende Installation unter
`/Applications/Ruf.app` aktualisiert. Der neue native Build wurde zunächst separat
mit der gültigen vorhandenen Apple-Development-Identität signiert und mit
`codesign --verify --deep --strict` geprüft. Die designated requirement stimmt
mit der vorherigen Installation überein.

Ruf wurde über den regulären Quit-Ablauf beendet, die frühere App lokal als
Rollback-Sicherung behalten, die geprüfte neue App eingesetzt und erneut
geöffnet. Die installierte Binärdatei stimmt per SHA-256 mit dem vorher geprüften
signierten Update überein; Signaturprüfung, eingebetteter nativer SBOM-Buildkontext
und laufender neuer App-Prozess wurden danach bestätigt. Benutzerkonten,
Schlüsselbunddaten und Verlauf wurden nicht manuell verändert. Es handelt sich
um eine Apple-Development-Signatur, nicht um einen notarisierten Developer-ID-Release.
