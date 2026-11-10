# Privacy notice for Ruf

**DRAFT – not yet approved for publication**\
**As of: 6 November 2026 · reviewed feature set: Ruf 0.1.0 for macOS**

This draft describes the application as technically reviewed. Before publication,
Mardu must review and confirm its role as controller, the legal bases and the
actual operational processes. The explicitly marked review points at the end do
not belong in an approved version.

## 1. Provider and privacy contact

The provider of Ruf and contact for processing initiated by Mardu is:

**Mardu GmbH**\
Alter Schlachthof 39 A1\
76131 Karlsruhe\
Germany

**Contact for privacy requests:** [info@mardu.de](mailto:info@mardu.de)

The company and contact details were taken from Mardu's public
[legal notice](https://www.mardu.de/publisher) and retrieved again on 6 November 2026. No separate data protection officer is named here because no such
appointment or contact address has been confirmed for this draft.

**To be reviewed before approval:** Which processing Mardu is responsible for
under data protection law and which falls within the responsibility of the user,
an employer or the chosen telephony provider. Running software locally does not
by itself determine this allocation of roles.

## 2. Scope

Ruf is a SIP softphone: the app connects to a SIP account you have set up yourself
with a telephone system or telephony provider. Ruf does not create a Mardu
telephony account. The reviewed application code contains no Mardu server for
routing or storing your calls.

This notice covers the macOS app. It does not describe separate telephony
products, websites or services offered by your SIP provider. The
[Mardu website](https://www.mardu.de/) is covered by its separate
[website privacy notice](https://www.mardu.de/privacy). The website analytics,
form and newsletter services described there are not integrated as app services
in the reviewed Ruf code.

## 3. Data Ruf processes

### SIP accounts and passwords

When you add a SIP account, Ruf processes its display name, server address,
username and password, and, where applicable, a separate authentication name and
an outbound proxy. This also includes your chosen transport, encryption and
activation settings.

Account metadata and app settings are stored in the local app data folder. Saved
passwords are stored in the macOS Keychain, not in the account metadata file or
on a Mardu server. To register, the native telephony component uses the required
password in memory; a saved password is not returned to the user interface. The
password field does, of course, process credentials you enter there yourself.

### Registration and calls through your provider

For registration, incoming and outgoing calls, hold, transfers and keypad tones,
Ruf communicates with the SIP infrastructure you configure. This involves
processing SIP identifiers, dialed phone numbers, technical connection data and
IP addresses in particular. During a call, audio is transmitted to the negotiated
media endpoint; received audio is played locally. The media endpoint may be a
server belonging to your provider or another endpoint determined during call
setup.

This communication is required for calls. It is not purely local processing.
Your provider and the network or call endpoints involved may process or store
their own connection data. What data they store and for how long depends on their
actual services and privacy notices. Ruf cannot make a general promise about
this.

UDP/TCP and ordinary RTP are unencrypted. TLS protects the SIP connection to the
verified server; the setting requiring SDES-SRTP requires TLS and protects the
media connection negotiated accordingly. This does not guarantee end-to-end
encryption across all providers and call segments involved.

### Microphone and audio tests

After the required macOS permission is granted, Ruf uses the microphone for calls
and for a microphone test you explicitly start. Simply opening the privacy
settings does not start microphone recording.

Ruf does not record normal calls as audio files. The microphone level test
briefly evaluates audio samples for the level display and discards them; it does
not create a recording or upload anything. Speaker and ringtone tests play
locally generated tones.

The explicitly selectable **development test tone mode** is a separate case: it
replaces the microphone with an artificial tone and saves received audio locally
as `received.wav` in the app cache. This received audio may also include content
from the other party. The file remains in the cache and is overwritten by later
test tone recordings. This special mode is not a general recording feature for
normal calls.

### macOS contacts and favorites

After the corresponding system permission is granted, Ruf reads names,
organizations and labeled phone numbers from accessible macOS contacts, along
with the contact identifiers needed to associate them. These details support
local search, number selection and name display for calls, history and
notifications. Ruf does not modify the system address book.

Ruf keeps the contact contents it reads in memory and does not store them as a
separate address book on a Mardu server. The app stores the identifiers of
contacts you mark as favorites locally. The underlying system address book and
any synchronization, for example through an account you have configured, are
managed by macOS and the respective account service.

Ruf does not upload your contact list to the SIP provider. When you call a
contact, the selected destination number is passed to the SIP infrastructure for
that call. Contacts are optional: you can also use Ruf without address book
access by entering numbers manually.

### Local call history

By default, Ruf stores call metadata locally in a SQLite database. This includes
the original account identity, call direction, phone number or a SIP display name
provided by the other party, timestamps, observed call duration, result and,
where applicable, a SIP status code. This is not an audio recording. Current
names from the macOS address book are added for display and are not written as
such to the history database.

The default retention setting is **Unlimited**. You can instead choose **30, 90
or 365 days** for completed calls. The period is measured from the start of each
call. Cleanup runs when the app starts, when changed history settings are saved
and daily while the app is running; it does not continue while Ruf is closed.
Ongoing calls are excluded from automatic deletion.

You can disable storage for future calls. This does not delete entries already
stored. You can remove them individually or all together in the history.

### Notifications and other system settings

With your system permission, Ruf can display local macOS notifications for
incoming calls. By default, these may show the caller's name and number. You can
hide caller details or disable Ruf notifications. Notification visibility, lock
screen display and retention also depend on your macOS settings.

Ruf uses operating system functions for audio devices, permission status, the
Keychain and optional launch at login. The reviewed code contains no Mardu push,
cloud sync or telemetry service of its own. Processing by macOS, the App Store
and Apple services you enable is also governed by your system settings and
[Apple's privacy policy](https://www.apple.com/legal/privacy/).

### Diagnostics and clipboard

Ruf can display a technical diagnostic report locally. It contains a limited
selection of states, such as app version, platform, registration counters,
permission status and error categories. It contains no account addresses,
passwords, phone numbers, contact names or call content.

The report is copied to the clipboard only when you explicitly request it. Ruf
does not send it automatically. The same applies to explicitly copying a phone
number. Whether and for how long content remains in a clipboard manager or a
cross-device clipboard you have enabled depends on your system and additional
software. If you then send a report to a support service yourself, that is a
separate action.

The reviewed application code does not integrate advertising, tracking or
analytics SDKs and contains no automatic upload of diagnostic or crash reports
to Mardu. Any diagnostic features of the operating system or App Store you have
enabled are a separate matter.

### External links and optional online release notes

Selecting the company, legal notice or privacy links opens the default browser.
The app passes a fixed website address, without account data, phone numbers or
contact lists. The subsequent website visit is governed by that website's privacy
notice.

Ruf includes its release notes locally. A build can additionally configure a
public Markdown file at `raw.githubusercontent.com` as an online source. Only in
that case is a request made when opening “What's new?” (“Was ist neu?”);
“Refresh” (“Aktualisieren”) requests it again. The app sends no app credentials,
cookies or referrer. The destination server nevertheless receives the technical
connection data customary for an internet request, in particular the IP address.
Its processing and retention are outside Ruf's control; GitHub is governed by
its [privacy statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

**Reviewed build state on 6 November 2026:** No online source is configured in
the current frontend build examined; no corresponding build environment variable
or project environment file was found. This state must be confirmed again
against the build actually distributed before publication. Without a configured
source, no changelog requests are made.

## 4. Recipients and possible international processing

The local app does not transmit contact data, call history or saved passwords to
a central Mardu data store. For calls, the configured SIP services and negotiated
endpoints receive the necessary telephony and audio data. Technical services for
the requested network connection are also involved, for example the name
resolution used by your system.

Whether telephony, operating system, browser or optional content services process
data outside the European Economic Area depends on the services actually used.
Their locations, retention periods and any transfer safeguards must be checked
against the respective providers' information. This draft makes no blanket claim
that processing takes place exclusively in Germany or the EU, nor any
unconfirmed claims about contractual or transfer safeguards.

## 5. Legal bases – to be reviewed before publication

The following assignments are **draft assumptions**, not confirmed
determinations for all use cases. It must first be established who is responsible
for each processing activity.

- Where processing by Mardu is necessary to perform an actual contract for the
  app or related services requested, Article 6(1)(b) GDPR may apply. This draft
  does not assume any particular contractual relationship.
- Where optional processing is based on valid consent, Article 6(1)(a) GDPR may
  apply. A macOS system permission alone does not replace an assessment of the
  applicable legal basis under data protection law, particularly for information
  about other people in the address book.
- Article 6(1)(f) GDPR may apply to necessary support, security or error handling.
  Mardu must establish the specific purposes, necessity and balancing of
  interests; possible interests include handling a request and securely operating
  its own services.
- Article 6(1)(c) GDPR requires a specific applicable legal obligation. This draft
  does not invent such obligations or retention periods derived from them.

This notice does not determine the legal bases and obligations of your SIP
provider or an organizational operator.

## 6. Your settings and deletion options

The following options are available in the reviewed app version. German labels
in parentheses identify the controls in the current app.

1. **Contacts and microphone:** Under **Settings → Privacy** (**Einstellungen →
   Datenschutz**), you can check the permission status and open the corresponding
   system settings. There, revoke Ruf's permission under **Privacy & Security →
   Contacts** (**Datenschutz & Sicherheit → Kontakte**) or **Microphone**
   (**Mikrofon**). Without contact access, Ruf removes its contact projection;
   the system address book remains unchanged. Without microphone access, the
   corresponding microphone function is unavailable.
2. **Call history:** Under **Settings → Contacts & History** (**Einstellungen →
   Kontakte & Verlauf**), you can turn off **Save history** (**Verlauf speichern**)
   or change **Retention** (**Aufbewahrung**) and apply the change with **Save
   display & history** (**Anzeige & Verlauf speichern**). This requires
   telephony to be idle. A shorter period that causes deletion must be confirmed.
   In **History** (**Verlauf**), you can remove completed entries using **Delete
   call …** (**Anruf löschen …**) or **Delete history …** (**Verlauf löschen …**)
   after confirmation.
3. **Favorites:** Remove the star from the contact in question. This removes the
   app's favorite association, not a contact from the macOS address book.
4. **Accounts and credentials:** Under **Settings → Accounts** (**Einstellungen →
   Konten**), select the account and **Remove account …** (**Konto entfernen …**).
   Ruf removes the local account configuration and attempts to delete the
   associated saved password from the Keychain. If access is unavailable,
   password deletion remains queued and is reported as pending; it has not yet
   completed. Unlock the Keychain and restart Ruf so cleanup can be retried.
   This does not cancel a SIP account with the telephony provider.
5. **Notifications:** Under **Settings → Notifications** (**Einstellungen →
   Mitteilungen**), you can turn off **Show caller details** (**Anruferdetails
   anzeigen**) or **Show incoming calls** (**Eingehende Anrufe anzeigen**) and
   save the change. The macOS notification settings also control system
   permission.

Removing an account does not automatically delete its past history entries.
Revoking contact permission likewise does not delete SIP call metadata already
stored. Use the separate history functions for this.

Simply moving the app to the Trash **does not guarantee complete data
deletion**. Keychain entries, local app and cache files, a metadata backup that
may have been created during data migration and your own system backups may
remain. Before uninstalling, therefore, remove saved accounts and history
through the app and pay attention to any deletion errors shown. Remaining Ruf
data, test tone files and associated Keychain entries may need to be removed
separately using system tools. Check that they belong to Ruf; this notice does
not recommend indiscriminately deleting unrelated Keychain or system data.

The app's deletion functions do not promise forensic overwriting of storage
media or deletion of third-party backups. Mardu cannot remotely access or delete
local data without your separate involvement.

## 7. Contact and support requests to Mardu

If you contact Mardu yourself, Mardu receives the information you submit, such as
your sender address, message and any diagnostic information you voluntarily
attach. Do not include SIP passwords or unnecessary call and contact data.

Requests made through the Mardu website are covered by its separate privacy
notice. The actual receipt, handling and deletion processes used for Ruf support,
including service providers, legal bases and retention periods, must be confirmed
before publication of this app draft. No specific support retention period is
claimed here.

## 8. Your data protection rights

Subject to the applicable legal conditions, your rights include access,
rectification, erasure, restriction of processing and data portability. Where
processing is based on consent, you may withdraw it with effect for the future;
this does not affect the lawfulness of earlier processing. Where processing is
based on legitimate interests, you may object on grounds relating to your
particular situation. These rights are not unconditional and do not apply to
data that the organization receiving the request does not hold.

For processing by Mardu, contact [info@mardu.de](mailto:info@mardu.de). Requests
about data processed by your SIP provider or another independent service should
also be directed to that provider. The local controls immediately available to
you are described in Section 6.

You may also lodge a complaint with a data protection supervisory authority,
particularly in the place of your habitual residence, workplace or the alleged
infringement. For Mardu's registered office, the
[State Commissioner for Data Protection and Freedom of Information for
Baden-Württemberg](https://www.baden-wuerttemberg.datenschutz.de/) may be a
suitable contact.

The reviewed app version contains no advertising profiling or solely automated
decisions with legal or similarly significant effects. Local phone number
matching serves only to display contacts and does not verify a caller's
identity.

## 9. Assessment for Apple's app privacy disclosures

The App Store category “Data Not Collected” is distinct from the question of
whether personal data is processed at all. Ruf processes data locally and
transmits data needed for calls to your chosen telephony infrastructure.

Under Apple's definition, purely local processing does not count as data
collection for the App Privacy label. The disclosure concerns, in particular,
data the developer or integrated partners can access after transmission for
longer than necessary to serve the real-time request. For the reviewed code
without its own data upload, “Data Not Collected” is a technical draft assessment
**that Mardu must confirm against the final build and its actual services**. A
telephony service operated by Mardu or a partner, additional diagnostic
transmission or an online service enabled later would require a separate
assessment. The App Store disclosure does not replace this privacy notice.

## 10. Internal approval points – address before publication

The following items are editorial instructions and must be resolved before
publication:

- Confirm the provider, allocation of roles under data protection law and privacy
  contact; check whether a data protection officer must be named.
- Review the assumptions in Section 5 and replace them with the legal bases
  actually applicable and specific responsibilities.
- Confirm that neither Mardu nor integrated partners receive additional
  telephony, telemetry, analytics or crash data in the distributed offering.
  Separately review any data obtained from Apple services and its use.
- Confirm Ruf support processes and any service providers, transfers and
  retention periods involved; do not apply website contracts to app processing
  without checking them.
- Check the changelog configuration and availability of development test tone
  mode in the final App Store build. Remove information about its local audio
  file only if this mode is actually inaccessible in the published build.
- Check the documented storage, deletion and permission paths in the final
  signed macOS build, including possible remaining Keychain entries, app
  containers and backups. Do not promise automatic deletion on uninstall.
- Publish a publicly accessible **Ruf-specific privacy notice** or extend the
  existing company page to include this app scope. As of the review date, the
  existing page at `https://www.mardu.de/privacy` describes only the website. The
  URL that actually applies after this must be set consistently in the app and
  App Store Connect; this draft is not yet a public privacy page.
- Align App Privacy answers, app permission text and the final privacy notice
  with the same published build. Then remove draft markings and internal review
  instructions and set an approval date.

## Sources and technical basis for the draft

Public sources, checked on 6 November 2026:

- [Mardu legal notice](https://www.mardu.de/publisher) – provider details and
  public contact; direct HTTPS retrieval succeeded.
- [Mardu website privacy notice](https://www.mardu.de/privacy) – existing website
  scope, no published Ruf-specific version.
- [European Commission: Information for individuals](https://commission.europa.eu/law/law-topic/data-protection/information-individuals_en)
  – privacy information and data subject rights.
- [Baden-Württemberg data protection authority: Data subject rights](https://www.baden-wuerttemberg.datenschutz.de/betroffenenrechte/)
  – conditions for exercising rights and context for complaints.
- [Apple: App privacy details](https://developer.apple.com/app-store/app-privacy-details/)
  – definition of data collection and local processing for the privacy label.
- [Apple: App Review Guidelines, Section 5.1](https://developer.apple.com/app-store/review/guidelines/#privacy)
  – requirements for app-specific privacy notices.

Technical basis in the repository:

- [Product scope](../apps/desktop/PRODUCT.md)
- [Contacts](api/contacts.md), [history](api/history.md) and
  [retention settings](api/preferences.md)
- [Diagnostic report](api/diagnostics.md), [audio tests](api/audio-diagnostics.md)
  and [telephony/test tone mode](api/README.md)
- [External links and changelog](api/about.md)
- `apps/desktop/src-tauri/src/store.rs` and `phone.rs` – account metadata,
  Keychain access and queued password deletion
- `apps/desktop/src-tauri/src/history/database.rs` – local history database
- `apps/desktop/src/lib/components/DataSettings.svelte`,
  `AccountSetup.svelte`, `NotificationSettings.svelte` and
  `PrivacySettings.svelte` – existing controls and deletion options
