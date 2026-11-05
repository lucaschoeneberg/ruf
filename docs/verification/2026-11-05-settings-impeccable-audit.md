# Impeccable-Audit: gesamte Einstellungen · 2026-11-05

> **Nachprüfung:** A01–A06 wurden in der freigegebenen Reihenfolge behoben.
> [Umsetzung, Regressionen und Prüfnachweis](2026-11-05-settings-audit-fixes.md).
> Der folgende Bericht dokumentiert den ursprünglichen Auditstand.

## Implementation Integrity: bestanden, mit Korrekturbedarf

Die Einstellungen bilden ein erkennbares Ruf-Desktop-System: acht Bereiche,
permanente Kontenliste, gemeinsame Zed-Palette, IBM Plex Sans, flache Zeilen,
explizite Speicheraktionen und getrennte App-/Systemberechtigungen. Es gibt keine
beliebigen Dashboard-Bausteine oder erfundenen Daten in der Produktoberfläche.
Der Impeccable-Detektor meldet für die geprüften Quellen **keine Befunde**.

Die manuelle Code- und Browserprüfung findet trotzdem konkrete Zustands-, Such-
und Randfallprobleme. Ein leerer Detektorbericht ist keine Bestätigung vollständiger
Barrierefreiheit oder Fehlerfreiheit.

## Ergebnis

| Dimension                  | Punkte    | Wesentlicher Befund                                                                                                     |
| -------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------- |
| Barrierefreiheit           | 3/4       | Versionssuche verliert Fokus; Kontoaktionen haben keinen Kontonamen im zugänglichen Namen.                              |
| Performance                | 3/4       | Begrenzte Aktualisierungsraten und keine auffälligen Layoutschleifen; gemeinsames, nicht aufgeteiltes JavaScript-Paket. |
| Responsive Darstellung     | 3/4       | Normale Inhalte passen in beide Fenstergrößen; gültiger langer Kontoname überläuft.                                     |
| Theming                    | 4/4       | Gemeinsame semantische Farbvariablen, lokale Schrift und konsistente dunkle Darstellung.                                |
| Implementierungsintegrität | 2/4       | Berechtigungsanzeige vermischt Fehlerarten; Suchindex und vorhandene Ziele sind unvollständig gekoppelt.                |
| **Gesamt**                 | **15/20** | **Gut; gezielte Korrekturen erforderlich.**                                                                             |

**Sechs Befunde: 0 × P0, 0 × P1, 5 × P2, 1 × P3.** Die Einstufung beschreibt
Nutzerwirkung im geprüften Desktop-Kontext, keine formale WCAG-Zertifizierung.
Priorität haben die irreführende Mitteilungsdiagnose, überlaufende Kontonamen und
die fehlerhafte Suchnavigation.

## Umfang und Methode

- Impeccable 4.3.1, expliziter Befehl `audit`; Kontext über den Plugin-Launcher
  einmal geladen. Die tatsächliche Oberfläche ist Svelte in einer Tauri-WebView.
  Deshalb gelten die Web-Prüfungen im Desktop-Nutzungskontext. Die nativen
  iOS-/Android-Prüfkataloge sind hierfür nicht passend.
- Quellen: `Settings.svelte`, alle eingebundenen Einstellungskomponenten,
  `settings-navigation.ts`, `main.ts` und gemeinsame CSS-Regeln.
- Frische Browseraufnahmen aller acht Bereiche bei **900 × 660** und
  **740 × 540**; zusätzlich untere Seitenbereiche und gezielte Randfälle. Zwei
  begrenzte Aufnahmegruppen, keine iterative Produktüberarbeitung.
- Ausschließlich vorhandene synthetische IPC-Fixtures aus
  `tests/ui/browser/settings.mjs`; keine persönlichen Konten/Kontakte,
  echten SIP-Anrufe, OS-Freigabedialoge oder Systemmitteilungen. Externe
  Changelog-Anfragen wurden im Browser mit festem Audittext beantwortet.
- DOM-Messungen zu Breiten, Beschriftungen, Überschriften und Textkontrast;
  native Bedienelemente, Transparenz-/Überlagerungsfälle und Screenreader-Ausgabe
  sind damit nicht vollständig automatisch bewertet.
