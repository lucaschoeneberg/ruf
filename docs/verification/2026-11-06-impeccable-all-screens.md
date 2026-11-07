# Ruf: Impeccable audit und clarify · 2026-11-06

> **Nachprüfung:** Die drei Befunde A01–A03 wurden anschließend behoben.
> [Umsetzung, Regressionstests und erneuter Impeccable-Audit](2026-11-06-impeccable-p2-fixes.md).
> Die nachfolgenden Befunde und 16/20 Punkte dokumentieren den ursprünglichen Stand.

## Implementierungsintegrität: bestanden, mit offenen Zustandsfehlern

Ruf verwendet ein zusammenhängendes, produktspezifisches System: kompakte
Telefonansichten, separate Einstellungen mit acht Bereichen, gemeinsame dunkle
Tokens, lokale IBM Plex Sans, echte Zustandsmodelle und benannte Bedienelemente.
Der Impeccable-4.3.1-Detektor liefert für `apps/desktop/src` **0 Befunde**.
Die zusätzliche Code- und Browserprüfung findet drei technische Probleme, die
im reinen `audit` dokumentiert bleiben. `clarify` verbessert die Texte direkt.

| Dimension                  | Punkte          | Einordnung                                                                                                                         |
| -------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Barrierefreiheit           | 3/4             | Benannte Bedienelemente, Fokusführung und Statusmeldungen; widersprüchliche Berechtigungsmeldungen bleiben offen.                  |
| Performance                | 3/4             | Begrenzte Listen/Paginierung, entprellte Verlaufssuche, keine auffälligen Layoutschleifen; gemeinsames JavaScript-Paket.           |
| Responsive Darstellung     | 3/4             | Reguläre Fenstergrößen geprüft; deutlicher Überlauf in der zusätzlichen 200-%-Vergrößerung.                                        |
| Theming                    | 4/4             | Konsistente dunkle Tokens und geprüfte Textkontraste. Eine helle Darstellung gehört nicht zum Produktauftrag.                      |
| Implementierungsintegrität | 3/4             | Ehrliche Telefoniezustände und wiederverwendete Verträge; zwei Fehlerpfade in Datenschutz widersprechen dem tatsächlichen Vorgang. |
| **Gesamt**                 | **16/20 · Gut** | **Gezielte technische Nacharbeit erforderlich.**                                                                                   |

**Offen: 0 × P0, 0 × P1, 3 × P2, 0 × P3.** Das ist eine technische
Desktop-Einschätzung, keine vollständige WCAG-Konformitätsprüfung. Die Scores
wurden aus der geprüften Implementierung abgeleitet, nicht vom Detektor erzeugt.

## Umfang und Nachweise

Codeprüfung aller gerenderten Svelte-Oberflächen und ihrer gemeinsamen CSS-,
Such-, Validierungs- und IPC-Hilfen. Browserprüfungen mit den bestehenden
synthetischen IPC-Fixtures, ohne persönliche Daten und ohne echte Telefonie.

- Telefon: **340 × 500** und Mindestgröße **320 × 440**.
- Einstellungen: **900 × 660** und Mindestgröße **740 × 540**, jeweils oberer
  und unterer Inhalt aller acht Bereiche.
- Zusatzprüfung: alle acht Bereiche bei **740 × 540 mit CSS `body { zoom: 2 }`**.
  Das ist eine Vergrößerungssimulation, kein Nachweis nativer WebView-Zoom- oder
  Screenreader-Unterstützung. Der bestehende 125-%-Test verändert überwiegend
  Gerätemetriken und ersetzt diese Prüfung nicht.
- Bestehende Browserflows prüfen Tastaturbedienung, Fokuswiederherstellung,
  Suchnavigation, leere Ergebnisse, langsame Antworten, Fehler, Wiederholung,
  Speichern/Verwerfen, Löschbestätigungen und lange Identitäten.
- Frische zusätzliche Auditaufnahmen: `output/playwright/review-*-top.png`,
  `review-*-bottom.png`, `review-*-zoom-200.png`,
  `review-microphone-open-error.png`, `review-contacts-read-error.png`.
  Messdaten: `output/playwright/review-results.json`.
