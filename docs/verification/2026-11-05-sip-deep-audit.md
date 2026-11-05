# SIP-Zuverlässigkeit: tiefer Audit am 5. November 2026

## Auftrag und Prüfgrenze

Geprüft wurde die vorhandene Implementierung von Svelte über Tauri, `Engine`,
Snapshot und Anrufjournal bis zur nativen baresip/libre-Ereignisschleife.
Schwerpunkt waren alle Annehmen-, Ablehnen- und Auflegen-Wege, aufeinanderfolgende
Anrufe, konkurrierende Aktionen und die Erholung nach Signalisierungs- und
Medienverlust. Die nativen Netzwerktests verwenden tatsächliche UDP-Pakete,
native SIP-Transaktionen und unveränderte Protokolltimer mit synthetischen
Loopback-Gegenstellen. Sie verwenden keine persönlichen Konten oder Kontakte.

Der Arbeitsbaum enthielt schon vor diesem Audit umfangreiche Änderungen.
Diese wurden erhalten. Vorhandene Korrekturen zur Registrierung, NAT-Lebensdauer,
Credential-Freigabe und Benachrichtigungszustellung wurden mitgelesen; dieser
Bericht schreibt sie nicht diesem Audit zu. Gleichzeitig entstandene Änderungen
an der Softwareübersicht gehören ebenfalls nicht zu diesen SIP-Korrekturen.

## Bestätigte und korrigierte Fehler

| Priorität | Fehler und tatsächliche Folge                                                                                                         | Ursache und Korrektur                                                                                                                                                                                                                                             | Nachweis                                                                                                                                                                                                                                                        |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1        | Netzverlust konnte den einzigen Anrufplatz dauerhaft belegen.                                                                         | Ohne RTP-Inaktivitätstimeout blieb ein etablierter Anruf aktiv, wenn Medien und entferntes Auflegen ausblieben. Die Netzwerkerholung wartete gleichzeitig auf einen freien Anrufplatz. Der vorhandene baresip-Watchdog ist jetzt auf 60 Sekunden gestellt.        | Echter PCMU-Empfang wurde durch native Aufnahme belegt. Anschließend fiel die Gegenstelle vollständig aus. Vorher war der Anruf nach 65 Sekunden weiter aktiv; danach wird er ohne Benutzer-Auflegen freigegeben und ein weiterer eingehender Anruf angenommen. |
| P1        | Ein veraltetes „Ablehnen“ konnte einen inzwischen angenommenen Anruf beenden.                                                         | Hauptfenster und Tray verwendeten allgemeines Auflegen, während Benachrichtigungen bereits den zustandsgeprüften Reject-Pfad nutzten. Ein eigener IPC-Befehl und ein eigener Menüaktionspräfix führen jetzt auf denselben vorhandenen `Engine::reject_call`-Pfad. | Browserregression vor der Korrektur rot: Der extern angenommene Anruf wurde idle. Nachher bleibt er aktiv. Native Token- und Incoming-Prüfungen bleiben erhalten.                                                                                               |
| P2        | Eine fehlgeschlagene native Annahme konnte den weiter angebotenen Anruf verstummen lassen.                                            | Der Klingelton wurde vor `ua_answer` gestoppt. Bei einem behebbaren Fehler kehrte Rust zu Incoming zurück, ohne den Klingelton wiederherzustellen. Er wird nun erst nach erfolgreicher nativer Annahme gestoppt.                                                  | Der echte C-Queuehandler wird mit fehlschlagendem und anschließend erfolgreichem `ua_answer` ausgeführt. Die libre-Playback-Referenz bleibt beim Fehler erhalten und wird beim Erfolg genau einmal freigegeben.                                                 |
| P2        | Ein tatsächlich beantworteter Anruf ohne ACK verschwand ohne Fehleranzeige und wurde als verpasst statt fehlgeschlagen klassifiziert. | Der native Stack schließt diesen Aufbau mit Status null. Das neue private Ereignis `EV_ANSWER_CLOSED` unterscheidet die tatsächlich gesendete Antwort vom bloßen Rust-Zustand Answering. Snapshot und Journal erhalten einen belegten Aufbaufehler.               | Echte 32-Sekunden-UDP-Transaktion ohne ACK: zunächst reproduziert ohne Fehlermeldung, nachher `nativeFailure`, kein SIP-Status, freier Slot und erfolgreicher nächster Anruf.                                                                                   |
| P2        | Zwei bereits eingereihte Auflegen-Aktionen konnten nach erfolgreichem Auflegen einen falschen NativeFailure melden.                   | Die zweite Aktion fand bei gleicher Generation keinen Anruf mehr und meldete EINVAL. Derselbe bereits geschlossene native Anruf wird nun idempotent behandelt. Andere Generationen bleiben gesperrt.                                                              | Regression direkt gegen den Produktions-Queuehandler: vor der Korrektur generischer Fehler, danach keine Fehlermeldung.                                                                                                                                         |
| P2        | Tatsächlich gleichzeitiges Annehmen/Auflegen konnte einen falschen nativen Fehler melden.                                             | Auflegen hatte den Anruf schon geschlossen, eine bereits eingereihte Annahme derselben Generation lief danach in EINVAL. Ohne lebenden Anruf wird diese Annahme jetzt als veraltete Aktion mit `invalidState` behandelt.                                          | Zwei Barrier-synchronisierte Threads durch dieselbe Engine-Sperre reproduzierten vorher `nativeFailure` nach 0,21 Sekunden. Danach zwölf Rennen mit jeweils erfolgreichem Folgeanruf und Schutz gegen alte Tokens bestanden.                                    |
| P2        | Ein ausgehender Versuch, der den Slot gegen einen eingehenden Anruf verlor, konnte dessen Fehleranzeige überschreiben.                | Neben dem korrekt dem Versuch zugeordneten DialFailed wurde ein generischer Fehler mit der Generation des laufenden Anrufs gesendet. Der doppelte generische Fehler entfällt für Dial; der spezifische Fehler bleibt.                                             | Native Queue-Regression mit unterschiedlichen Tokens und Rust-Callback-Regression: Gewinner bleibt erhalten und fehlerfrei; ein tatsächlich eigener fehlgeschlagener Versuch meldet weiterhin seinen Fehler.                                                    |

