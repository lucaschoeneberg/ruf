# SIP-Softphone: MVP-Entwurf

Stand: 4. November 2026. Status: MVP-Roadmap. Die Projektgrundlage wurde mit der anschließenden Nutzeranweisung zur Umsetzung freigegeben; Telefoniefunktionen bleiben spätere Meilensteine.

## Ausgangslage und Entscheidungen

Der Projektordner ist leer und noch kein Git-Repository. Es gibt keine vorhandenen Komponenten, Dependencies, DTOs oder Build-Konventionen. In dieser Planungsrunde wird ausschließlich dieses Dokument angelegt; keine Anwendung wird erzeugt und keine Dependency installiert.

Vom Nutzer vorgegeben: macOS und Windows, Rust, Tauri 2, baresip sowie MIT für den eigenen Code. Langfristig breite SIP-Kompatibilität und Verknüpfung mit Kontakten und Kalendern. Für das erste MVP ausdrücklich gewählt: **Telefonie, Kontakte und Verlauf zuerst; Kalender im folgenden Meilenstein**.

Die nachstehenden Detailentscheidungen sind Empfehlungen, keine bereits vom Nutzer bestätigten Anforderungen. Ein ausführbarer Implementierungsplan folgt nach Abstimmung dieses Entwurfs.

## Produktziel

Eine Person installiert die App, hinterlegt ihre SIP-Zugangsdaten, bleibt erreichbar, kann einen Kontakt oder eine Nummer anrufen und anschließend den Verlauf nachvollziehen. Mehrere Konten bleiben gleichzeitig registriert. Für jeden ausgehenden Anruf wird ein Konto ausgewählt; eingehende Anrufe sind eindeutig einem Konto zugeordnet.

Das erste MVP unterstützt ein Gespräch insgesamt, einschließlich eines gehaltenen Gesprächs. Ein zweiter eingehender Anruf wird mit SIP 486 Busy Here abgelehnt und entsprechend protokolliert. Es gibt kein automatisches Halten oder Beenden des bestehenden Gesprächs. Die Auswahl eines anderen Kontos verändert kein laufendes Gespräch.

## Vorgeschlagener Umfang

### Telefonie und Konten

- Konten hinzufügen, ändern, deaktivieren und löschen; Anzeigename, Registrar/Domain, SIP-Benutzer, abweichender Authentifizierungsbenutzer, Passwort, Transport und optionaler Outbound-Proxy.
- Ändern, Deaktivieren oder Löschen eines Kontos mit einem laufenden oder gehaltenen Gespräch wird mit einem fachlichen Fehler abgewiesen. Das Gespräch kann zuerst ausdrücklich beendet werden.
- Ein generisches SIP-Profil; wenige dokumentierte Vorgaben für die tatsächlich getesteten Anlagen. Erweiterte Felder sind eingeklappt.
- Registrierungszustand und verständliche Fehleranzeige; sichtbarer Unterschied zwischen Netzwerkfehler, Authentifizierungsfehler und fehlender Mikrofonfreigabe.
- Alle aktivierten Konten werden parallel registriert. Standardkonto für neue ausgehende Gespräche, zusätzlich explizite Kontowahl beim Wählen.
- Anrufen, Annehmen, Ablehnen, Auflegen, Klingelton, Stummschalten, Halten/Fortsetzen und DTMF über RFC 4733. SIP-INFO-DTMF nur als explizite Profiloption, wenn ein Ziel es benötigt.
- UDP, TCP und TLS als explizite Transportoptionen. Bei TLS werden Serverzertifikat und Hostname geprüft; kein stiller Wechsel auf unverschlüsselten Transport.
- G.711 A-law und mu-law zuerst. G.722 und Opus nach erfolgreichem plattformübergreifendem Build ergänzen, bevor eine Unterstützung zugesagt wird.
- Verschlüsselte Medien über SRTP mit einem nachgewiesenen SDES-Profil im MVP-Testumfang; Schlüsselaustausch nur über TLS. Ein Profil kann Verschlüsselung verlangen; ein Fehlschlag führt dann nicht zu unverschlüsseltem Weitertelefonieren. Kein pauschales Ende-zu-Ende-Verschlüsselungsversprechen.
- Automatische Neuregistrierung nach Standby und Netzwerkunterbrechung. Laufende Gespräche können bei Netzwechsel abbrechen; die App muss dies erkennen und neue Gespräche wieder ermöglichen.
- Mikrofon- und Ausgabegerät wählen; Gerätestatus und eindeutige Meldung beim Abziehen. Headset-Betrieb ist der erste Audio-Abnahmepfad. Freisprechen wird erst nach einem Echotest auf beiden Plattformen als unterstützt markiert; AEC-Modul und dessen Build/Lizenz werden im ersten Meilenstein ausgewählt.