- Die lokalen Aufnahme-/Messartefakte bleiben ignoriert und werden nicht eingecheckt.

## Prüfung pro Screen und zugehöriger Zustände

| Screen                     | Geprüft                                                                                                                                                 | Ergebnis von clarify / audit                                                                                                                         |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Telefonrahmen und Tastatur | Kontoauswahl, Anmeldung, Laden/Fehler, Wiederholung, Nummerneingabe, Plus/Löschen, Kontaktvorschläge, Vorschau                                          | Laufende Anmeldung heißt jetzt „Wird angemeldet …“; Nummern- und Fehlertexte bereits konkret.                                                        |
| Starthilfe                 | Erstes Konto, gespeichertes Konto, Anmeldung läuft/fehlgeschlagen/bestätigt, Vorschau und fehlende Telefonie                                            | „Dein SIP-Konto ist angemeldet“ beschreibt Registrierung präzise; ein gestarteter Anruf wird nicht mehr als bereits bestehendes Gespräch bezeichnet. |
| Kontakte                   | Freigabe, Sperre, Einschränkung, Laden/Fehler, Liste, Suche, Favoriten, mehrere Nummern, Details, Kopieren, Anruf                                       | Gleichzeitige Favoriten- und Textsuche nennt jetzt „Keine passenden Favoriten“ und bietet passende Rückkehr zu allen Kontakten.                      |
| Verlauf                    | Leerzustand, Suche/Filter, Kontaktbezug, Paginierung, Details, Rückruf, Kopieren, einzelne/gesamte Löschung, Speicherfehler                             | Einheitliches „Verlauf löschen …“; Bestätigungen nennen Verlauf, abgeschlossene Anrufe und Unwiderruflichkeit.                                       |
| Anruf und Gespräch         | Eingehend, Annehmen/Ablehnen, Aufbau, Early Media, aktiv, gehalten, Stumm, Fehler, Sicherheit, DTMF                                                     | Vorhandene Status-/Sicherheitsbegriffe erhalten; aktueller Gesprächszustand bleibt maßgeblich.                                                       |
| Vermittlung und Rückfrage  | Zielnummer, Kontaktwahl, direkte Vermittlung, Rückfrage, Wechseln, Abbrechen, Abschluss und Fehler                                                      | Hilfetext erklärt Übergabe ohne Rückfrage und Halten beim Rückfragen; „Vermittlungsziel schließen“ benennt die reine Formularaktion.                 |
| Allgemein                  | Autostart, versteckter Start, Hintergrundbetrieb, Speichern/Verwerfen                                                                                   | Versteckter Start wird ausdrücklich auf automatisches Öffnen bezogen.                                                                                |
| SIP-Konten                 | Übersicht, Auswahl, Hinzufügen/Bearbeiten/Entfernen, Pflichtfelder, gespeichertes/fehlendes Passwort, erweiterte Verbindung, Validierung, Entwurfschutz | „Angemeldet“/„Noch nicht angemeldet“ vereinheitlicht; erforderliche Angaben, Serverformat, Benutzername und optionale separate Kennung erklärt.      |
| Ton                        | Geräte/fehlende Geräte, Systemstandard, Klingelton/-lautstärke, Hörprobe, Mikrofon-/Lautsprechertest, Speichern                                         | Lautstärketext nennt ausdrücklich den Klingelton; vorhandene Testdauer-/Speicherhinweise bleiben.                                                    |
| Mitteilungen               | App-Schalter, Anruferdetails, Systemfreigabe, Fehler, Testmitteilung, Speichern                                                                         | Bereits konkrete Trennung von Freigabe und Zustellung; keine unnötigen Textänderungen.                                                               |
| Kontakte & Verlauf         | Sortierung, Firma, Nummernzuordnung, Speicherung, Aufbewahrung/Bestätigung, Löschung                                                                    | Verlaufsspeicherung beschreibt Anrufdetails und grenzt Audio ab; Löschung nennt abgeschlossene Anrufe.                                               |
| Datenschutz                | Mikrofon, macOS-Kontakte, Freigabeaktionen, lokale Daten und Fehler                                                                                     | Freigabeaktion heißt wie im Telefon „macOS-Kontakte freigeben“; zwei technische Zustandsprobleme A01/A02 dokumentiert.                               |
| Diagnose & Hilfe           | Bericht, Verfügbarkeit, Freigaben, Audio/Schlüsselbund/Verlauf, Kopieren, Hilfe, Tastatur, Entwicklungs-Audio                                           | Technische Begriffe stehen im passenden Hilfekontext; vorhandene Hinweise zur Aussagekraft bleiben erhalten.                                         |
| Über Ruf                   | Version, Änderungen, Telefoniegrenzen, Software/Lizenzen, Inventar, Links                                                                               | „Dialoge“ durch erklärten Wechsel zwischen zwei Gesprächen ersetzt; Lizenztexte und rechtliche Inhalte unverändert.                                  |
| Gemeinsame Interaktionen   | Suche/kein Treffer, Fokusziele, ungespeicherte Änderungen, laufende Vorgänge, Kontenlimit                                                               | Bestehende Aktionen und Schutzmechanismen wiederverwendet; A03 betrifft die zusätzliche Vergrößerung.                                                |