Beim Einbau des Medien-Watchdogs wurde zusätzlich ein Integrationsfehler vor
Abschluss gefunden: Nach mehr als 60 Sekunden legitimem Halten war der alte
Empfangszeitpunkt beim Fortsetzen bereits abgelaufen. Ein nativer Test zeigte
das sofortige Ende nach Resume. Der Resume-Handler aktiviert jetzt den bestehenden
Stream-Timer neu; dadurch beginnt ein vollständiges Empfangszeitfenster. Der
Regressionstest hält 65 Sekunden und prüft anschließend das fortgesetzte Gespräch.

## Einheitliche Bedienpfade

| Oberfläche                           | Annehmen                                   | Ablehnen eines angebotenen Anrufs          | Auflegen eines laufenden Anrufs                        |
| ------------------------------------ | ------------------------------------------ | ------------------------------------------ | ------------------------------------------------------ |
| Hauptfenster / gemeinsames CallPanel | `answer` → `Engine::answer_call`           | `reject` → `Engine::reject_call`           | `hangup` → `Engine::hangup_call`                       |
| Tray / Anwendungsmenü                | `answer:<token>` → dieselbe Engine-Methode | `reject:<token>` → dieselbe Engine-Methode | `hangup:<token>` → dieselbe Engine-Methode             |
| Native Anrufbenachrichtigung         | gebundene Aktion → dieselbe Engine-Methode | gebundene Aktion → dieselbe Engine-Methode | Keine allgemeine Auflegen-Aktion für etablierte Anrufe |

Alle Pfade verwenden die bestehende verwaltete Sitzung und denselben nativen
Queuehandler. Ein Call-Token bezeichnet eine lokale Anrufgeneration. Die native
Ausführung prüft ihn erneut. „Ablehnen“ ist nur für einen unbeantworteten Incoming
zulässig; „Auflegen“ bleibt in Calling, Ringing, EarlyMedia, Answering, Active und
Held möglich. Menüs erfassen die ursprüngliche Bedeutung und das Token gemeinsam,
damit ein altes „Ablehnen“ nicht durch eine spätere Zustandsänderung zum Auflegen wird.