### Kontakte

- Suche nach Namen und Telefonnummer; Wahl einer der vorhandenen Nummern; Namensauflösung bei eingehenden Anrufen.
- Ein gemeinsames Kontaktmodell mit Quellenkennung und stabiler Quellen-ID. Mehrere Telefonnummern pro Kontakt; Herkunft bleibt sichtbar. Keine Zusammenführung allein aufgrund gleicher Namen.
- macOS: native Kontakte über Contacts nach Systemfreigabe; Aktualisierung bei Änderungen und beim Wiederöffnen der App. Zunächst nur lesender Zugriff durch die Anwendung.
- Beide Plattformen: vCard-Dateiimport als gemeinsame Basis. Unterstütztes Importprofil: UTF-8, vCard 3.0 und 4.0, Name und TEL-Felder, mehrere Kontakte und Telefonnummern. Keine Fotos oder beliebigen Attachments. Defekte Einträge werden mit einer Zusammenfassung übersprungen; eine defekte Datei darf den bestehenden Bestand nicht entfernen.
- Wiederholter Import ersetzt die Einträge derselben Importquelle nach erfolgreichem Parsen; andere Quellen bleiben erhalten. Importdateien mit unbekannter Herkunft werden als neue Quelle behandelt.
- Windows-MVP: vCard-Import, ausdrücklich kein Versprechen universellen automatischen Zugriffs auf Outlook, Thunderbird oder alle lokalen Adressbücher. Native Windows-Kontakte und Microsoft Graph werden für die nächste Integrationsrunde untersucht.
- Ein vCard-Import ist eine Momentaufnahme, keine laufende Synchronisation. Diese Einschränkung wird in der Oberfläche genannt.
- Originalnummer und normalisierte Suchform getrennt halten. Interne Durchwahlen, Stern-/Raute-Codes und SIP-Adressen dürfen nicht durch eine erzwungene E.164-Konvertierung verändert werden. Eine Ländervorgabe wird für tatsächlich nationale externe Nummern verwendet.

### Verlauf und Desktop-Verhalten

- Lokaler Verlauf der von dieser App beobachteten Gespräche: Konto, Gegenstelle, Richtung, Beginn, Annahme, Ende und Ergebnis; Rückruf über das zugeordnete Konto.
- Ergebnisse unterscheiden: beendet, verpasst, abgelehnt, besetzt abgewiesen, abgebrochen, fehlgeschlagen und unterbrochen. Ein normales SIP CANCEL allein beweist keine Annahme auf einem anderen Gerät.
- Zeitpunkte als UTC speichern; Dauer aus tatsächlich beobachteter Verbindung bestimmen. Ein beim Absturz offener Eintrag wird beim Neustart als unterbrochen markiert; unbekannte Endzeit und Dauer werden nicht erfunden.
- Einträge und gesamter Verlauf löschbar. Kein Import von Anlagenhistorie im MVP.
- App kann im Tray weiterlaufen. Fensterschließen blendet aus; ein ausdrückliches Beenden beendet die Engine und meldet Konten ab. Dieses Verhalten wird beim ersten Schließen erklärt. Bei laufendem Gespräch bleibt das Fenster sichtbar und bietet Beenden oder Weiterlaufen als bewusste Wahl.
- Eingehende Anrufe werden in einem sichtbaren Fenster mit Klingelton angeboten; Systembenachrichtigungen ergänzen dies. Entzogene Benachrichtigungsrechte dürfen die Erreichbarkeit nicht verdecken.
- Minimale Oberfläche: Wählen/Suche, aktives Gespräch, Kontakte, Verlauf und kompakte Konto-/Audioeinstellungen. Tastaturbedienung und zugängliche Beschriftungen gehören zur Abnahme.