## Offene technische Befunde

### A01 · [P2] Öffnungsfehler wird als Mikrofon-Freigabefehler angezeigt

**Ort:** `apps/desktop/src/lib/components/PrivacySettings.svelte:44`,
`apps/desktop/src/lib/components/PermissionStatus.svelte:47`.
**Kategorie:** Implementierungsintegrität / Barrierefreiheit.

**Reproduktion:** Mikrofonstatus synthetisch `notDetermined`; ausschließlich
`open_microphone_settings` schlägt fehl. Der Status wechselt zu „Prüfung
fehlgeschlagen“, obwohl keine Freigabeabfrage fehlgeschlagen ist. Der Fehlertext
fordert zudem pauschal eine Freigabe, obwohl der bekannte Status nicht durch
die fehlgeschlagene Öffnungsaktion verändert wurde.

**Auswirkung:** Nutzer können einen Fehler beim Öffnen nicht von einem unbekannten
oder abgelehnten Freigabestatus unterscheiden und verfolgen die falsche Ursache.

**Empfehlung:** Fehler der Freigabeabfrage, Freigabeanforderung und Öffnungsaktion
getrennt verwalten. Nur ein fehlgeschlagener Read darf den bekannten Status
verwerfen; die Öffnungsaktion bekommt eine eigene Fehlermeldung mit manuellem Weg.
Kein eigenständiger WCAG-Verstoß allein aus dieser Reproduktion abgeleitet.
**Nächster Befehl:** `$impeccable harden`.

### A02 · [P2] Fehlgeschlagene Kontakteprüfung bleibt als laufend beschriftet

**Ort:** `apps/desktop/src/lib/components/ContactsSettings.svelte:24` und `:92`.
**Kategorie:** Implementierungsintegrität / Statuskommunikation.

**Reproduktion:** Beim ersten Öffnen von Datenschutz schlägt
`get_contacts_permission` fehl. `working` wird korrekt zurückgesetzt und eine
Fehlermeldung eingeblendet. Weil `permission` null bleibt, behauptet die
Statuszeile weiterhin „Freigabe wird geprüft …“.

**Auswirkung:** Nutzer warten auf eine beendete Operation; Ladestatus und
Fehlerhinweis widersprechen einander. Auch der zugängliche Status vermittelt
einen weiterlaufenden Vorgang.

**Empfehlung:** Laden anhand des laufenden Reads anzeigen; fehlgeschlagen,
unbekannt und nie geprüft gesondert ausdrücken. Ein Fehler beim Öffnen der
Systemeinstellungen darf einen bekannten Kontaktstatus ebenfalls nicht ändern.
Kein vollständiger Screenreader-Test durchgeführt.
**Nächster Befehl:** `$impeccable harden`.

### A03 · [P2] Einstellungen verlieren bei 200-%-Vergrößerung nutzbare Breite

**Ort:** `apps/desktop/src/app.css:1004`, `:1155` und `:1176`.
**Kategorie:** Responsive Darstellung / Barrierefreiheit.

