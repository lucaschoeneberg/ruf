# Datenschutzhinweise für Ruf

**ENTWURF – noch nicht zur Veröffentlichung freigegeben**\
**Stand: 6. November 2026 · geprüfter Funktionsstand: Ruf 0.1.0 für macOS**

Dieser Entwurf beschreibt den technisch geprüften Anwendungsstand. Die
Verantwortlichenrolle, die Rechtsgrundlagen und die tatsächlichen betrieblichen
Abläufe muss Mardu vor Veröffentlichung prüfen und bestätigen. Die ausdrücklich
gekennzeichneten Prüfpunkte am Ende gehören nicht in eine freigegebene Fassung.

## 1. Anbieterin und Datenschutzkontakt

Anbieterin von Ruf und Ansprechpartnerin für die von Mardu veranlassten
Verarbeitungen ist:

**Mardu GmbH**\
Alter Schlachthof 39 A1\
76131 Karlsruhe\
Deutschland

**Kontakt für Datenschutzanfragen:** [info@mardu.de](mailto:info@mardu.de)

Die Unternehmens- und Kontaktdaten wurden dem öffentlichen
[Impressum von Mardu](https://www.mardu.de/publisher) entnommen und am 6. November 2026 erneut abgerufen. Ein gesonderter Datenschutzbeauftragter wird
hier nicht benannt, da eine entsprechende Bestellung oder Kontaktadresse für
diesen Entwurf nicht bestätigt ist.

**Vor Freigabe zu prüfen:** Welche Verarbeitungen Mardu datenschutzrechtlich
verantwortet und welche im Verantwortungsbereich des Nutzers, eines Arbeitgebers
oder des gewählten Telefonieanbieters liegen. Die lokale Ausführung einer
Software entscheidet diese Rollenverteilung nicht allein.

## 2. Geltungsbereich

Ruf ist ein SIP-Softphone: Die App verbindet sich mit einem SIP-Konto, das du
selbst bei einer Telefonanlage oder einem Telefonieanbieter eingerichtet hast.
Ruf richtet kein Mardu-Telefoniekonto ein. Der geprüfte Anwendungscode enthält
keinen Mardu-Server für die Vermittlung oder Speicherung deiner Gespräche.

Diese Hinweise betreffen die macOS-App. Sie beschreiben keine eigenständigen
Telefonieangebote, Websites oder Dienste deines SIP-Anbieters. Für die
[Mardu-Website](https://www.mardu.de/) gelten die gesonderten
[Website-Datenschutzhinweise](https://www.mardu.de/privacy). Die dort beschriebenen
Website-Analyse-, Formular- und Newsletterdienste sind im geprüften Ruf-Code
nicht als App-Dienste eingebunden.

## 3. Welche Daten Ruf verarbeitet

### SIP-Konten und Passwörter

Wenn du ein SIP-Konto hinzufügst, verarbeitet Ruf dessen Anzeigenamen,
Serveradresse, Benutzernamen und Passwort sowie gegebenenfalls einen abweichenden
Authentifizierungsnamen und einen ausgehenden Proxy. Hinzu kommen gewählte
Transport-, Verschlüsselungs- und Aktivierungseinstellungen.

Kontometadaten und App-Einstellungen liegen im lokalen App-Datenordner. Gespeicherte
Passwörter werden im macOS-Schlüsselbund abgelegt, nicht in der Kontometadatendatei
oder auf einem Mardu-Server. Zum Anmelden verwendet die native Telefoniekomponente
das benötigte Passwort im Arbeitsspeicher; ein gespeichertes Passwort wird nicht
an die Oberfläche zurückgegeben. Das Passwortfeld verarbeitet natürlich die
Zugangsdaten, die du dort selbst eingibst.

### Anmeldung und Telefonate über deinen Anbieter

Für die Anmeldung, eingehende und ausgehende Anrufe, Halten, Vermitteln und
Tastentöne kommuniziert Ruf mit der von dir konfigurierten SIP-Infrastruktur.
Dabei werden insbesondere SIP-Kennungen, gewählte Rufnummern, technische
Verbindungsdaten und IP-Adressen verarbeitet. Während eines Gesprächs wird Audio
an die ausgehandelte Mediengegenstelle übertragen; empfangenes Audio wird lokal
wiedergegeben. Die Mediengegenstelle kann ein Server deines Anbieters oder eine
andere, im Verbindungsaufbau bestimmte Gegenstelle sein.

Diese Kommunikation ist für Telefonate erforderlich. Sie ist keine rein lokale
Verarbeitung. Dein Anbieter und die beteiligten Netz- oder Gesprächsgegenstellen
können eigene Verbindungsdaten verarbeiten oder speichern. Welche Daten dort wie
lange gespeichert werden, bestimmt sich nach deren tatsächlichen Diensten und
Datenschutzhinweisen. Ruf kann darüber keine allgemeine Zusage machen.

UDP/TCP und gewöhnliches RTP sind unverschlüsselt. TLS schützt die SIP-Verbindung
zum geprüften Server; die Einstellung für erforderliches SDES-SRTP setzt TLS
voraus und schützt die entsprechend ausgehandelte Medienverbindung. Daraus folgt
keine Garantie einer Ende-zu-Ende-Verschlüsselung über alle beteiligten Anbieter
und Gesprächsabschnitte.

### Mikrofon und Audiotests

Ruf nutzt das Mikrofon nach der erforderlichen macOS-Freigabe für Telefonate und
für einen von dir ausdrücklich gestarteten Mikrofontest. Das bloße Öffnen der
Datenschutzeinstellungen startet keine Mikrofonaufnahme.

Normale Telefonate werden von Ruf nicht als Audiodateien aufgezeichnet. Der
Mikrofon-Pegeltest wertet kurzzeitig Audioproben für die Pegelanzeige aus und
verwirft sie; er erstellt keine Aufnahme und lädt nichts hoch. Lautsprecher- und
Klingeltests geben lokal erzeugte Töne wieder.

Davon zu unterscheiden ist der ausdrücklich auswählbare
**Entwicklungs-Testtonmodus**: Er ersetzt das Mikrofon durch einen künstlichen Ton
und speichert empfangenes Audio lokal als `received.wav` im App-Cache. Dieses
empfangene Audio kann auch Inhalte der Gegenstelle enthalten. Die Datei bleibt
im Cache und wird bei späteren Testtonaufzeichnungen überschrieben. Dieser
Sondermodus ist keine allgemeine Mitschnittfunktion für normale Telefonate.

### macOS-Kontakte und Favoriten

Nach einer entsprechenden Systemfreigabe liest Ruf aus den zugänglichen
macOS-Kontakten Namen, Organisationen und beschriftete Telefonnummern sowie die
zur Zuordnung erforderlichen Kontaktkennungen. Diese Angaben dienen der lokalen
Suche, Nummernauswahl und Namensanzeige bei Anrufen, im Verlauf und in Mitteilungen.
Ruf verändert das Systemadressbuch nicht.

Die gelesenen Kontaktinhalte werden in Ruf im Arbeitsspeicher vorgehalten und
nicht als eigenes Adressbuch auf einem Mardu-Server gespeichert. Die App legt
lokal die Kennungen der von dir markierten Favoriten ab. Das zugrunde liegende
Systemadressbuch und dessen etwaige Synchronisation, beispielsweise über ein von
dir eingerichtetes Konto, werden von macOS und dem jeweiligen Kontodienst
verwaltet.

Ruf lädt deine Kontaktliste nicht zum SIP-Anbieter hoch. Wenn du einen Kontakt
anrufst, wird die ausgewählte Zielnummer für diesen Anruf an die SIP-Infrastruktur
übergeben. Kontakte sind optional: Du kannst Ruf auch ohne Adressbuchzugriff durch
manuelle Nummerneingabe verwenden.

### Lokaler Anrufverlauf

Standardmäßig speichert Ruf Anrufmetadaten lokal in einer SQLite-Datenbank. Dazu
gehören die ursprüngliche Kontoidentität, Anrufrichtung, Rufnummer beziehungsweise
ein von der Gegenstelle übermittelter SIP-Anzeigename, Zeitpunkte, beobachtete
Gesprächsdauer, Ergebnis und gegebenenfalls ein SIP-Statuscode. Das ist keine
Audioaufzeichnung. Aktuelle Namen aus dem macOS-Adressbuch werden für die Anzeige
hinzugerechnet und nicht als solche in die Verlaufsdatenbank geschrieben.

Die Voreinstellung für die Aufbewahrung lautet **Unbegrenzt**. Du kannst für
abgeschlossene Anrufe stattdessen **30, 90 oder 365 Tage** wählen. Die Frist
bezieht sich auf den Beginn des jeweiligen Anrufs. Die Bereinigung erfolgt beim
App-Start, beim Speichern geänderter Verlaufseinstellungen und während des
Betriebs täglich; sie läuft nicht weiter, während Ruf beendet ist. Laufende
Anrufe sind von der automatischen Löschung ausgenommen.

Du kannst die Speicherung für künftige Anrufe abschalten. Bereits gespeicherte
Einträge werden dadurch nicht gelöscht. Sie lassen sich im Verlauf einzeln oder
gemeinsam entfernen.

### Mitteilungen und andere Systemeinstellungen

Mit deiner Systemfreigabe kann Ruf lokale macOS-Mitteilungen über eingehende
Anrufe anzeigen. In der Voreinstellung können Name und Nummer des Anrufers darin
erscheinen. Du kannst Anruferdetails ausblenden oder Ruf-Mitteilungen deaktivieren.
Sichtbarkeit, Sperrbildschirmdarstellung und Aufbewahrung von Mitteilungen hängen
zusätzlich von deinen macOS-Einstellungen ab.

Für Audiogeräte, Berechtigungsstatus, Schlüsselbund und den optionalen Start bei
der Anmeldung nutzt Ruf Betriebssystemfunktionen. Der geprüfte Code enthält
keinen eigenen Push-, Cloud-Sync- oder Telemetriedienst von Mardu. Verarbeitung
durch macOS, den App Store und von dir aktivierte Apple-Dienste richtet sich
zusätzlich nach deinen Systemeinstellungen und den
[Datenschutzhinweisen von Apple](https://www.apple.com/legal/privacy/).

### Diagnose und Zwischenablage

Ruf kann lokal einen technischen Diagnosebericht anzeigen. Er enthält eine
begrenzte Auswahl an Zuständen, etwa App-Version, Plattform, Anmeldezähler,
Berechtigungsstatus und Fehlerkategorien. Er enthält keine Kontoadressen,
Passwörter, Rufnummern, Kontaktnamen oder Gesprächsinhalte.

Der Bericht wird nur durch deine ausdrückliche Aktion in die Zwischenablage
kopiert. Ruf versendet ihn nicht automatisch. Gleiches gilt für das ausdrückliche
Kopieren einer Telefonnummer. Ob und wie lange ein Inhalt in einer
Zwischenablageverwaltung oder einer von dir aktivierten geräteübergreifenden
Zwischenablage erhalten bleibt, hängt von deinen System- und Zusatzprogrammen ab.
Wenn du einen Bericht anschließend selbst an einen Supportdienst sendest, ist
dies ein gesonderter Vorgang.

Der geprüfte Anwendungscode bindet keine Werbe-, Tracking- oder Analytics-SDKs ein
und enthält keinen automatischen Upload von Diagnose- oder Absturzberichten an
Mardu. Davon zu unterscheiden sind etwaige von dir aktivierte Diagnosefunktionen
des Betriebssystems oder App Stores.

### Externe Links und optionale Online-Änderungshinweise

Die Firmen-, Impressums- und Datenschutzlinks öffnen nach deiner Auswahl den
Standardbrowser. Die App übergibt dabei eine feste Websiteadresse, keine
Kontodaten, Telefonnummern oder Kontaktlisten. Beim anschließenden Websitebesuch
gelten die dortigen Datenschutzhinweise.

Ruf enthält seine Versionshinweise lokal. Ein Build kann zusätzlich eine
öffentliche Markdown-Datei bei `raw.githubusercontent.com` als Onlinequelle
konfigurieren. Nur dann wird beim Öffnen von „Was ist neu?“ eine Anfrage
ausgeführt; „Aktualisieren“ fordert sie erneut an. Die App sendet dabei keine
App-Zugangsdaten, Cookies oder Referrer. Der Zielserver erhält dennoch die für
eine Internetanfrage üblichen technischen Verbindungsdaten, insbesondere die
IP-Adresse. Dessen Verarbeitung und Speicherdauer liegen nicht unter der
Kontrolle von Ruf; für GitHub gelten dessen
[Datenschutzhinweise](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

**Geprüfter Buildstand am 6. November 2026:** Im untersuchten aktuellen
Frontend-Build ist keine Onlinequelle konfiguriert; es wurde keine entsprechende
Build-Umgebungsvariable oder Projekt-Umgebungsdatei festgestellt. Dieser Stand
muss vor Veröffentlichung am tatsächlich ausgelieferten Build erneut bestätigt
werden. Ohne konfigurierte Quelle erfolgen keine Changelog-Anfragen.

## 4. Empfänger und mögliche internationale Verarbeitung

Die lokale App übermittelt Kontaktdaten, Anrufverlauf und gespeicherte Passwörter
nicht an einen zentralen Datenspeicher von Mardu. Für Anrufe erhalten die
konfigurierten SIP-Dienste und die ausgehandelten Gegenstellen die notwendigen
Telefonie- und Audiodaten. Hinzu kommen technische Dienste für die gewünschte
Netzverbindung, beispielsweise die von deinem System verwendete Namensauflösung.

Ob Telefonie-, Betriebssystem-, Browser- oder optional verwendete Inhaltsdienste
Daten außerhalb des Europäischen Wirtschaftsraums verarbeiten, hängt von den
tatsächlich eingesetzten Diensten ab. Deren Standorte, Fristen und etwaige
Übermittlungsgarantien sind anhand der jeweiligen Anbieterinformationen zu
prüfen. Dieser Entwurf behauptet keine pauschale Verarbeitung ausschließlich in
Deutschland oder der EU und keine unbestätigten Vertrags- oder
Übermittlungsgarantien.

## 5. Rechtsgrundlagen – vor Veröffentlichung zu prüfen

Die folgenden Zuordnungen sind **Entwurfsannahmen**, keine bereits bestätigten
Festlegungen für sämtliche Einsatzfälle. Zunächst muss feststehen, wer die
jeweilige Verarbeitung verantwortet.

- Soweit eine Verarbeitung durch Mardu zur Erfüllung eines tatsächlich
  bestehenden Vertrags über die App oder zugehöriger, angefragter Leistungen
  erforderlich ist, kommt Art. 6 Abs. 1 Buchst. b DSGVO in Betracht. Eine bestimmte
  Vertragsbeziehung wird mit diesem Entwurf nicht unterstellt.
- Soweit optionale Verarbeitungen auf einer wirksamen Einwilligung beruhen,
  kommt Art. 6 Abs. 1 Buchst. a DSGVO in Betracht. Eine macOS-Systemfreigabe allein
  ersetzt nicht die Prüfung der einschlägigen datenschutzrechtlichen Grundlage,
  insbesondere bei Angaben anderer Personen im Adressbuch.
- Für erforderliche Support-, Sicherheits- oder Fehlerbehandlungen kann
  Art. 6 Abs. 1 Buchst. f DSGVO in Betracht kommen. Mardu muss dafür die konkreten
  Zwecke, Erforderlichkeit und Interessenabwägung festlegen; mögliche Interessen
  sind die Bearbeitung einer Anfrage und der sichere Betrieb eigener Dienste.
- Art. 6 Abs. 1 Buchst. c DSGVO setzt eine konkret einschlägige gesetzliche
  Verpflichtung voraus. Dieser Entwurf erfindet keine solchen Pflichten oder
  daraus abgeleiteten Aufbewahrungsfristen.

Rechtsgrundlagen und Pflichten deines SIP-Anbieters oder eines betrieblichen
Betreibers werden durch diese Hinweise nicht festgelegt.

## 6. Deine Einstellungen und Löschmöglichkeiten

Diese Möglichkeiten bestehen im geprüften App-Stand:

1. **Kontakte und Mikrofon:** Unter **Einstellungen → Datenschutz** kannst du den
   Berechtigungsstatus prüfen und die zugehörigen Systemeinstellungen öffnen.
   Entziehe dort unter **Datenschutz & Sicherheit → Kontakte** beziehungsweise
   **Mikrofon** die Freigabe für Ruf. Ohne Kontaktzugriff entfernt Ruf die
   Kontaktprojektion; das Systemadressbuch bleibt unverändert. Ohne Mikrofonzugriff
   steht die entsprechende Mikrofonfunktion nicht zur Verfügung.
2. **Anrufverlauf:** Unter **Einstellungen → Kontakte & Verlauf** kannst du
   **Verlauf speichern** abschalten oder **Aufbewahrung** ändern und die Änderung
   mit **Anzeige & Verlauf speichern** übernehmen. Das erfordert einen ruhenden
   Telefoniezustand. Eine verkürzte Frist mit Löschwirkung muss bestätigt werden.
   Im **Verlauf** kannst du abgeschlossene Einträge über **Anruf löschen …** oder
   **Verlauf löschen …** nach Bestätigung entfernen.
3. **Favoriten:** Entferne die Sternmarkierung beim jeweiligen Kontakt. Dies
   entfernt die App-Favoritenzuordnung, keinen Kontakt aus dem macOS-Adressbuch.
4. **Konten und Zugangsdaten:** Wähle unter **Einstellungen → Konten** das Konto
   und **Konto entfernen …**. Ruf entfernt die lokale Kontokonfiguration und
   versucht das zugehörige gespeicherte Passwort aus dem Schlüsselbund zu
   löschen. Bei fehlendem Zugriff bleibt die Passwortlöschung vorgemerkt und
   wird als ausstehend gemeldet; sie ist dann noch nicht abgeschlossen. Entsperre
   den Schlüsselbund und starte Ruf erneut, damit die Bereinigung erneut versucht
   werden kann. Ein SIP-Konto wird damit nicht beim Telefonieanbieter gekündigt.
5. **Mitteilungen:** Unter **Einstellungen → Mitteilungen** kannst du
   **Anruferdetails anzeigen** oder **Eingehende Anrufe anzeigen** abschalten und
   die Änderung speichern. Die macOS-Mitteilungseinstellungen steuern zusätzlich
   die Systemfreigabe.

Das Entfernen eines Kontos löscht nicht automatisch dessen frühere
Verlaufseinträge. Der Entzug einer Kontaktfreigabe löscht ebenfalls keine bereits
gespeicherten SIP-Anrufmetadaten. Nutze dafür die getrennten Verlaufsfunktionen.

Das bloße Verschieben der App in den Papierkorb ist **keine zugesicherte
vollständige Datenlöschung**. Schlüsselbundeinträge, lokale App- und Cache-Dateien,
eine gegebenenfalls bei einer Datenmigration angelegte Metadaten-Sicherung und
eigene Systembackups können erhalten bleiben. Entferne gespeicherte Konten und
Verlauf deshalb vor einer Deinstallation über die App und beachte angezeigte
Löschfehler. Verbliebene Ruf-Daten, Testtondateien und zugehörige
Schlüsselbundeinträge müssen gegebenenfalls gesondert mit den Systemwerkzeugen
entfernt werden. Prüfe dabei die Zuordnung zu Ruf; diese Hinweise empfehlen keine
pauschale Löschung fremder Schlüsselbund- oder Systemdaten.

Die App-Löschfunktionen sind keine Zusage einer forensischen Überschreibung von
Datenträgern oder einer Löschung fremder Backups. Mardu kann lokale Daten ohne
deine gesonderte Mitwirkung nicht aus der Ferne einsehen oder löschen.

## 7. Kontakt- und Supportanfragen an Mardu

Wenn du Mardu selbst kontaktierst, erhält Mardu die von dir übermittelten Angaben,
beispielsweise deine Absenderadresse, Nachricht und freiwillig beigefügte
Diagnoseinformationen. Sende keine SIP-Passwörter oder unnötigen Gesprächs- und
Kontaktdaten mit.

Für Anfragen über die Mardu-Website gelten deren gesonderte Datenschutzhinweise.
Die tatsächlich für Ruf-Support verwendeten Empfangs-, Bearbeitungs- und
Löschprozesse einschließlich Dienstleistern, Rechtsgrundlagen und Speicherfristen
müssen vor Veröffentlichung dieses App-Entwurfs bestätigt werden. Eine konkrete
Support-Aufbewahrungsfrist wird hier nicht behauptet.

## 8. Deine Datenschutzrechte

Unter den jeweiligen gesetzlichen Voraussetzungen hast du insbesondere Rechte auf
Auskunft, Berichtigung, Löschung, Einschränkung der Verarbeitung und
Datenübertragbarkeit. Soweit eine Verarbeitung auf einer Einwilligung beruht,
kannst du diese mit Wirkung für die Zukunft widerrufen; die Rechtmäßigkeit einer
vorherigen Verarbeitung bleibt davon unberührt. Bei einer Verarbeitung auf
Grundlage berechtigter Interessen kannst du aus Gründen deiner besonderen
Situation widersprechen. Die genannten Rechte gelten nicht voraussetzungslos
oder für Daten, über die die angefragte Stelle nicht verfügt.

Für Verarbeitungen durch Mardu wende dich an
[info@mardu.de](mailto:info@mardu.de). Anfragen zu Daten, die dein SIP-Anbieter oder
ein anderer eigenständiger Dienst verarbeitet, solltest du auch an diesen
Anbieter richten. Die unmittelbar verfügbaren lokalen Bedienmöglichkeiten stehen
in Abschnitt 6.

Du kannst dich außerdem bei einer Datenschutzaufsichtsbehörde beschweren,
insbesondere an deinem gewöhnlichen Aufenthaltsort, deinem Arbeitsplatz oder am
Ort des vermuteten Verstoßes. Für den Unternehmenssitz von Mardu kommt der
[Landesbeauftragte für den Datenschutz und die Informationsfreiheit
Baden-Württemberg](https://www.baden-wuerttemberg.datenschutz.de/) als Anlaufstelle
in Betracht.

Der geprüfte App-Stand enthält kein Werbeprofiling und keine ausschließlich
automatisierten Entscheidungen mit rechtlicher oder ähnlich erheblicher Wirkung.
Die lokale Telefonnummernzuordnung dient ausschließlich der Kontaktanzeige und
ist keine Identitätsprüfung eines Anrufers.

## 9. Einordnung für Apples App-Datenschutzangaben

Die App-Store-Kategorie „Keine Daten erfasst“ ist von der Frage zu unterscheiden,
ob personenbezogene Daten überhaupt verarbeitet werden. Ruf verarbeitet lokal
Daten und überträgt für Telefonate notwendige Daten an die von dir gewählte
Telefonieinfrastruktur.

Nach Apples Definition gilt rein lokale Verarbeitung nicht als Datenerhebung für
das App-Privacy-Label. Maßgeblich für die Angabe sind insbesondere Daten, auf die
der Entwickler oder eingebundene Partner nach einer Übertragung länger als zur
Bearbeitung der Echtzeitanfrage zugreifen können. Für den geprüften Code ohne
eigenen Datenupload ist „Keine Daten erfasst“ eine technische Entwurfseinordnung,
**die Mardu anhand des finalen Builds und seiner tatsächlichen Dienste bestätigen
muss**. Ein von Mardu oder einem Partner betriebener Telefoniedienst, zusätzliche
Diagnoseübermittlung oder ein später aktivierter Onlinedienst wäre gesondert zu
bewerten. Die App-Store-Angabe ersetzt diese Datenschutzhinweise nicht.

## 10. Interne Freigabepunkte – vor Veröffentlichung bearbeiten

Die folgenden Punkte sind redaktionelle Arbeitsanweisungen und müssen vor einer
Veröffentlichung aufgelöst werden:

- Anbieterin, datenschutzrechtliche Rollenverteilung und Datenschutzkontakt
  bestätigen; prüfen, ob ein Datenschutzbeauftragter zu benennen ist.
- Die Annahmen aus Abschnitt 5 fachlich prüfen und durch die tatsächlich
  einschlägigen Rechtsgrundlagen sowie konkrete Verantwortlichkeiten ersetzen.
- Bestätigen, dass weder Mardu noch integrierte Partner im ausgelieferten Angebot
  zusätzliche Telefonie-, Telemetrie-, Analyse- oder Absturzdaten erhalten.
  Etwaige aus Apple-Diensten bezogene Daten und deren Verwendung gesondert prüfen.
- Ruf-Supportprozesse und gegebenenfalls eingesetzte Dienstleister,
  Übermittlungen und Speicherfristen bestätigen; keine Website-Verträge ungeprüft
  auf App-Verarbeitungen übertragen.
- Am finalen App-Store-Build die Changelog-Konfiguration und die Verfügbarkeit des
  Entwicklungs-Testtonmodus prüfen. Angaben zu dessen lokaler Audiodatei nur
  entfernen, wenn dieser Modus im veröffentlichten Build tatsächlich nicht
  erreichbar ist.
- Die dokumentierten Speicher-, Lösch- und Berechtigungspfade am endgültig
  signierten macOS-Build prüfen, einschließlich möglicher Schlüsselbundreste,
  App-Container und Backups. Keine automatische Löschung durch Deinstallation
  versprechen.
- Eine öffentlich erreichbare **Ruf-spezifische Datenschutzfassung** veröffentlichen
  oder die bestehende Unternehmensseite um diesen App-Geltungsbereich ergänzen.
  Die bisherige Seite `https://www.mardu.de/privacy` beschreibt zum Prüfdatum nur
  die Website. Die danach tatsächlich gültige URL muss in der App und in App
  Store Connect konsistent hinterlegt werden; dieser Entwurf ist noch keine
  öffentliche Datenschutzseite.
- App-Privacy-Antworten, App-Berechtigungstexte und endgültige Datenschutzhinweise
  auf denselben veröffentlichten Build abstimmen. Anschließend Entwurfsmarkierungen
  und interne Prüfanweisungen entfernen und ein Freigabedatum festlegen.

## Quellen und technische Grundlage des Entwurfs

Öffentliche Grundlagen, geprüft am 6. November 2026:

- [Mardu-Impressum](https://www.mardu.de/publisher) – Betreiberdaten und
  öffentlicher Kontakt; eigener HTTPS-Abruf erfolgreich.
- [Mardu-Website-Datenschutzhinweise](https://www.mardu.de/privacy) – bestehender
  Website-Geltungsbereich, keine veröffentlichte Ruf-spezifische Fassung.
- [Europäische Kommission: Information for individuals](https://commission.europa.eu/law/law-topic/data-protection/information-individuals_en)
  – Datenschutzinformationen und Betroffenenrechte.
- [LfDI Baden-Württemberg: Betroffenenrechte](https://www.baden-wuerttemberg.datenschutz.de/betroffenenrechte/)
  – Voraussetzungen der Rechte und Beschwerdekontext.
- [Apple: App privacy details](https://developer.apple.com/app-store/app-privacy-details/)
  – Definition der Datenerhebung und lokale Verarbeitung für das Privacy-Label.
- [Apple: App Review Guidelines, Abschnitt 5.1](https://developer.apple.com/app-store/review/guidelines/#privacy)
  – Anforderungen an appbezogene Datenschutzhinweise.

Technische Grundlagen im Repository:

- [Produktumfang](../apps/desktop/PRODUCT.md)
- [Kontakte](api/contacts.md), [Verlauf](api/history.md) und
  [Aufbewahrungseinstellungen](api/preferences.md)
- [Diagnosebericht](api/diagnostics.md), [Audiotests](api/audio-diagnostics.md) und
  [Telefonie-/Testtonmodus](api/README.md)
- [Externe Links und Changelog](api/about.md)
- `apps/desktop/src-tauri/src/store.rs` und `phone.rs` – Kontometadaten,
  Schlüsselbundzugriff und vorgemerkte Passwortlöschung
- `apps/desktop/src-tauri/src/history/database.rs` – lokale Verlaufsdatenbank
- `apps/desktop/src/lib/components/DataSettings.svelte`,
  `AccountSetup.svelte`, `NotificationSettings.svelte` und
  `PrivacySettings.svelte` – vorhandene Bedien- und Löschmöglichkeiten
