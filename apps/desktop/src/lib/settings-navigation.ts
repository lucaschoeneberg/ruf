import type { SettingsSection } from "./api";
export const settingsSections = [
  { id: "general", label: "Allgemein", icon: "settings" },
  { id: "accounts", label: "SIP-Konten", icon: "contacts" },
  { id: "audio", label: "Ton", icon: "audio" },
  { id: "notifications", label: "Mitteilungen", icon: "info" },
  { id: "contacts", label: "Kontakte & Verlauf", icon: "contacts" },
  { id: "privacy", label: "Datenschutz", icon: "info" },
  { id: "diagnostics", label: "Diagnose & Hilfe", icon: "settings" },
  { id: "about", label: "Über Ruf", icon: "info" },
] as const;
export type SettingResult = {
  section: SettingsSection;
  target: string;
  label: string;
  terms: string;
};
const fields: SettingResult[] = [
  {
    section: "general",
    target: "autostart-enabled",
    label: "Bei der Anmeldung starten",
    terms: "autostart login start schließen",
  },
  {
    section: "general",
    target: "start-hidden",
    label: "Bei der Anmeldung im Hintergrund starten",
    terms: "autostart login hintergrund fenster",
  },
  {
    section: "accounts",
    target: "name",
    label: "SIP-Konto bearbeiten",
    terms: "konten konto name verbindung standardkonto",
  },
  {
    section: "accounts",
    target: "registrar",
    label: "SIP-Server",
    terms: "server registrar host domain anmeldung verbinden",
  },
  {
    section: "accounts",
    target: "username",
    label: "Benutzername",
    terms: "anmeldekennung nebenstelle username",
  },
  {
    section: "accounts",
    target: "password",
    label: "Passwort",
    terms: "zugangsdaten schlüsselbund kennwort passwort",
  },
  {
    section: "accounts",
    target: "transport",
    label: "SIP-Transport",
    terms: "udp tcp tls verschlüsselung zertifikat verbindung",
  },
  {
    section: "accounts",
    target: "media-security",
    label: "Audioschutz",
    terms: "srtp sdes sicherheit verschlüsselung tls",
  },
  {
    section: "accounts",
    target: "auth-username",
    label: "Separate Anmeldekennung",
    terms: "auth username digest authentifizierung benutzername",
  },
  {
    section: "accounts",
    target: "outbound-proxy",
    label: "Outbound-Proxy",
    terms: "proxy server routing verbindung",
  },
  {
    section: "audio",
    target: "inputDevice",
    label: "Mikrofon",
    terms: "audio ton eingang",
  },
  {
    section: "audio",
    target: "outputDevice",
    label: "Lautsprecher",
    terms: "audio ton ausgang gespräch",
  },
  {
    section: "audio",
    target: "ringDevice",
    label: "Klingelton-Ausgabe",
    terms: "klingeln gerät ausgabe",
  },
  {
    section: "audio",
    target: "ringtone-enabled",
    label: "Bei Anrufen klingeln",
    terms: "klingelton klingeln ein aus",
  },
  {
    section: "audio",
    target: "ringtone-style",
    label: "Klingelton",
    terms: "klassisch sanft kurz ringtone",
  },
  {
    section: "audio",
    target: "ringtone-volume",
    label: "Klingellautstärke",
    terms: "klingelton lautstärke leise laut volumen",
  },
  {
    section: "audio",
    target: "audio-tests",
    label: "Audiotests",
    terms: "mikrofon testen pegel lautsprecher testen hörprobe",
  },
  {
    section: "notifications",
    target: "notification-permission",
    label: "Mitteilungsfreigabe",
    terms: "benachrichtigungen erlauben permission berechtigung systemfreigabe",
  },
  {
    section: "notifications",
    target: "notifications-enabled",
    label: "Eingehende Anrufe anzeigen",
    terms: "mitteilungen benachrichtigungen aktivieren",
  },
  {
    section: "notifications",
    target: "notification-caller",
    label: "Anruferdetails anzeigen",
    terms: "name rufnummer privatsphäre mitteilungen",
  },
  {
    section: "notifications",
    target: "test-notification",
    label: "Testmitteilung senden",
    terms: "benachrichtigung testen",
  },
  {
    section: "contacts",
    target: "contacts-display",
    label: "Kontaktanzeige",
    terms: "kontakte adressbuch anzeige",
  },
  {
    section: "contacts",
    target: "favorites-first",
    label: "Favoriten zuerst",
    terms: "favoriten sortieren reihenfolge",
  },
  {
    section: "contacts",
    target: "show-organization",
    label: "Firma anzeigen",
    terms: "organisation kontakt firma",
  },
  {
    section: "contacts",
    target: "history-recording",
    label: "Verlauf speichern",
    terms: "historie anrufverlauf aufzeichnen metadaten",
  },
  {
    section: "contacts",
    target: "history-retention",
    label: "Aufbewahrung",
    terms: "verlauf frist unbegrenzt 30 90 365 tage retention",
  },
  {
    section: "contacts",
    target: "clear-history",
    label: "Verlauf löschen",
    terms: "anrufverlauf leeren entfernen löschen",
  },
  {
    section: "privacy",
    target: "privacy-permissions",
    label: "Berechtigungen",
    terms: "mikrofon freigabe datenschutz zugriff schlüsselbund speicherung",
  },
  {
    section: "privacy",
    target: "contacts-permission",
    label: "Kontakte freigeben",
    terms: "macos adressbuch freigabe datenschutz zugriff",
  },
  {
    section: "diagnostics",
    target: "diagnostics-status",
    label: "Verbindung und Hilfe",
    terms: "diagnose fehler verbindung status export tastenkürzel",
  },
  {
    section: "diagnostics",
    target: "developer-audio",
    label: "Audio für Entwicklung",
    terms: "audioquelle testton entwicklung aufzeichnung",
  },
  {
    section: "about",
    target: "about-heading",
    label: "Version und Software",
    terms:
      "über ruf changelog neuerungen änderungen autor mardu firma impressum",
  },
  {
    section: "about",
    target: "software-licenses",
    label: "Software & Lizenzen",
    terms: "open source lizenztexte herkunft",
  },
  {
    section: "about",
    target: "software-inventory",
    label: "Projektinventar (SBOM)",
    terms: "software bill of materials sbom komponenten",
  },
];

export function searchSettings(query: string): SettingResult[] {
  const words = query
    .trim()
    .toLocaleLowerCase("de")
    .split(/\s+/)
    .filter(Boolean);
  if (!words.length) return [];
  return fields.filter((f) =>
    words.every((w) =>
      `${f.label} ${f.terms}`.toLocaleLowerCase("de").includes(w),
    ),
  );
}
