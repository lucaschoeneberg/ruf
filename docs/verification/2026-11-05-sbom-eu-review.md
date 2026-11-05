# Inhaltliche Prüfung der SBOM gegen den Cyber Resilience Act

Nachtrag: Die hier dokumentierten ursprünglichen Befunde wurden anschließend
technisch bearbeitet; siehe [Umsetzung und Build-Nachweise](2026-11-05-sbom-build-fix.md).
Die folgenden Artefakt-Hashes beschreiben ausdrücklich den Zustand vor der Korrektur.

Prüfstand: 5. November 2026. Ergebnis: **Die vorhandene Datei ist formal gültiges
CycloneDX 1.6, aber als produktbezogener CRA-SBOM-Nachweis noch nicht ausreichend.**
Dies korrigiert keine Software; es dokumentiert die Prüfung der bestehenden
Implementierung und der lokal vorhandenen Artefakte.

## Maßstab und Grenzen

Der [CRA, Verordnung (EU) 2024/2847](https://eur-lex.europa.eu/eli/reg/2024/2847/oj),
fordert in Anhang I Teil II Nr. 1 ein gebräuchliches maschinenlesbares Format,
das mindestens die direkten Produktabhängigkeiten abdeckt. Die Definition in
Art. 3 Nr. 39 umfasst Komponenteninformationen und deren Lieferkettenbeziehungen.
Anhang II Nr. 9 behandelt den Zugang, wenn der Hersteller die SBOM Nutzern
bereitstellt; eine vollständige Paketliste auf der Über-Seite ist damit nicht
generell vorgeschrieben.

Die wesentlichen Pflichten gelten nach dem
[Zeitplan der EU-Kommission](https://digital-strategy.ec.europa.eu/en/policies/cyber-resilience-act)
ab 11. Dezember 2027; Meldepflichten bereits ab 11. September 2026.
Diese Bewertung ist deshalb keine Feststellung eines heutigen Rechtsverstoßes.
Die konkrete Anwendbarkeit auf Ruf muss anhand des Vertriebsmodells eingeordnet
werden. Eine MIT-Lizenz allein beantwortet das nicht; siehe die
[Kommission zu Open Source und kommerzieller Bereitstellung](https://digital-strategy.ec.europa.eu/en/policies/cra-open-source).

Geprüft wurden der SBOM-Inhalt und seine technische Nachweisfähigkeit. Eine
Gesamtbewertung von Risikomanagement, Schwachstellenbehandlung, CE-Verfahren
oder allen eventuell anwendbaren Normen/Durchführungsrechtsakten erfolgte nicht.
Optionale CycloneDX-Felder werden hier nicht pauschal zu EU-Pflichtfeldern erklärt.

## Geprüfte Eingaben und Artefakte

- Generator: `apps/desktop/build/software-bill.ts` und dessen TypeScript-Modell.
- Aktuelle Lockfiles, Cargo-Manifeste, `native/sources.json`, SDK-Manifest und
  native Linkanweisungen in `crates/baresip-adapter/build.rs`.
- Erzeugte Datei: `apps/desktop/dist/software-bill.cdx.json`. Ihr Inhalt stimmt
  mit einem frischen Aufruf des Generators überein.
- Vorhandenes macOS-Binary:
  `target/release/bundle/macos/Ruf.app/Contents/MacOS/sip-desktop`.
- Desktop-/Release-Workflows und die SBOM-Anzeige in `AboutPanel.svelte`.

SHA-256 der geprüften SBOM:
`d12cbd682068beae6ec7cc2ab11eb3be4ae18b779b96c71cb945e29397f5c9bc`.

SHA-256 des geprüften App-Binary:
`5a915cb17d017b9b258cc1d47852253e4de91866e265aea163bd24c17d3bdc72`.

## Ergebnisse

| Prüfpunkt                                  | Befund                                                                                                                                                                                                                                              | Bewertung                                                                                                                                                |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Maschinenlesbares Format                   | Das JSON besteht die offizielle CycloneDX-1.6-Schemaprüfung.                                                                                                                                                                                        | Erfüllt auf Formatebene; keine Aussage über Vollständigkeit.                                                                                             |
| Namen und Versionen                        | 587 eindeutige Paket-URLs: 501 Cargo-Pakete, 84 npm-Pakete und zwei native Quellstände.                                                                                                                                                             | Als Projektinventar nützlich.                                                                                                                            |
| Direkte und aufgelöste Rust-Abhängigkeiten | Die externen direkten Rust-Abhängigkeiten des macOS-Desktop-Pakets sind vorhanden. Im normalen Cargo-Abhängigkeitsgraphen sind 255 externe Pakete erreichbar, mit Build-Abhängigkeiten 272; keines davon fehlt in der Datei.                        | Rust-Abdeckung positiv, aber kein Beweis über die gesamte Anwendung. Eigene Workspace-Crates sind unter dem App-Code zusammengefasst.                    |
| Native Abhängigkeiten                      | OpenSSL fehlt vollständig im JSON. Das SDK nennt 3.5.9. Das vorhandene Binary enthält `_OPENSSL_init_ssl`, `_baresip_init`, `_ua_alloc` und die Versionszeichenfolge `OpenSSL 3.5.9 29 Sep 2026`. `build.rs` linkt `ssl` und `crypto` statisch.     | **Inhaltliche Lücke:** Eine tatsächlich eingebundene native Softwarekomponente fehlt.                                                                    |
| Beziehungen                                | Es gibt weder `dependencies` noch Komponentenreferenzen, die App, direkte Abhängigkeiten und deren Unterabhängigkeiten verbinden.                                                                                                                   | **Inhaltliche Lücke:** Die flache Liste bildet die geforderten Lieferkettenbeziehungen nicht ab.                                                         |
| Bezug zur ausgelieferten Variante          | Der Generator liest weder Zielplattform noch Cargo-Features oder den tatsächlichen SDK-Stand. Native Quellstände erscheinen auch bei portablen Builds; alle Lockfile-Plattformvarianten und Build-Werkzeuge werden ohne Klassifikation aufgenommen. | **Nachweislücke:** Kein belastbarer Bezug zu einem bestimmten ausgelieferten Produktbuild.                                                               |
| Lokale Änderungen an Fremdcode             | baresip/libre werden mit den upstream Versions-/Commitangaben erfasst. Die tatsächlich angewendeten lokalen Patches stehen im SDK-Manifest, aber nicht in der SBOM.                                                                                 | Herkunft und Änderungen sind im Inventar nicht vollständig nachvollziehbar.                                                                              |
| Zuordnung und Wiederauffindbarkeit         | Publisher und App-Version sind vorhanden. Erstellungszeitpunkt, Generator, Zielplattform, Feature-Auswahl und Artefakt-Hash fehlen. Die Release-Jobs sichern nur Installer als eigene Assets.                                                       | Zusätzliche Nachweis-/Archivierungslücke; nicht jedes einzelne Feld ist dadurch automatisch eine gesetzliche Pflicht.                                    |
| Ergänzende Informationen                   | Kein Komponenteneintrag enthält Lieferant, Lizenz oder Hash. Schriftdateien und eingebettete Quellen sind nicht vollständig inventarisiert.                                                                                                         | Weitere Qualitäts- und Provenienzlücken. Die Lizenzdateien außerhalb der SBOM ersetzen diese Angaben nicht. Keine pauschale CRA-Einzelpflichtbehauptung. |

Die Cargo-Zahlen stammen aus einem auf `aarch64-apple-darwin` gefilterten
Metadatengraphen mit `sip-desktop/native`. Sie sind keine exakte Zählung der
letztlich im Linker verbleibenden Bibliotheken. Auch eine npm-`devDependency`
darf nicht automatisch ausgeschlossen werden: beispielsweise liefert Svelte
Code für die kompilierte Oberfläche. Für JavaScript muss der tatsächliche
Bundle-Inhalt berücksichtigt werden.

Die 587 Einträge belegen somit weder 587 installierte Laufzeitbibliotheken noch
die Vollständigkeit gegenüber dem Binary. Das SDK-Feld
`licenseInventoryComplete: false` betrifft ausdrücklich die Lizenzinventur und
wurde nicht als eigener Beweis für eine SBOM-Lücke gewertet.

## Erforderliche Korrekturen für einen belastbaren Nachweis

1. Die Produktvariante festlegen: App-Version, Zielplattform und aktivierte
   native Features. Ein Projektinventar darf zusätzlich bestehen bleiben,
   sollte aber von der produktbezogenen SBOM unterscheidbar sein.
2. Die ausgewählten Rust-Pakete und deren Beziehungen aus dem aufgelösten
   Cargo-Graphen ableiten; den JavaScript-Anteil anhand des Bundles ermitteln.
3. Die nativen Komponenten aus dem tatsächlich verwendeten SDK ergänzen,
   insbesondere OpenSSL mit genauer Version und den lokalen Patchständen.
4. Eindeutige Referenzen und Abhängigkeitsbeziehungen vom App-Eintrag über
   direkte bis zu erfassten indirekten Komponenten erzeugen. Unbekannte
   Vollständigkeit kenntlich machen, statt sie durch eine hohe Anzahl zu ersetzen.
5. SBOM und Installer gemeinsam identifizierbar archivieren; Generator,
   Erstellungszeitpunkt und Artefakt-Zuordnung dokumentieren. Lizenz-, Lieferanten-
   und Hashdaten aus verifizierten Metadaten ergänzen, soweit verfügbar.
6. Inhaltliche Prüfungen hinzufügen: native Builds ohne OpenSSL-Eintrag müssen
   fehlschlagen; Produktwurzel, direkte Abhängigkeiten, Beziehungen und gewählte
   Plattform müssen nachvollziehbar sein. Die Schemaprüfung allein genügt nicht.

Die kompakte Oberfläche kann dabei bleiben. Die acht sichtbaren Bausteine sind
eine Übersicht; der eigentliche Nachweis ist die vollständige, zum jeweiligen
Produktbuild passende maschinenlesbare Datei. Die aktuelle Beschriftung
„Vollständige SBOM“ ist durch die festgestellte Lücke inhaltlich noch nicht gedeckt.

## Tatsächlich ausgeführte Prüfungen

- Frischer Generatorlauf und Inhaltsvergleich mit der vorhandenen Build-Datei.
- Validierung des aktuellen JSON gegen das
  [offizielle CycloneDX-1.6-Schema](https://cyclonedx.org/schema/bom-1.6.schema.json)
  mit `jsonschema` in einer isolierten `uv`-Umgebung: bestanden.
- `cargo metadata --locked --offline --format-version 1 --filter-platform aarch64-apple-darwin --features sip-desktop/native`;
  danach Abgleich direkter und transitiver Paketidentitäten gegen die SBOM.
- `bun test tests/ui/software-bill.test.ts`: ein Test bestanden. Dieser Test
  kontrolliert ausgewählte Paketversionen, nicht die komplette CRA-Abdeckung.
- `nm -gU`, `strings -a`, `otool -L` und SHA-256-Prüfsummen am vorhandenen Binary.
  Statische Bibliotheken wurden nicht allein anhand von `otool -L` bewertet.
- Quellprüfung des Generators, SDK-Manifests, Linkschritts und der Workflows.

Kein neuer Build, keine SIP-/Audiotests, kein gehosteter CI-Lauf und keine
Windows-Binärprüfung wurden für dieses Review ausgeführt. Produktcode, UI,
Abhängigkeiten, APIs und DTOs wurden nicht verändert.