**Messung:** Bei 740 × 540 und CSS-Zoom 2 bleiben der Einstellungs-Hauptspalte
170 CSS-Pixel. Je nach Bereich misst deren scrollender Inhalt 204–228 Pixel.
Die Seitenleiste beansprucht weiter 200 CSS-Pixel; Medienabfragen orientieren
sich an der unveränderten Fensterbreite. Kontoaktionen und Formularfelder
überlaufen ihre Spalte, Kontonamen brechen teilweise zeichenweise um.

**Auswirkung:** Vergrößerte Schrift führt zu zusätzlichem horizontalem Scrollen
und erschwert das gleichzeitige Lesen von Beschreibung und Bedienung. Normale
Desktopgrößen bleiben davon getrennt bewertet.

**Empfehlung:** Den verfügbaren Inhaltsbereich als Grundlage für Umbrüche nutzen;
Seitenleiste bei starker Vergrößerung komprimieren/umschalten und Bedienelemente
innerhalb der tatsächlichen Spaltenbreite halten. Anschließend echten nativen
Text-/WebView-Zoom sowie Browserzoom prüfen. Bezug: WCAG 1.4.4/1.4.10 als
weiterer Prüfauftrag; die Simulation allein beweist keinen nativen Verstoß.
**Nächster Befehl:** `$impeccable adapt`.

## Positive und ausgeschlossene Befunde

- Keine unbenannten sichtbaren Buttons/Formularfelder in der zusätzlichen
  DOM-Prüfung der acht Einstellungsbereiche.
- Repräsentative Token-Textkontraste: weiß auf Primärblau **4,63:1**, gedämpfter
  Text auf erhöhter Fläche **5,11:1**, auf Standardfläche **6,50:1**, Warntext
  auf Fehlerfläche **8,57:1**. Keine Behauptung vollständiger Kontrastabdeckung.
- Kleine Desktop-Schaltflächen sind beabsichtigte Dichte. Ein pauschaler
  44-Pixel-Touchmaßstab wird nicht als Fehler der Desktop-App ausgegeben.
- Zwei Pixel `scrollWidth`-Differenz an der nativen Lautstärkeregelung sind
  ohne festgestellten Bedienverlust kein eigener Layoutbefund.
- Reduzierte Bewegung entfernt hier Farbtransitions; es gibt keine notwendige
  animierte Zustandsvermittlung, die dadurch verloren geht.
- Das JavaScript-Paket umfasst nach Build etwa **354 kB / 94 kB gzip**.
  Lizenztexte werden gemeinsam geladen. Ohne gemessene Startzeit-/Bedienprobleme
  wird daraus kein spekulativer Performancefehler gemacht.
- Der Hook meldet `DESIGN.md` neuer als `.impeccable/design.json`. Diese
  Metadatenabweichung wird gemeldet und nicht nebenbei repariert.

## Wiederverwendung, Dateien und Verträge

Geändert: `App.svelte`, `Settings.svelte` und die Komponenten `AboutPanel`,
`AccountSetup`, `CallPanel`, `ContactsPanel`, `ContactsSettings`, `DataSettings`,
`GeneralSettings`, `GettingStarted`, `HistoryPanel`, `SoundPanel`; außerdem die
vier vorhandenen Browserflows `contacts.mjs`, `history.mjs`, `phone.mjs`,
`settings.mjs` und dieser Bericht.

Vorhandene Statusverzweigungen, Komponenten, Validierung, Freigabe-APIs,
Löschbestätigungen und Browser-Fixtures weiterverwendet. Keine neuen Dependencies.
API-Befehle, DTOs, Validierungsregeln und native SIP-/Audioimplementierung werden
durch diesen Auftrag nicht verändert; daher keine API-Dokumentationsänderung.
Bereits vor Arbeitsbeginn vorhandene SIP-/Build-/Dokumentationsänderungen bleiben
erhalten, auch die bestehende zusätzliche Fehlermeldung in `CallPanel.svelte`.

Terminologie: **SIP-Konto** bezeichnet die Identität, **Anmeldung** deren
Registrierung, **Anruf** den Versuch, **Gespräch** die bestehende Verbindung,
**Verlauf** die gespeicherten Anrufdetails. **Audioaufzeichnung** bleibt dem
expliziten Entwicklungsmodus vorbehalten.