### Spätere Meilensteine

Kalender, CardDAV/CalDAV, Microsoft-365-Anmeldung, automatische Windows-Kontaktquellen, Anlagenhistorie, mehrere parallele Gespräche, Rückfrage und Vermitteln, Konferenzen, BLF/Präsenz, Voicemail-Anzeige, Aufzeichnung und Video folgen nach dem MVP. Das langfristige Integrationsziel bleibt erhalten.

## Architektur und betrachtete Alternativen

Die Stack-Auswahl Rust/Tauri/baresip wurde vom Nutzer vorgegeben. Frühere Alternativen waren ein Rust-Stack wie RVoIP mit zusätzlichem Qualifizierungsrisiko oder eine stärker selbst zusammengesetzte Lösung auf rsipstack. Sie werden für dieses MVP nicht weiterverfolgt.

Für baresip sind zwei Integrationsformen möglich:

1. **Bibliothek mit schmalem C-Adapter (Empfehlung):** direkte typisierte Steuerung, keine zusätzliche lokale Netzwerkschnittstelle, kontrollierte Übergabe der Anmeldedaten. Dafür Verantwortung für C-Speicher, Threading und native Builds; ein nativer Absturz betrifft die App.
2. **Separater baresip-Prozess:** bessere Absturzisolation, dafür Prozessüberwachung, sichere Konfigurationsübergabe und zusätzlicher Steuerkanal. Das vorhandene ctrl_tcp-Modul ist eine Option, die vor einer Entscheidung auf Funktionsumfang und lokale Absicherung geprüft werden müsste.

Der Entwurf empfiehlt Variante 1 entsprechend der bisherigen Stack-Empfehlung. Der erste technische Meilenstein muss ihre praktische Eignung auf beiden Betriebssystemen bestätigen. Ein Wechsel auf einen separaten Prozess wäre eine neu zu dokumentierende Architekturentscheidung.

### Verantwortlichkeiten

- Svelte/TypeScript/Vite: Darstellung, Formularzustand und Bedienung.
- Tauri: App-Lebenszyklus, Fenster/Tray, Commands, Events und Betriebssystemanbindungen.
- phone-core: fachliche Konto- und Gesprächszustände, Anwendungsaktionen und Verlauf. Keine Abhängigkeit von der Webview oder C-Zeigern.
- baresip-adapter: kleine C-Schnittstelle und sichere Rust-Hülle; Registrierung, Gesprächssteuerung und Übersetzung nativer Ereignisse.
- baresip: SIP, SDP, RTP/RTCP, Codec-Verarbeitung und Audiogeräte. Keine Audioframes über Tauri-Events; keine zusätzliche CPAL-Pipeline.
- SQLite/rusqlite: Konfiguration ohne Passwörter, Verlauf und Kontaktcache mit versionierten Migrationen. Ein dedizierter Datenbank-Worker verhindert blockierende Datenbankzugriffe in UI- und Engine-Threads.
- keyring: SIP-Passwörter im Betriebssystem-Schlüsselspeicher. Bei Fehlern klarer Fehlerzustand statt Speicherung in Klartextdateien. Das Passwort ist während der Eingabe kurzfristig im Formular und wird an Rust übergeben; es erscheint nicht in Antwort-DTOs, Datenbank oder Logs.

