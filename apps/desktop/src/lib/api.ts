import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AccountConfig,
  AppInfo,
  EngineSnapshot,
  SoundSettings,
  AudioDevices,
  DesktopSettings,
  AboutLink,
  NotificationPermission,
  MicrophonePermission,
  AudioTestKind,
  AppPreferences,
  DiagnosticsReport,
} from "./generated/AppInfo";

/** Invoke desktop IPC, preserving native errors and rejecting browser calls asynchronously. */
export async function desktopInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
  unavailableMessage = "Der native Status ist nur in der Desktop-App verfügbar.",
): Promise<T> {
  if (!isTauri()) throw new Error(unavailableMessage);
  return invoke<T>(command, args);
}

export async function getAppInfo(): Promise<AppInfo> {
  return desktopInvoke<AppInfo>("get_app_info");
}

export const openAboutLink = (link: AboutLink) =>
  desktopInvoke<void>("open_about_link", { link });

export const getEngineSnapshot = () =>
  desktopInvoke<EngineSnapshot>("get_engine_snapshot");
export const configureAccount = (account: AccountConfig) =>
  desktopInvoke<EngineSnapshot>("configure_account", { account });
export const saveAccount = (
  id: number | null,
  name: string,
  account: AccountConfig,
) => desktopInvoke<EngineSnapshot>("save_account", { id, name, account });
export const removeAccount = (id: number) =>
  desktopInvoke<EngineSnapshot>("remove_account", { id });
export const reconnectAccount = (id: number) =>
  desktopInvoke<EngineSnapshot>("reconnect_account", { id });
export const selectAccount = (id: number) =>
  desktopInvoke<EngineSnapshot>("select_account", { id });
export type SettingsSection =
  | "accounts"
  | "audio"
  | "general"
  | "contacts"
  | "about"
  | "notifications"
  | "privacy"
  | "diagnostics";
export const openSettings = (section?: SettingsSection) =>
  desktopInvoke<void>("open_settings", section ? { section } : undefined);
export const getDesktopSettings = () =>
  desktopInvoke<DesktopSettings>("get_desktop_settings");
export const requestNotificationPermission = () =>
  desktopInvoke<DesktopSettings>("request_notification_permission");
export const quitApp = () => desktopInvoke<void>("quit_app");
export const settingsReady = () => desktopInvoke<void>("settings_ready");
export const saveDesktopSettings = (
  autostartEnabled: boolean,
  notificationsEnabled: boolean,
) =>
  desktopInvoke<DesktopSettings>("save_desktop_settings", {
    autostartEnabled,
    notificationsEnabled,
  });
export const getAudioDevices = () =>
  desktopInvoke<AudioDevices>("get_audio_devices");
export const saveSoundSettings = (sound: SoundSettings) =>
  desktopInvoke<EngineSnapshot>("save_sound_settings", { sound });
export const previewRingtone = (device: string) =>
  desktopInvoke<EngineSnapshot>("preview_ringtone", { device });
export const stopRingtonePreview = () =>
  desktopInvoke<EngineSnapshot>("stop_ringtone_preview");
export const dial = (destination: string) =>
  desktopInvoke<EngineSnapshot>("dial", { destination });
export const answer = (callId: string) =>
  desktopInvoke<EngineSnapshot>("answer", { callId });
export const reject = (callId: string) =>
  desktopInvoke<EngineSnapshot>("reject", { callId });
export const hangup = (callId: string) =>
  desktopInvoke<EngineSnapshot>("hangup", { callId });

export const setCallMuted = (callId: string, muted: boolean) =>
  desktopInvoke<EngineSnapshot>("set_call_muted", { callId, muted });
export const setCallHeld = (callId: string, held: boolean) =>
  desktopInvoke<EngineSnapshot>("set_call_held", { callId, held });
export const transferCall = (callId: string, destination: string) =>
  desktopInvoke<EngineSnapshot>("transfer_call", { callId, destination });
export const startConsultation = (callId: string, destination: string) =>
  desktopInvoke<EngineSnapshot>("start_consultation", { callId, destination });
export const switchCall = (callId: string) =>
  desktopInvoke<EngineSnapshot>("switch_call", { callId });
export const cancelConsultation = (callId: string) =>
  desktopInvoke<EngineSnapshot>("cancel_consultation", { callId });
export const completeTransfer = (callId: string) =>
  desktopInvoke<EngineSnapshot>("complete_transfer", { callId });