- Frisch ausgeführt: `bun run check` (**0 Fehler, 0 Warnungen**),
  `bun run test:ui` (**61 bestanden**) und
  `SIP_UI_FLOW=settings bun run test:ui:flows` (**bestanden**).
- Impeccable `detect --json` über Einstellungsquellen, Komponenten und CSS:
  **0 mechanische Befunde**. Die Empfehlungen unten stammen aus zusätzlich
  geprüften Code- und Browserbefunden.

## Detaillierte Befunde

### A01 · [P2] Fehler einer Testmitteilung verfälscht den Freigabestatus

**Ort:** `apps/desktop/src/lib/components/NotificationSettings.svelte:102`,
`:87`, `:177`; gemeinsame Darstellung in `PermissionStatus.svelte:47`.
**Kategorie:** Implementierungsintegrität / Zustandskommunikation.

**Reproduktion:** Mitteilungsfreigabe ist `authorized`. Das synthetische Senden
von `test_notification` wird abgelehnt. Anschließend zeigt die Systemfreigabe
„Prüfung fehlgeschlagen“, während derselbe Bildschirm darunter weiterhin
„Mitteilungen sind erlaubt“ erklärt. Der gemessene Backend-Freigabewert bleibt
`authorized`. Auch ein Fehler beim Öffnen der Systemeinstellungen nutzt dieselbe
Fehlervariable.

**Auswirkung:** Nutzer werden bei einem Zustellungs-/Öffnungsfehler zu einer
falschen Berechtigungsdiagnose geführt. Gerade die Fehlersuche bei ausbleibenden
Mitteilungen wird dadurch unzuverlässig.

**Empfehlung:** Fehler der Freigabeabfrage, des Öffnens und des Testversands
separat führen. Ausschließlich ein fehlgeschlagener Freigabe-Read darf den
Freigabestatus als unbekannt/fehlgeschlagen darstellen. Die native Anfrage darf
weiterhin unabhängig von App-Einstellungsentwürfen bleiben.
**Standard:** Wahrheitsgetreue Zustandskommunikation; keine zusätzliche formale
WCAG-Verletzung aus diesem Test abgeleitet.
**Befehl:** `$impeccable harden`.

Beleg: `output/playwright/impeccable-audit-notification-failure.png`.

### A02 · [P2] Versionssuche hat kein erreichbares Fokusziel

**Ort:** `apps/desktop/src/lib/settings-navigation.ts:89`,
`apps/desktop/src/lib/components/AboutPanel.svelte:136`,
`apps/desktop/src/Settings.svelte:301`.
**Kategorie:** Barrierefreiheit / Navigation.

**Reproduktion:** „Version“ suchen und „Version und Software · Über Ruf“
auswählen. Der Suchindex fordert `about-heading` an. Im Panel existiert nur
`class="about-heading"`, keine passende ID. Nach dem Entfernen des Suchtreffers
liegt der gemessene Fokus auf `body`, nicht auf dem Ziel.

**Auswirkung:** Tastaturnutzer verlieren ihre Position; die versprochene
Navigation zum gefundenen Inhalt bleibt aus.