### Threading und Zustandsführung

Tauri behält seine Plattform-Ereignisschleife. baresip/libre erhält einen dedizierten OS-Thread. Alle baresip-Steueraufrufe und das Erzeugen/Zerstören nativer Handles erfolgen auf diesem Thread; threadübergreifende Aufträge nutzen libres Message-Queue und wecken dessen Ereignisschleife.

Callbacks kopieren benötigte Daten in besitzende Rust-Werte, bevor die Callback-Lebensdauer endet. Keine C-Zeiger gelangen in Frontend, Datenbank oder Commands. Rust-Panics dürfen die C-Grenze nicht überqueren. Start, Abbruch und geordnetes Herunterfahren werden explizit behandelt.

Eine serialisierte Rust-Verarbeitung ist die fachliche Quelle für Konten und Gespräche. Anfragen eines anderen Threads reservieren einen Gesprächsplatz atomar in dieser Verarbeitung, bevor die Engine angewiesen wird; so können zwei gleichzeitige Wahlversuche kein zweites Gespräch eröffnen. Eingehende Engine-Ereignisse werden dort ebenfalls verarbeitet.

Die Engine-Verbindung liefert angenommene Aufträge bzw. unmittelbare Fehler. Ein erfolgreich angenommener Wählauftrag bedeutet nicht, dass ein Gespräch verbunden wurde; das folgt erst aus einem späteren Engine-Ereignis. Ein begrenzter Auftragskanal kann Überlast explizit zurückmelden. Essenzielle Gesprächsereignisse dürfen nicht still verloren gehen; ein Verarbeitungsfehler wird als Zustand mit notwendiger Neusynchronisierung sichtbar.

Rust hält einen Snapshot mit monotoner Revision. Das Frontend abonniert Ereignisse vor dem initialen Snapshot, puffert diese und verarbeitet anschließend nur neuere Revisionen. Bei einer Lücke oder nach Wiederöffnen des Fensters wird erneut ein Snapshot geladen. Der Frontendzustand ist eine Projektion; kritische Aktionen werden in Rust validiert.

## Vorgeschlagene Projektstruktur

```text
Cargo.toml                       # Workspace mit den drei Rust-Paketen
apps/desktop/
  src/                           # Svelte-Oberfläche nach Funktionen gegliedert
  src/lib/generated/             # aus Rust erzeugte API-Typen
  src-tauri/
    src/commands/                # schmale Tauri-API
    src/platform/                # macOS-/Windows-Anbindungen
    src/storage/                 # SQLite und Schlüsselspeicher
    capabilities/                # freigegebene Tauri-Funktionen
crates/
  phone-core/src/                 # accounts, calls, contacts, history
  baresip-adapter/
    src/                         # sichere Rust-Hülle
    native/                      # kleiner C-Adapter
native/                          # reproduzierbare baresip/libre-Build-Rezepte
tests/interop/                   # Anlagenkonfigurationen und SIPp-Szenarien
docs/
  api/                           # Commands, DTOs, Ereignisse, Fehler
  compatibility/                 # exakte getestete Versionen und Profile
  superpowers/specs/             # dieser Entwurf
```

Zunächst drei Rust-Pakete, kein allgemeines Plugin-Framework und keine universelle Engine-Abstraktion. Die C-Anbindung ist isoliert, damit ihr unsicherer Teil unabhängig geprüft werden kann. Plattformmodule bleiben im Desktop-Paket, bis echte Wiederverwendung eine Auslagerung rechtfertigt.

## Schnittstellen und Dokumentationsweg

Die interne API besteht aus Tauri-Commands und Events, ohne eigenen HTTP-Server. Geplante Aktionen: Snapshot lesen; Konto anlegen/ändern/deaktivieren/löschen; Gespräch starten/annehmen/ablehnen/beenden/halten; Mute und DTMF; Audiogeräte lesen/auswählen; Kontakte suchen/importieren; Verlauf lesen/löschen.