Die Anrufanzeige bleibt auch bei geöffneten Kontakten, Verlauf und Einrichtung
erreichbar. Eine fehlgeschlagene Befehlsausführung setzt die UI-Sperre zurück.
Fehlgeschlagene Snapshot-Lesevorgänge löschen keinen bekannten Anruf. Ereignisse
werden über die vorhandenen Revisionen geordnet; Polling kann die Anzeige nach
ausbleibender Ereigniszustellung wieder aktualisieren. Registrierung des gewählten
ausgehenden Kontos ist keine Voraussetzung für die Annahme auf einem anderen Konto.

Ein unbeantwortetes eingehendes Angebot besitzt bereits den nativen
`call_local_timeout` von 120 Sekunden (`call_accept` startet den vorhandenen
Invite-Timer). Dieser Pfad wurde im Quellcode verfolgt; sein voller
120-Sekunden-Ablauf wurde in diesem Audit nicht zusätzlich ausgeführt.
Absichtlich veraltete Aktionen können weiterhin eine `invalidState`-Warnung
auslösen; diese verändert weder den Anrufzustand noch den belegten Slot.

## Verhaltens- und Erholungsmatrix

| Szenario                                     | Ausgeführter Nachweis                                                                                                                                                                                         |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Mehrere eingehende Anrufe nacheinander       | Neue SIP-Dialog-IDs und Branches; wiederholtes Annehmen, ACK und entferntes BYE im selben Engine-Lauf.                                                                                                        |
| Abbruch und Ablehnung in einer Serie         | Abwechselnd entferntes CANCEL und lokale 486-Ablehnung; anschließend erfolgreicher eingehender Anruf.                                                                                                         |
| Annehmen und vor ACK auflegen                | Dreimal hintereinander im nativen Stack; verspätetes ACK bleibt folgenlos; nächster Anruf wird angenommen.                                                                                                    |
| Tatsächlich gleichzeitiges Annehmen/Auflegen | Zwölf Thread-/Barrier-Rennen über dieselbe Sitzungsserialisierung, jeweils gefolgt von neuem Incoming, Annahme, ACK, Active und BYE sowie alten Token-Aktionen.                                               |
| Fehlendes ACK nach gesendeter Antwort        | Echter nativer 32-Sekunden-Timeout; sichtbarer Fehler, kein verlorener Slot, danach nächster Anruf.                                                                                                           |
| Fehlende Antwort auf lokales BYE             | Alter Dialog wiederholt BYE; neuer Anruf bleibt über 35 Sekunden und damit über den alten Transaktionstimeout erhalten.                                                                                       |
| Vollständiger Signalisierungsausfall         | REGISTER und INVITE werden bei weiterhin gebundenen UDP-Sockets verworfen, einschließlich Wiederholungen. Tatsächlicher Timeout, automatische Wiederregistrierung und erfolgreicher Folgeanruf ohne Neustart. |
| Medienausfall eines etablierten Gesprächs    | Zunächst tatsächlicher PCMU-Empfang; danach kein RTP/RTCP und keine Signalisierungsantworten. Automatische Freigabe durch nativen Watchdog und nächste Annahme.                                               |
| Langes legitimes Halten                      | 65 Sekunden ohne Medien beenden Held nicht. Resume bekommt eine neue Frist und bleibt bedienbar.                                                                                                              |
| Veraltete Aktion für Anruf A                 | Annehmen, Ablehnen und Auflegen können den neuen Anruf B nicht verändern.                                                                                                                                     |
| UI-Befehlsfehler                             | Fehlgeschlagene Annahme, Ablehnung und Auflegen lassen erneute Bedienung und den nächsten Anruf zu.                                                                                                           |
| Alle lokalen Beendigungszustände             | Browserfälle für Incoming, Answering, Calling, Ringing, EarlyMedia, Active und Held; danach neuer ausgehender Anruf.                                                                                          |

Browserprüfungen ersetzen dabei die IPC-Gegenstelle durch einen zustandsbehafteten
Fixture. Sie belegen Bedienbarkeit und Wrapper, nicht tatsächliche SIP-/RTP-Zustellung.
Die separaten nativen Tests belegen Protokoll- und Engine-Verhalten.

## Wiederverwendung und Dokumentation

Es wurden keine neuen Dependencies, kein SIP im Frontend und keine zweite
Audio-Pipeline eingeführt. Wiederverwendet wurden `PhoneState`, `with_session`,
`Engine::reject_call`, Call-Tokens, Queue-/Callback-Protokoll, ts-rs, CallJournal,
baresip-Playback und der vorhandene Stream-Watchdog. Die bestehenden Loopback-,
Asterisk-, Browser- und CMake-Testwege wurden erweitert.