**Empfehlung:** Ein bestehendes, semantisch passendes Ziel mit der registrierten
ID und programmatischer Fokussierbarkeit versehen. Alle Suchziele in einem
Browser-Vertragstest auf Existenz und tatsächlich gesetzten Fokus prüfen.
**Standardbezug:** [WCAG 2.4.3 – Fokusreihenfolge](https://www.w3.org/WAI/WCAG22/Understanding/focus-order.html).
Dieser Befund ist ein verifizierter Fokusverlust, keine vollständige
Konformitätsbewertung der Anwendung.
**Befehl:** `$impeccable harden`.

### A03 · [P2] Sichtbare Einstellungsnamen fehlen im Suchindex

**Ort:** `apps/desktop/src/lib/settings-navigation.ts:18` und `:95`.
**Kategorie:** Implementierungsintegrität / Auffindbarkeit.

**Gemessene Beispiele:**

| Eingabe             | Tatsächliches Ergebnis                 | Erwartetes Ziel       |
| ------------------- | -------------------------------------- | --------------------- |
| `Anruferdetails`    | Kein Treffer                           | `notification-caller` |
| `Klingellautstärke` | Kein Treffer                           | `ringtone-volume`     |
| `Firma anzeigen`    | Kein Treffer                           | `show-organization`   |
| `Aufbewahrung`      | „Verlauf speichern“                    | `history-retention`   |
| `Passwort`          | „SIP-Konto bearbeiten“, Fokus auf Name | `password`            |

**Auswirkung:** Nutzer können vorhandene Einstellungen über deren sichtbare
Bezeichnung nicht finden oder landen beim falschen Feld. Die Feldsuche arbeitet
an mehreren Stellen lediglich wie eine grobe Bereichssuche.

**Empfehlung:** Vorhandene konkrete Feld-IDs und sichtbare deutsche Beschriftungen
im gemeinsamen Register abbilden; zusätzlich Synonyme beibehalten. Für jedes
sichtbare Suchfeld mindestens Beschriftung, Treffer und passendes DOM-Ziel prüfen.
Keine Kontoinhalte oder Passwörter in den statischen Feldindex aufnehmen.
**Standard:** Produktvertrag der Feldsuche; kein eigenständiger WCAG-Verstoß
behauptet.
**Befehl:** `$impeccable harden`.

### A04 · [P2] Gültiger langer Kontoname überlagert die Kontoaktionen

**Ort:** `apps/desktop/src/Settings.svelte:961`,
`apps/desktop/src/app.css:1004`.
**Kategorie:** Responsive Darstellung.

**Reproduktion:** Erstes von acht synthetischen Konten mit 64 zusammenhängenden
`W` benennen; das liegt innerhalb der vorhandenen Namensvalidierung. Bei
740 × 540 misst der Inhaltsbereich **540 px**, seine horizontale Inhaltsbreite
jedoch **877 px**. Die Überschrift beansprucht **853 px** innerhalb einer
**282 px** breiten Beschreibungsspalte. Sie überlagert „Als Standard“ und läuft
hinter die Aktionsschaltflächen. Der äußere Dokument-Überlauftest bleibt dabei
fälschlich unauffällig.

**Auswirkung:** Kontoidentität und Aktionen sind schlechter lesbar; im
scrollenden Einstellungsbereich entsteht zusätzliche horizontale Navigation.

**Empfehlung:** Das bestehende Umbruchverhalten der Detailüberschrift auch auf
Kontonamen in der Übersicht anwenden. Innenbereiche und überlagernde Aktionen
mit zulässigen langen Namen testen, nicht ausschließlich
`document.documentElement.scrollWidth`.
**Standardbezug:** Reflow-Grundsatz aus
[WCAG 1.4.10](https://www.w3.org/WAI/WCAG22/Understanding/reflow.html).
Der Test bei 740 px ist eine Desktop-Randfallprüfung und ersetzt keinen formalen
WCAG-Reflow-Test bei 320 CSS-Pixeln.
**Befehl:** `$impeccable adapt`.

Beleg: `output/playwright/impeccable-audit-long-account-740.png`.

### A05 · [P2] Wiederholte Kontoaktionen haben keinen programmatischen Kontobezug

**Ort:** `apps/desktop/src/Settings.svelte:958`, `:981`, `:991`.
**Kategorie:** Barrierefreiheit.

**Nachweis:** Bei acht Konten entstehen acht Schaltflächen „Deaktivieren“ und
acht Schaltflächen „Als Standard“. Die Aktionsnamen enthalten keinen Kontonamen;
es gibt auch keine Verknüpfung per `aria-labelledby`/`aria-describedby` zu einer
benannten Kontogruppe. Die Überschrift steht visuell in einer benachbarten
Beschreibung. Das `aria-label` auf dem äußeren generischen Übersichts-`div`
stellt diesen Bezug nicht pro Konto her.

**Auswirkung:** In einer isolierten Schaltflächenliste oder beim Durchtabben ist
schwerer erkennbar, welches Konto eine Aktion verändert. Die rein visuelle
Zuordnung ist für assistive Bedienung nicht gleichwertig verfügbar.

**Empfehlung:** Bestehende Zeilen mit dem jeweiligen Kontonamen semantisch
verbinden und Aktionen eindeutig benennen, beispielsweise
„Deaktivieren · Büro“. Sichtbare Aktionswörter im zugänglichen Namen erhalten;
keine zusätzlichen sichtbaren Identitätsdaten hinzufügen.
**Standardbezug:** [WCAG 2.4.6 – Überschriften und Beschriftungen](https://www.w3.org/WAI/WCAG22/Understanding/headings-and-labels.html).
DOM-Befund; tatsächliche VoiceOver-/Windows-Screenreader-Ausgabe wurde nicht getestet.
**Befehl:** `$impeccable harden`.

### A06 · [P3] Kontenbereich beginnt mit Unterüberschriften vor der Hauptüberschrift

**Ort:** `apps/desktop/src/Settings.svelte:954`,
`apps/desktop/src/lib/components/AccountSetup.svelte:88`.
**Kategorie:** Barrierefreiheit / semantische Struktur.

**Nachweis:** Die Reihenfolge lautet `h2 SIP-Konten`, mehrere `h3 Kontoname`,
anschließend `h1 Kontoname`, dann `h2 Kontoinformationen`. Die sichtbare
Seitenüberschrift ist damit semantisch untergeordnet, während das spätere
Kontodetail die Hauptüberschrift bildet.

**Auswirkung:** Die Navigation über eine Überschriftenliste beschreibt die
sichtbare Hierarchie nur unzureichend.

**Empfehlung:** Eine Hauptüberschrift für den Bereich, darunter sinnvoll
geordnete Übersicht und Kontodetail. Die vorhandene AccountSetup-Komponente
für ihren eingebetteten Kontext wiederverwenden; keinen zweiten Formularpfad
anlegen.
**Standardbezug:** Struktur-/Orientierungsempfehlung; eine bloß übersprungene
Überschriftenebene wird hier nicht pauschal als WCAG-AA-Verletzung gewertet.
**Befehl:** `$impeccable polish`.

## Wiederkehrende Muster

1. **Unvollständige Verbindung zwischen Register und gerendertem Feld:** Suchindex,
   Ziel-IDs und sichtbare Beschriftungen werden getrennt gepflegt. Dadurch bleiben
   fehlende Ziele und unvollständige Suchbegriffe trotz grüner Routentests unbemerkt.
2. **Gemeinsame Fehlervariable für unterschiedliche Zustände:** Der
   Mitteilungstest kann die Berechtigungsdarstellung verändern. Der Fehlerbereich
   muss zur Operation gehören, deren Zustand er beschreibt.
3. **Tests mit normalen Inhalten verdecken innere Layoutprobleme:** Der
   Dokument-Überlauf bleibt selbst beim belegten Überlauf einer Kontozeile negativ.
4. **Visuelle Gruppierung ist stärker als semantische Gruppierung:** Kontozeilen
   sind optisch klar; Kontext und Überschriftenstruktur müssen assistiv ebenso
   eindeutig verfügbar sein.

## Positive Befunde und verworfene Fehlalarme

- Alle acht Standardseiten passen bei beiden geprüften Fenstergrößen ohne
  horizontalen Dokument- oder Inhaltsüberlauf. Das Kontenblock-Konzept bleibt
  erhalten und entspricht der gewünschten direkten Kontonavigation.
- Keine fehlenden Beschriftungen der geprüften sichtbaren Formularfelder.
  Statusinformationen benutzen vorhandene Status-/Alarmrollen.
- In den automatisch ausgewerteten sichtbaren Text-/Hintergrundkombinationen
  kein Textkontrast unter dem jeweiligen Prüfschwellwert; deaktivierte Elemente
  wurden nicht fälschlich als Kontrastverletzung gezählt. Native Steuerungsfarben
  und sämtliche Transparenzfälle sind damit nicht zertifiziert.
- Gemeinsame Farbvariablen und lokale 400/600-Schriftdateien; keine neue Palette
  oder willkürliche Dashboard-Gestaltung.
- Berechtigungen werden beim Öffnen gelesen, ohne automatische `request_*`-Aufrufe.
  Das beobachtete Fehlversandproblem betrifft die Anzeige, nicht eine heimliche
  Freigabeanfrage.
- Bestehende Browserregressionen zu Entwürfen, Fehlerwiederholung,
  Kontenaktivierung, Audio-Eigentümerschaft und expliziten Aktionen bestanden.
- Das globale Reduced-Motion-Verhalten entfernt die vorhandenen kurzen
  Farbtransitionen. Statuswechsel bleiben durch Text und Zustand erkennbar;
  daraus wird hier kein Bewegungs-/Feedbackverlust konstruiert.
- Die kompakten Desktop-Steuerungen werden nicht pauschal an mobilen
  44-/48-Pixel-Vorgaben gemessen. Kleine Checkbox-Glyphen sind mit ihren
  beschrifteten und getrennten Zeilen zu bewerten, nicht isoliert als
  angeblicher WCAG-Verstoß.

## Performance-Beobachtung

Der vorhandene letzte Build enthält ein gemeinsames JavaScript-Paket von
**324.728 Bytes** und CSS von **32.457 Bytes**. `main.ts:3–4` importiert Telefon
und Einstellungen statisch; About lädt Lizenztexte und das Startinventar bereits
als Imports, einschließlich `import.meta.glob(..., eager: true)`. Eine spätere
Aufteilung könnte unnötige Anfangsarbeit reduzieren. Mangels gemessener
Startzeit-/Frame-Budget-Verletzung ist das **keiner der sechs Fehlerbefunde**.
Vor einer Optimierung sollten tatsächliche WebView-Startzeiten und Kosten des
Inventars gemessen werden. Aktualisierungen sind begrenzt: übergeordneter
Snapshot-Fallback alle zwei Sekunden, Pegelabfrage nur während eines Tests.

## Kontextdrift, getrennt von Produktfehlern

`apps/desktop/PRODUCT.md:56` bezeichnet Lautstärke/Mikrofonpegel noch als nicht
enthalten. Auch Teile von DESIGN.md beschreiben den älteren Stand der Suche und
Über-Seite. Die implementierten Einstellungen und der freigegebene Umbauplan
sind hier die aktuellere Wahrheit. Diese Dokumente wurden im Audit **nicht**
stillschweigend umgeschrieben; eine gezielte Aktualisierung gehört in einen
separaten `$impeccable document`-Schritt.

## Empfohlene Reihenfolge

1. **[P2] `$impeccable harden`:** Fehlerdomänen der Mitteilungen trennen;
   registrierte Suchziele, vollständige Feldsuche und eindeutig benannte
   Kontoaktionen mit gezielten Regressionen absichern.
2. **[P2] `$impeccable adapt`:** Zulässige lange Kontonamen ohne Überlagerung
   darstellen und den inneren Inhaltsüberlauf prüfen.
3. **[P3] `$impeccable polish`:** Überschriftenhierarchie des Kontenbereichs
   an die sichtbare Struktur angleichen; abschließende begrenzte Sichtkontrolle.

Diese Schritte lassen sich einzeln oder gemeinsam beauftragen. Nach Korrekturen
`$impeccable audit` erneut ausführen und die Bewertung aktualisieren.

## Artefakte, Änderungen und Grenzen

Nur dieser Auditbericht wurde im Projekt neu angelegt. Keine Produktdateien,
APIs, DTOs, Abhängigkeiten oder Einstellungen wurden geändert. Vorhandene
Svelte-Komponenten, CSS-Tokens, typed IPC-Fixtures und der gepinnte
Playwright-CLI-Runner wurden wiederverwendet. API-Dokumentation bleibt unverändert.

Aufnahmen liegen im ignorierten `output/playwright/impeccable-audit-*`.
Detektor-, Browser-, Randfall- und Suchdaten liegen in privaten taskbezogenen
Dateien unter `/tmp/ruf-impeccable-settings-*`. Die synthetischen Bezeichnungen
in den Bildern sind Audit-Fixtures.

Nicht geprüft: tatsächliches VoiceOver, Windows-Screenreader, WebKit-spezifische
Darstellung, OS-Freigabedialoge, physische Audiogeräte, reale Mitteilungsbanner,
SIP-Verhalten oder Hosted CI. Der Audit ist eine technische Prüfung der
Einstellungsoberfläche und ergänzt die zuvor separat dokumentierten nativen Tests.