Rust definiert die externen DTOs mit serde; TypeScript-Typen werden daraus mit ts-rs erzeugt. Die zusätzliche Build-Dependency vermeidet manuell doppelt gepflegte Modelle. Generierte Dateien werden überprüfbar eingecheckt; CI erkennt Abweichungen. Domänenmodelle werden nur dann gesondert abgebildet, wenn die API fachlich oder wegen Geheimnissen eine andere Darstellung braucht.

Für jeden Command dokumentiert docs/api Pflicht-/optionale Felder, erlaubte Werte, Validierung, Erfolgsantwort, Wirkung und Fehler. Änderungen dokumentieren ebenfalls die Ereignisse, Zeitformate und Bedeutung von IDs. Beispiele sind AccountView ohne Passwort, StartCallInput mit Konto-ID und Ziel sowie CallView mit Zustand und Konto-Zuordnung.

Ein einheitlicher AppError liefert einen stabilen Code, eine sichere Meldung und ggf. strukturierte Details. Fehlerklassen umfassen ungültige Eingabe, unbekannte ID, nicht registriertes/deaktiviertes Konto, bereits belegten Gesprächsplatz, ungültigen Gesprächszustand, fehlende Berechtigung, Schlüsselspeicherfehler und Engine-Ausfall. Exakte Feldnamen und Codes werden im folgenden Implementierungsplan festgelegt. Tauri nutzt Ergebnisse und Fehler statt HTTP-Statuscodes; SIP-Antwortcodes werden davon getrennt dokumentiert.

## Reihenfolge und überprüfbare Ergebnisse

| Meilenstein                     | Ergebnis                                                                                                       | Abnahme                                                                                                                                                                    |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M0: Native Machbarkeit          | Gepinnte baresip-/libre-Versionen, kleiner Rust/C-Aufrufpfad und Builds auf macOS und Windows                  | Registrierung, eingehender/ausgehender Anruf, Ton in beiden Richtungen und sauberer Shutdown auf beiden Plattformen; AEC- und TLS/SRTP-Buildpfad festlegen                 |
| M1: Erster vollständiger Ablauf | Installierbare Tauri-App, ein Konto, sicher hinterlegte Zugangsdaten, Nummer wählen/annehmen/ablehnen/auflegen | Nach App-Neustart wieder erreichbar; verständliche Fehler bei falschem Passwort und verweigertem Mikrofon; keine Geheimnisse in Logs oder Konfigurationsdateien            |
| M2: Alltagstelefonie            | Mehrere registrierte Konten, Kontowahl, Hold/Mute/DTMF, Audioauswahl und definierte Belegtbehandlung           | Zwei Konten parallel erreichbar; genau ein Gespräch; Sprachmenü reagiert auf DTMF; zweiter Anruf beeinflusst laufendes Gespräch nicht                                      |
| M3: Dauerhafter Verlauf         | SQLite-Migrationen, Verlauf, Rückruf und Wiederherstellung nach Absturz                                        | Ereignisse erzeugen keine doppelten Historieneinträge; Richtung, Ergebnis und Konto stimmen; unterbrochene Gespräche werden erkannt; Löschung bleibt nach Neustart wirksam |
| M4: Kontakte                    | vCard-Import auf beiden Plattformen und native macOS-Kontakte                                                  | Mehrere Nummern, interne Durchwahlen, Unicode-Namen, wiederholter Import und verweigerte/entzogene Systemfreigabe funktionieren nachvollziehbar                            |
| M5: MVP-Abnahme                 | Tray-Verhalten, Standby/Netz-/Gerätewechsel, Verschlüsselung, Installer und Kompatibilitätsbericht             | Getestete Profile bestehen die vereinbarte Matrix auf beiden Plattformen; signierte/notarisierte Installationspakete für die veröffentlichte Beta                          |