Betroffene Produktionsdateien sind `App.svelte`, `lib/api.ts`, Tauri
`commands.rs`, `shell.rs`, `lib.rs`, `build.rs`, `capabilities/main.json` samt
generierter Reject-Permission sowie Adapter `native/bridge.c` und `src/ffi.rs`.
Tests liegen in `nat_tests.rs`, `sip_event_tests.rs`, `tests/ui/browser/phone.mjs`,
`native/tests/call_actions.c`, dessen CMake-Target und dem vorhandenen
`scripts/check-network-roam.mjs`-Runner. Vorherige Änderungen in denselben Dateien
bleiben separat bestehen.

Der additive Reject-Befehl, Pflichtfeld, Zustandsvalidierung, Fehler und Rückgabe
sind in `docs/api/README.md` dokumentiert. Das private Ereignis und seine
Ownership-/Fehlersemantik stehen in `native/README.md`; Medienerholung und
Snapshot-Semantik in `docs/api/sip-events.md`; die Journal-Klassifizierung in
`docs/api/history.md`. Es werden keine neuen öffentlichen DTO-Felder benötigt.

## Grenzen

- Die automatische Medienerholung misst die tatsächliche gemeinsame
  RTP/RTCP-Aktivität des vorhandenen Stacks. Weiter eintreffendes RTCP hält einen
  Anruf auch ohne Audio-RTP am Leben. Mikrofonstille allein beendet keinen Anruf.
- Ein Peer ohne RTP/RTCP für mehr als 60 Sekunden muss Hold aushandeln, wenn das
  Gespräch erhalten bleiben soll. Für lange, korrekt ausgehandelte Hold-Phasen
  bleibt manuelles Auflegen verfügbar; Medieninaktivität dient dort nicht als
  Abbruchsignal.
- Ein unerreichbares Netz kann keine neuen Anrufe zustellen. Die Tests belegen
  Freigabe und erneute Annahme nach Wiederherstellung des synthetischen Pfads,
  keine Unterbrechungsfreiheit während des Ausfalls.
- Physisches WLAN-Abschalten, beliebige VPN-/Router-Wechsel bei gleicher IP,
  Sleep/Wake, jedes Providerverhalten und Windows-Laufzeit wurden hier nicht
  ausgeführt. Die vorhandenen C-Tests prüfen Transport-Erholung mit injizierter
  Interface-Inventarisierung; tatsächlicher Paketverlust wird separat nativ geprüft.
- Der Klingelton-Test prüft native Referenz-/Fehlerbehandlung ohne Hardware.
  Hörbarkeit und OS-Gerätezulassungen werden dadurch nicht behauptet.
- Aus einem Audit folgt keine mathematische Garantie über alle möglichen Rennen.
  Entscheidend sind die beschriebenen reproduzierten Fehler, gezielten Patches
  und wiederholbaren Verhaltensnachweise.

## Abschlusschecks