## Verifikation

| Prüfung                                                          | Beobachtetes Ergebnis                                                                                               |
| ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `bun run format:check`                                           | Bestanden; 268 Dateien formatiert, Rustfmt ohne Abweichung.                                                         |
| `bun run check`                                                  | Bestanden; Svelte 0 Fehler / 0 Warnungen, TypeScript ohne Fehler.                                                   |
| `bun run test:ui`                                                | 88 bestanden, 0 fehlgeschlagen.                                                                                     |
| `SIP_UI_CAPTURE=1 bun run test:ui:flows`                         | Alle sieben Flows bestanden: contacts, history, phone, call-controls, dialer-identity, settings, layout.            |
| Zusätzliche Audit-Browserprüfung                                 | Acht Einstellungsbereiche in beiden regulären Größen und mit CSS-Zoom 200 %, beide Fehlerreproduktionen beobachtet. |
| `bun run build`                                                  | Bestanden.                                                                                                          |
| `bun run check:versions`                                         | Bestanden mit Rustup-Pfad vor Homebrew.                                                                             |
| `bun run types:generate`                                         | Bestanden; generierte `AppInfo.ts` danach unverändert.                                                              |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Bestanden mit gepinnter Rust-Version 1.98.1.                                                                        |
| `cargo test --locked --workspace`                                | 120 bestanden; 1 expliziter OS-Schlüsselbundtest ignoriert. Portable Prüfung ohne Native-Feature.                   |
| `bun run desktop:build`                                          | Bestanden für macOS aarch64 portable; `.app`, `.dmg`, SBOM und Hashnachweise erzeugt.                               |
| Impeccable `detect src --json`                                   | 0 mechanische Befunde, vor und nach den Textänderungen.                                                             |
| `git diff --check`                                               | Bestanden.                                                                                                          |

Die erste Versionsprüfung scheiterte an Homebrew Rust 1.99.1 vor der gepinnten
Rustup-Version. Der erste Desktop-Build scheiterte zusätzlich mit lokalem
`libLLVM.dylib`-/Ressourcenfehler. Der erfolgreiche Wiederholungsaufruf verwendete
`PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_BUILD_JOBS=2` und
`DYLD_LIBRARY_PATH=/Users/lucaschoeneberg/.rustup/toolchains/1.98.1-aarch64-apple-darwin/lib`.
Keine Toolchain-/Projektkonfiguration dafür verändert. Cargo zeigte dabei noch
zwischengespeicherte Strip-Warnungen; der gesamte Build einschließlich Paketierung
und Export endete mit Exitcode 0. Signierung lokal ad hoc, keine Notarisierung.

Die erste UI-Testwiederholung traf auf eine zu breit formulierte vorhandene
Textprüfung; der betreffende Hilfetext wurde eindeutiger formuliert. Der
Lösch-Browsercheck wurde an die neue Bestätigungsgruppe und deren Text angepasst.
Die oben genannten erfolgreichen Ergebnisse stammen aus den anschließenden Läufen.

Browser-Fixtures bestätigen Darstellung und Interaktion; sie bestätigen weder
SIP-Registrierung noch Audio, OS-Dialoge oder Windows-Laufzeit. Keine erneuten
Asterisk-/Hardwaretests, da dieser Patch nur UX-Texte und ihre vorhandenen
Selektoren ändert. Keine Aussage über gehostete CI oder native Runtime-Tests.

## Empfohlene Reihenfolge

1. **[P2] `$impeccable harden`:** A01/A02 in Datenschutz beheben.
2. **[P2] `$impeccable adapt`:** A03 unter starker Vergrößerung beheben und nativ prüfen.
3. **`$impeccable audit`:** die drei konkreten Reproduktionen nachprüfen.
4. **`$impeccable polish`:** abschließender gemeinsamer Darstellungscheck.

Diese Folgeaufträge können einzeln oder zusammen ausgeführt werden. Nach den
Korrekturen sollte der Audit mit denselben Reproduktionen wiederholt werden.