Builds und automatisierte Checks auf beiden Plattformen beginnen in M0/M1. Release-Paketierung wird ab M1 mitgeführt. Erst M5 enthält die abschließende Beta-Freigabe, nicht den ersten Windows-Test.

Grobe Schätzung für einen erfahrenen Vollzeitentwickler: 2–4 Wochen für M0, danach 10–16 weitere Wochen für M1–M5. Das sind insgesamt etwa 3–5 Monate mit vorhandenem Zugang zu beiden Betriebssystemen, Testanlagen und einem Anbieter-Testkonto. Keine Terminzusage; Windows-Build, AEC, NAT und Tests mit echter Hardware bestimmen die Unsicherheit. Die Meilensteine werden nach M0 neu geschätzt.

## Kompatibilität und Freigabekriterien

Vorgeschlagene offiziell unterstützte erste Plattformen: macOS 14 oder neuer auf Apple Silicon (arm64) und Windows 11 x64. Für beide ist eine Laufzeitabnahme mit realen Audiogeräten auf entsprechender Hardware erforderlich; Build-Erfolg oder Emulation allein genügt nicht. Intel-Macs werden erst nach derselben Abnahme als weitere unterstützte Architektur aufgenommen. Windows ARM und weitere Betriebssysteme folgen später. Jede freigegebene Architektur erhält ein eigenes Build-Artefakt; das Packen wird auf sauberen Zielsystemen geprüft.

Erste Testziele: Asterisk und FreeSWITCH im reproduzierbaren lokalen Labor, anschließend FRITZ!Box und easybell als echte Umgebungen. Version, Transport, Codecs, Verschlüsselung, Netzwerkpfad und bestandene Funktionen werden pro Profil erfasst. Diese Liste sind Prüfziele, noch keine Kompatibilitätszusagen. Fehlender Labor-/Kontozugang verhindert die Freigabe dieses Profils, nicht die Arbeit an den anderen.

**Verpflichtendes Beta-Gate für M5:** Beide freigegebenen Plattformarchitekturen müssen gegen jeweils Asterisk und FreeSWITCH Registrierung, Ein-/Ausgang mit Ton in beiden Richtungen, Annehmen/Ablehnen/Abbruch, Mute, Hold/Resume, RFC-4733-DTMF und korrekt zugeordneten Verlauf bestehen. Pro Anlagen-/Plattformkombination werden G.711 A-law und mu-law jeweils mit UDP/RTP, TCP/RTP und TLS/SDES-SRTP (AES_CM_128_HMAC_SHA1_80) geprüft. Die TLS-Prüfung umfasst einen vertrauenswürdigen Testserver sowie erwartete Ablehnung bei ungültigem Zertifikat oder Hostnamen; ein Labor-CA-Zertifikat darf gezielt vertraut werden. Ein kombinierter Test mit zwei gleichzeitig registrierten Konten belegt die korrekte Kontowahl und zweite-Anruf-Abweisung.

Zusätzlich sind je Plattform ein 30-Minuten-Gespräch, Wiedererreichbarkeit nach Standby/Netzunterbrechung, Gerätestatus nach Headsetentzug sowie beidseitige Anrufe hinter einem üblichen Heimrouter gegen den außerhalb dieses LAN erreichbaren Testserver verpflichtend. Die exakten Serverversionen und Builds werden in M0 gepinnt. Fehlender Zugang zu Asterisk, FreeSWITCH oder einer der beiden Pflichtplattformen blockiert die Beta-Freigabe. FRITZ!Box/easybell, weitere Codecs und Intel-Macs sind zusätzliche, separat zu qualifizierende Profile und blockieren die Basis-Beta nicht. Die Veröffentlichung benennt nur tatsächlich bestandene Profile.

Ubiquiti UniFi Talk bleibt ein separates Erkundungsziel; eine unterstützte fremde Softphone-Anmeldung ist bisher nicht nachgewiesen. 3CX, sipgate, Yeastar, Grandstream und Enterprise-Systeme erhalten spätere Testprofile. Es gibt kein Versprechen, alle Anlagen in Version 0.1 abzudecken.

