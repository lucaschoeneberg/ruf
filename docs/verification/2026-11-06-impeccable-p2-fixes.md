# Ruf: P2-Korrekturen und erneuter Impeccable-Audit · 2026-11-06

## Implementierungsintegrität: bestanden

Die drei offenen Befunde aus dem [vorherigen Audit](2026-11-06-impeccable-all-screens.md)
sind in den ursprünglichen Browserreproduktionen behoben. Prüf-, Anfrage- und
Öffnungsfehler bleiben unterscheidbar. Einstellungen reagieren auch auf die
tatsächlich verfügbare Inhaltsbreite bei Vergrößerung. Die gemeinsame dunkle
Darstellung, acht Bereiche und direkte Kontennavigation bleiben erhalten.

| Dimension                  | Punkte               | Nachprüfung                                                                                                                                                      |
| -------------------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Barrierefreiheit           | 3/4                  | Status und Fehler widersprechen einander nicht mehr; fokussierte Felder und Navigation im vergrößerten Fenster erreichbar. Kein vollständiger Screenreader-Test. |
| Performance                | 3/4                  | Zusätzliche Layoutanpassung rein in CSS, ohne Mess-/Polling-Schleife oder Dependency; kein neues natives Laufzeitprofil erstellt.                                |
| Responsive Darstellung     | 4/4                  | Alle acht Einstellungsbereiche in normalen Größen und bei Vergrößerung geprüft; Felder bleiben unverdeckt.                                                       |
| Theming                    | 4/4                  | Bestehende Tokens, Schrift, Fehlerdarstellung und Fokusregeln wiederverwendet.                                                                                   |
| Implementierungsintegrität | 4/4                  | Bekannte Berechtigung wird nur bei fehlgeschlagenem Read verworfen; Status und Aktionsfehler bleiben getrennt.                                                   |
| **Gesamt**                 | **18/20 · Sehr gut** | **Bewertung der nachgeprüften Einstellungsflächen, keine neue Vollbewertung der gesamten App.**                                                                  |

**Offen im geprüften Umfang: 0 × P0, 0 × P1, 0 × P2, 0 × P3.**
Impeccable 4.3.1 `detect apps/desktop/src --json`: **0 mechanische Befunde**.
Die Punkte sind eine begründete manuelle Bewertung, keine Detektorausgabe oder
vollständige WCAG-Zertifizierung.

## A01: Mikrofon-Prüfung und Öffnungsfehler getrennt

In `PrivacySettings.svelte` erhält die Freigabeabfrage einen eigenen Fehlerzustand;
Anfrage und Öffnungsaktion verwenden den getrennten Aktionsfehler. Die
Freigabeanforderung ist weiterhin explizit, nur bei `notDetermined` verfügbar und
liest anschließend den aktuellen Systemstatus erneut. Ein fehlgeschlagener Read
setzt den Status auf unbekannt; Öffnungs-/Anfragefehler tun das nicht.

**Erneute Reproduktion:** `open_microphone_settings` schlägt fehl, während die
vorherige Abfrage `notDetermined` geliefert hat. Der Screen zeigt weiterhin
„Noch nicht angefragt“ und separat „Die Systemeinstellungen konnten nicht geöffnet
werden“ mit manuellem Weg. Die Prüfung läuft nicht fälschlich als fehlgeschlagen.
Mit `authorized` bleibt „Erlaubt“ erhalten. Ein erfolgreicher Öffnungsversuch
entfernt den alten Aktionsfehler.

Weitere Browserregressionen prüfen fehlgeschlagene Anfrage mit erfolgreichem
Wiederholungsversuch sowie fehlgeschlagenen Read nach bekannter Autorisierung.
Reines Prüfen fordert keine Mikrofonfreigabe an und startet keine Aufnahme.

## A02: Kontakte-Ladezustand folgt dem tatsächlichen Read

`ContactsSettings.svelte` unterscheidet laufende Prüfung, Prüfungsfehler,
Aktionsfehler und bekannten Freigabestatus. Ein fehlgeschlagener Read verwirft
auch eine zuvor bekannte Freigabe; ein Fehler beim Öffnen/Anfordern tut das nicht.
Die vorhandenen nativen APIs und der gemeinsame Fehlerbereich bleiben bestehen.

**Erneute Reproduktion:** Der erste `get_contacts_permission`-Aufruf schlägt fehl.
Nach Ende des Aufrufs steht „Prüfung fehlgeschlagen“ statt „Freigabe wird geprüft …“
im Status. Die Fehlermeldung erklärt die Wiederholung. Ein erfolgreicher Retry
zeigt „Freigegeben“ und entfernt den Abfragefehler.