| Check                                                                                          | Beobachtetes Ergebnis                                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `bun run native:build`                                                                         | SDK erfolgreich neu gebaut und bereitgestellt; SDK-NAT-Regressionsfälle bestanden.                                                                                                                                                                                                                                  |
| `bun run format:check`                                                                         | Erfolgreich.                                                                                                                                                                                                                                                                                                        |
| `bun run check`                                                                                | Svelte: null Fehler und null Warnungen; TypeScript erfolgreich.                                                                                                                                                                                                                                                     |
| `bun run build`                                                                                | Erfolgreich.                                                                                                                                                                                                                                                                                                        |
| `bun run check:versions`                                                                       | Erfolgreich, Bun 1.4.4.                                                                                                                                                                                                                                                                                             |
| `bun run types:generate`                                                                       | Erfolgreich; Hashvergleich aller generierten Dateien vor/nach erneuter Generierung unverändert.                                                                                                                                                                                                                     |
| `cargo clippy --locked --workspace --all-targets -- -D warnings`                               | Erfolgreich.                                                                                                                                                                                                                                                                                                        |
| `cargo clippy --locked --workspace --features sip-desktop/native --all-targets -- -D warnings` | Erfolgreich, einschließlich abschließender nativer Korrekturen.                                                                                                                                                                                                                                                     |
| `cargo test --locked --workspace`                                                              | Erfolgreich; native-only Tests sind in diesem portablen Lauf nicht aktiv.                                                                                                                                                                                                                                           |
| `cargo test --locked --workspace --features sip-desktop/native -- --test-threads=1`            | Erfolgreich nach dem letzten Race-Fix: insgesamt 132 bestanden, null fehlgeschlagen, zehn explizit übersprungen. Adapter: 56 bestanden; Desktop: 61 bestanden; übrige Core-/Lifecycle-Tests ebenfalls bestanden. Die sechs Asterisk-Fälle und ein Desktop-Authorization-Fall wurden separat erfolgreich ausgeführt. |
| `bun run test:ui`                                                                              | 43 bestanden, null fehlgeschlagen. Die zusätzliche Softwareübersicht war bereits parallel im Arbeitsbaum hinzugekommen.                                                                                                                                                                                             |
| `bun run test:ui:flows`                                                                        | Alle fünf Suites erfolgreich: Kontakte, Verlauf, Telefon, Einstellungen, Layout; 37 Telefon-Browserfälle.                                                                                                                                                                                                           |
| `bun run test:network-roam`                                                                    | Sechs Interface-/Retry-Zustandstests und der native Anrufaktions-Harness erfolgreich.                                                                                                                                                                                                                               |
| `bun run test:asterisk`                                                                        | Alle sechs Interoperabilitätstests erfolgreich; RTP-Echo 4,04 Sekunden, RMS 5811, 440,1 Hz.                                                                                                                                                                                                                         |
| `bun run test:sip-diagnostics`                                                                 | Erfolgreich; 62 erlaubte Marker, sensible Fixture-Inhalte ausgeschlossen und standardmäßig deaktiviert.                                                                                                                                                                                                             |
| `bun run desktop:build --features native --bundles app`                                        | Native macOS-App erfolgreich gebaut und ad hoc signiert; nicht notarisiert oder installiert.                                                                                                                                                                                                                        |
| `git diff --check`                                                                             | Erfolgreich.                                                                                                                                                                                                                                                                                                        |

Rust-Prüfungen mit nativer Verknüpfung wurden über die gepinnte Toolchain 1.98.1
ausgeführt. Der anfängliche No-ACK-RED-Lauf verwendete noch den auf dem Host
voreingestellten Compiler; die abschließenden Prüfungen verwenden den Projekt-Pin.
Die nativen Library-Tests werden seriell ausgeführt, da der SDK-Zustand pro
Prozess global ist. Ein früherer paralleler Gesamtlauf wurde vor dem endgültigen
Stand bewusst abgebrochen und wird nicht als bestandener Abschlusscheck gezählt.

Die sechs im Workspace-Lauf übersprungenen Asterisk-Fälle wurden durch
`test:asterisk` ausdrücklich ausgeführt. Separate Tests mit echten
OS-Keychain-Schreibzugriffen oder Hardware-Metadaten werden dadurch nicht als
ausgeführt behauptet. Die Linkerwarnungen zur neueren macOS-Zielversion der
vorhandenen Homebrew-OpenSSL-Objekte bleiben eine Distributionsgrenze für alte
macOS-Versionen; die hier erzeugte App wurde auf diesem Host gebaut.
Der App-Build meldete außerdem fehlgeschlagenes Debug-Symbol-Stripping, weil dem
lokalen `rust-objcopy` seine `libLLVM.dylib` fehlt. Cargo und Tauri beendeten den
Build trotzdem erfolgreich und erstellten das signierte Bundle. Die Toolchain-
Installation wurde dafür nicht außerhalb des Projekts verändert.

Zusätzlich wurde der vorhandene Desktop-Test
`incoming_during_authorization_preserves_account_and_cleanup_journal` mit
`cargo test --locked -p sip-desktop --features native incoming_during_authorization_preserves_account_and_cleanup_journal -- --ignored --test-threads=1`
ausdrücklich ausgeführt und bestand. Er prüft gegen Asterisk, dass ein während
blockierter Credential-Freigabe eingehender Anruf bedienbar bleibt und die
Kontometadaten erhalten bleiben; er verwendet ausschließlich In-Memory-Credentials.