Automatisiert: Rust-Zustandsübergänge, gleichzeitige Aktionen, SQLite-Migrationen und Historien-Idempotenz, Import-/Suchfälle, DTO-Verträge, ausgewählte UI-Interaktionen und SIPp-Szenarien. Native Integrationstests prüfen reales baresip; ein Fake allein ist kein Telefonienachweis.

Mit Hardware: Mikrofon/Lautsprecher und USB-/Bluetooth-Headset, SIP-Abbruch beim Klingeln, abgewiesene Gespräche, fehlender Codec, DTMF, Hold/Resume, 30-Minuten-Gespräch, Standby, Netzunterbrechung und Geräteentzug. Ungültige TLS-Zertifikate müssen scheitern; vorgeschriebene Medienverschlüsselung darf nicht still entfallen. Anrufe hinter einem üblichen Heimrouter werden in beiden Richtungen geprüft; sichere Erreichbarkeit hinter beliebigem CGNAT wird nicht vorausgesetzt. STUN, Outbound-Proxy oder SBC müssen zum jeweils getesteten Profil passen.

Vorgesehene Routinechecks: cargo fmt --check, cargo clippy und cargo test für den Workspace; Frontend-Typecheck, ausgewählte UI-Tests und Produktionsbuild; anschließend native Paket-Builds. Konkrete Commands und Testdateien werden erst mit dem tatsächlichen Scaffold verbindlich dokumentiert.

## MIT und Auslieferung

Eigener Code unter MIT. Fremdcode behält seine Lizenz. Gepinnte native Versionen, explizit ausgewählte Module und Codecs sowie Third-Party-Notices gehören in den Build und in die ausgelieferten Pakete. Optionale baresip-Module werden nicht pauschal aktiviert. Der konkrete transitive Lizenzbestand wird nach Auswahl der Build-Features geprüft; er wurde in dieser Planungsrunde noch nicht aufgelöst.

Ein eigener Cloud-Dienst ist für das MVP nicht vorgesehen. Updates und Signierung werden spätestens vor der Beta mit einem konkreten Veröffentlichungsweg, Plattformzertifikaten und geschützten CI-Zugangsdaten festgelegt. Kalender- und Kontaktdaten werden nicht an einen eigenen Server übertragen.

## Quellen und Grenzen der bisherigen Prüfung

- baresip als Bibliothek und Threading/Message-Queue: https://github.com/baresip/baresip/wiki/Using-baresip-as-a-Library
- Öffentliche aktuelle C-Schnittstelle: https://github.com/baresip/baresip/blob/main/include/baresip.h
- baresip-Funktionen, Plattformen und Module: https://github.com/baresip/baresip
- Tauri-Zustandsverwaltung: https://v2.tauri.app/develop/state-management/
- Tauri-Commands: https://v2.tauri.app/develop/calling-rust/
- Tauri-Berechtigungen, einschließlich eigener Commands: https://v2.tauri.app/security/capabilities/
- TypeScript-Typen aus Rust: https://github.com/Aleph-Alpha/ts-rs
- Kontakte auf macOS: https://developer.apple.com/documentation/Contacts
- Windows-API-Grenzen: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/winrt-api-desktop-app-support

Geprüft wurden Ordnerinhalt, Dokumentation und öffentliche Schnittstellen. Es wurde noch kein nativer Build, kein Paket und kein SIP-Anruf ausgeführt. Die ältere baresip-Wiki-Beispielanwendung ersetzt nicht die Prüfung gegen die konkret gepinnten Header.

Eine unabhängige Architekturprüfung hat den verpflichtenden Testumfang als einzige wesentliche offene Entscheidung identifiziert. Der Entwurf legt deshalb jetzt das Beta-Gate und die Hardwareabnahme pro freigegebener Architektur ausdrücklich fest.