Weitere Browserregressionen prüfen Öffnungsfehler bei bekannter Freigabe,
fehlgeschlagenen erneuten Read sowie Anfragefehler mit erfolgreichem Retry.
Freigabeprüfungen fordern keinen Zugriff an und lesen keine Kontaktinhalte.

## A03: Vergrößerte Einstellungen passen in ihren Inhaltsbereich

Der Einstellungs-Mount füllt das Fenster mit festen Insets. Die Höhe der Shell
folgt diesem Mount. Zwei benannte CSS-Container messen Fenster und Hauptbereich:

- Bis 480px tatsächlicher Fenster-Containerbreite: 136px Seitenleiste, vertikal
  scrollbar; Navigationstexte bleiben sichtbar, redundante Icons entfallen.
  „Deine Konten“ bleibt als direkter Block unter der Bereichsnavigation erhalten.
- Bis 400px Hauptbereich: einspaltige Zeilen und Aktionen, 12px seitlicher Abstand,
  begrenzte Kontrollbreiten und umbrechende Breadcrumbs/Schaltflächen.
- Die Speicherleiste läuft bei engem Inhalt im Formular mit. Sie verdeckt keine
  vergrößerten Felder. Bei normaler Fenstergröße bleibt sie wie bisher haftend.

**Erneute Messung bei 740 × 540 und CSS-Zoom 2:** Hauptbereich **234px**,
scrollender Inhalt ebenfalls **234px**, statt vorher **170px / bis zu 228px**.
Alle acht Einstellungsbereiche bestehen den Überlauftest. Die nativen Range-Controls
melden weiter zwei Pixel Differenz innerhalb ihrer Zeile; das erzeugt weder
horizontalen Hauptbereichs-Scroll noch einen festgestellten Bedienverlust und
bleibt wie im Ausgangsaudit als Fehlalarm ausgeschlossen.

## Browserprüfung und Grenzen

Erneuter Audit über `PrivacySettings`, `ContactsSettings`, `PermissionStatus`,
die Einbindung in `Settings.svelte` und gemeinsame CSS-Regeln. Da die
Layoutänderung alle Einstellungen betrifft, wurden alle acht Bereiche geprüft:
Allgemein, SIP-Konten, Ton, Mitteilungen, Kontakte & Verlauf, Datenschutz,
Diagnose & Hilfe, Über Ruf.

- 900 × 660 und 740 × 540, jeweils oberer und unterer Inhalt.
- 740 × 540 mit CSS `body { zoom: 2 }`.
- 370 × 270 ohne CSS-Zoom als Layoutäquivalent eines halbierten Browserviewports.
  Dies ist keine Behauptung eines beobachteten nativen WebView-Zooms.
- Insgesamt **34 zusätzliche Messfälle**: 32 Bereich-/Größenkombinationen und
  die beiden ursprünglichen Berechtigungsfehler.
- Permanente Layoutregression mit acht Konten, geöffneten erweiterten
  Kontoeinstellungen und 200-%-Vergrößerung: keine internen Überläufe über zwei
  Pixel Toleranz; erster/letzter sichtbarer Eingabekontrollpunkt innerhalb des
  Fensters und per `elementFromPoint` unverdeckt; Navigation und Kontoname per
  Tastatur fokussierbar.
- Keine unbenannten sichtbaren Buttons/Formularfelder in der zusätzlichen
  DOM-Prüfung. Farben, Kontrasttokens und reduzierte Bewegung bleiben erhalten.

Lokale, ignorierte Artefakte: `output/playwright/p2-audit-results.json`,
`review-*-top.png`, `review-*-bottom.png`, `review-*-zoom-200.png`,
`review-*-370.png`, `review-microphone-open-error.png`,
`review-contacts-read-error.png` und `settings-*-zoom-200.png`.
Screenshots verwenden ausschließlich synthetische IPC-Daten.

## Dateien, Wiederverwendung und API-Dokumentation

- `apps/desktop/src/lib/components/PrivacySettings.svelte`: getrennte Prüf- und
  Aktionsfehler, Anfrage/Recheck nach vorhandenem Mitteilungs-Pattern.
- `apps/desktop/src/lib/components/ContactsSettings.svelte`: expliziter
  Prüfzustand, getrennte Fehlerarten und Umgang mit unbekanntem Status.
- `apps/desktop/src/app.css`: begrenzte Containerregeln für Einstellungen.
- `tests/ui/browser/settings.mjs`: sechs zusätzliche Browserregressionen mit
  Erweiterung der vorhandenen IPC-Fixture um die relevanten Fehlerpfade.