export function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "code" in error) {
    switch (error.code) {
      case "invalidInput":
        return "Bitte prüfe deine Eingaben und versuche es erneut.";
      case "notRegistered":
        return "Dein Konto ist nicht verbunden. Prüfe es in den Kontoeinstellungen.";
      case "busy":
        return "Die Telefonie ist gerade belegt. Beende ein laufendes Gespräch oder versuche es erneut.";
      case "invalidState":
        return "Die Aktion passt nicht zum aktuellen Gesprächsstatus. Prüfe den Status und versuche es erneut.";
      case "engineUnavailable":
        return "Telefonieren ist hier nicht verfügbar. Nutze die Desktop-App mit nativer Telefonie.";
      case "credentialAccessRequired":
        return "Ruf benötigt Zugriff auf das gespeicherte Passwort im System-Schlüsselbund. Wähle „Erneut verbinden“ und bestätige bei Bedarf die Systemabfrage.";
      case "credentialMissing":
        return "Für dieses Konto ist kein Passwort im System-Schlüsselbund gespeichert. Gib das Passwort ein und wähle „Speichern & verbinden“.";
      case "credentialFailure":
        return "Das gespeicherte Passwort im System-Schlüsselbund konnte nicht verwendet werden. Wähle „Erneut verbinden“ oder gib das Passwort erneut ein und speichere das Konto.";
      case "credentialCleanupPending":
        return "Die Kontoänderung ist gespeichert. Ein altes Passwort konnte noch nicht aus dem Schlüsselbund gelöscht werden. Erlaube den Zugriff und starte die App erneut, damit die Löschung wiederholt wird.";
      case "storageFailure":
        return "Die Einstellungen konnten nicht gespeichert oder geladen werden. Prüfe den Zugriff auf den App-Datenordner und versuche es erneut.";
      case "certificateRejected":
        return "Das TLS-Zertifikat oder der Servername wurde nicht bestätigt. Prüfe SIP-Server und Proxy. Private Telefonanlagen-Zertifikate müssen bewusst in den Vertrauenseinstellungen des Betriebssystems freigegeben sein.";
      case "transportFailure":
        return "Der gewählte SIP-Transport konnte nicht sicher verbunden werden. Prüfe Server, Port und Transport beim Anbieter. Ruf wechselt nicht automatisch auf eine unverschlüsselte Verbindung.";
      case "mediaSecurityRequired":
        return "Die erforderliche SRTP-Audioverschlüsselung konnte nicht bestätigt werden. Prüfe TLS und die SRTP-Unterstützung der Telefonanlage. Ruf lässt kein unverschlüsseltes Audio zu.";
      case "sipRedirect":
        return "Der Anruf soll umgeleitet werden. Ruf wählt das neue Ziel nicht automatisch. Prüfe die Zielnummer.";
      case "sipAuthenticationFailed":
        return "Die SIP-Anmeldung wurde abgelehnt. Prüfe die Zugangsdaten in den Kontoeinstellungen.";
      case "sipForbidden":
        return "Diese SIP-Aktion ist für dein Konto nicht erlaubt. Prüfe die Freigaben beim Anbieter.";
      case "sipNotFound":
        return "Die Rufnummer oder das SIP-Konto wurde nicht gefunden. Prüfe die Angaben.";
      case "sipTimeout":
        return "Die SIP-Gegenstelle hat nicht rechtzeitig geantwortet. Versuche es erneut.";
      case "sipUnavailable":
        return "Die SIP-Gegenstelle ist gerade nicht erreichbar. Versuche es später erneut.";
      case "sipBusy":
        return "Die Gegenstelle ist besetzt. Versuche es später erneut.";
      case "sipNotAcceptable":
        return "Die Gegenstelle akzeptiert die angebotenen Audioformate oder Sicherheitseinstellungen nicht. Prüfe Audio, TLS und SRTP beim SIP-Anbieter.";
      case "sipRejected":
        return "Der Anruf wurde von der Gegenstelle abgelehnt.";
      case "sipFailure":
        return "Die SIP-Verbindung ist fehlgeschlagen. Prüfe die Verbindung und versuche es erneut.";
      case "nativeFailure":
        return "Die Telefonie-Aktion ist fehlgeschlagen. Prüfe die Verbindung und versuche es erneut.";
    }
  }
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return "Die Telefonie-Aktion konnte nicht abgeschlossen werden.";
}

/** Exact CycloneDX JSON embedded by the selected desktop build. */
export const getSoftwareBill = () => desktopInvoke<string>("get_software_bill");

/** Read OS authorization independently of accounts, saved metadata and SIP state. */
export const getNotificationPermission = () =>
  desktopInvoke<NotificationPermission>("get_notification_permission");
export const openNotificationSettings = () =>
  desktopInvoke<void>("open_notification_settings");
export const testNotification = () => desktopInvoke<void>("test_notification");
export const getMicrophonePermission = () =>
  desktopInvoke<MicrophonePermission>("get_microphone_permission");
export const requestMicrophonePermission = () =>
  desktopInvoke<MicrophonePermission>("request_microphone_permission");
export const openMicrophoneSettings = () =>
  desktopInvoke<void>("open_microphone_settings");
export const startAudioTest = (kind: AudioTestKind, device: string) =>
  desktopInvoke<string>("start_audio_test", { kind, device });
export const stopAudioTest = (id: string) =>
  desktopInvoke<void>("stop_audio_test", { id });
export const getPreferences = () =>
  desktopInvoke<AppPreferences>("get_preferences");
export const savePreferences = (
  preferences: AppPreferences,
  confirmHistoryDeletion = false,
) =>
  desktopInvoke<AppPreferences>("save_preferences", {
    preferences,
    confirmHistoryDeletion,
  });
export const getDiagnostics = () =>
  desktopInvoke<DiagnosticsReport>("get_diagnostics");
export const copyDiagnostics = () => desktopInvoke<void>("copy_diagnostics");
export const setAccountEnabled = (id: number, enabled: boolean) =>
  desktopInvoke<EngineSnapshot>("set_account_enabled", { id, enabled });

/** Queue a single native telephone tone for an opaque current call token. */
export const sendDtmf = (callId: string, digit: string) =>
  desktopInvoke<EngineSnapshot>("send_dtmf", { callId, digit });