- `tests/ui/browser/layout.mjs`: tatsächliche CSS-Vergrößerung, interner Überlauf,
  verdeckte/fokussierte Felder und Navigation.
- `apps/desktop/DESIGN.md`: Containerverhalten dokumentiert.
- Dieser Bericht und der Verweis auf die Nachprüfung im ursprünglichen Audit.

Vorhandene `PermissionStatus`-Komponente, Freigabe-APIs, Fehler-/Statusdarstellung,
CSS-Tokens, Einstellungen und Browserrunner wiederverwendet. Keine neue Dependency,
kein zusätzliches SIP-/Audioverhalten, keine neuen IPC-Befehle oder DTOs.
API-/DTO-Verträge bleiben unverändert; keine API-Dokumentationsänderung nötig.
Der bekannte veraltete Design-Sidecar wurde nicht nebenbei neu erzeugt.

## Verifikation

Vor der Implementierung scheiterten die neuen Settings-Regressionsprüfungen an
den erwarteten Statusfehlern; die Layoutprüfung scheiterte in Allgemein mit
370px Shellbreite und bis zu 228px Inhalt in der 170px Hauptspalte.
Belege: `p2-settings-red.log`, `p2-layout-red.log` im lokalen Playwright-Ordner.
Beim Nachschärfen der Fokusprüfung wurden zunächst auch Felder in geschlossenen
Disclosures erfasst; die Prüfung öffnet jetzt die erweiterten Kontoeinstellungen
und prüft sichtbare Bedienelemente.

| Prüfung                                                          | Abschließend beobachtet                                                                                                           |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `bun run check`                                                  | Bestanden; Svelte 0 Fehler / 0 Warnungen, TypeScript ohne Fehler.                                                                 |
| `bun run test:ui`                                                | 88 bestanden, 0 fehlgeschlagen.                                                                                                   |
| `SIP_UI_CAPTURE=1 bun run test:ui:flows`                         | Alle sieben Flows bestanden, einschließlich der sechs zusätzlichen Berechtigungsfälle und acht vergrößerten Einstellungsbereiche. |
| Zusätzlicher Impeccable-Audit                                    | 34 Messfälle, ursprüngliche Fehlerreproduktionen korrigiert; 0 mechanische Detektorbefunde.                                       |
| `bun run build`                                                  | Bestanden; etwa 354 kB JavaScript / 94 kB gzip, 36,5 kB CSS / 7 kB gzip.                                                          |
| `bun run check:versions`                                         | Bestanden mit gepinnter Rust-Version.                                                                                             |
| `bun run types:generate`                                         | Bestanden; generierte `AppInfo.ts` unverändert.                                                                                   |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Bestanden mit Rust 1.98.1.                                                                                                        |
| `cargo test --locked --workspace`                                | 120 bestanden, 1 expliziter OS-Schlüsselbundtest ignoriert; portable Ausführung.                                                  |
| `bun run desktop:build`                                          | Bestanden; macOS aarch64 portable, `.app`, `.dmg`, SBOM und Hashnachweise erzeugt.                                                |
| `bun run format:check`, `git diff --check`                       | Bestanden.                                                                                                                        |

Rust-Kommandos verwenden `/opt/homebrew/opt/rustup/bin` vor Homebrew Rust auf
`PATH`. Der Desktop-Aufruf verwendet wie im vorherigen Auftrag zusätzlich
`CARGO_BUILD_JOBS=2` und die vorhandene Toolchain-Bibliothek über
`DYLD_LIBRARY_PATH=/Users/lucaschoeneberg/.rustup/toolchains/1.98.1-aarch64-apple-darwin/lib`.
Keine Toolchainkonfiguration verändert. Cargo gab noch zwischengespeicherte
Strip-Warnungen aus; der gesamte Paketierungs-/Exportlauf endete mit Exitcode 0.
Die erzeugten Pakete sind lokal ad hoc signiert und nicht notarisiert.

Prüflogs liegen lokal unter `output/playwright/p2-*.log`, insbesondere
`p2-ui-flows-final.log`, `p2-check.log`, `p2-rust-tests.log`,
`p2-web-build.log` und `p2-desktop-build.log`.

Keine beobachtete native Screenreader-/Zoomprüfung, Windows-Laufzeit, echten
Systemdialoge, SIP-Registrierungen, Asterisk-/Hardware-Audiotests oder gehostete CI
in diesem Auftrag. Der portable Desktop-Build ist kein Nachweis dieser Funktionen.

## Folgeschritte

Für A01–A03 ist keine weitere Korrektur offen. Ein optionaler `$impeccable polish`
kann vor einem Release native Zoom-/Screenreader- und Plattformprüfungen ergänzen.
