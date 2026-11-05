import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AccountConfig,
  AppInfo,
  EngineSnapshot,
  SoundSettings,
  AudioDevices,
  DesktopSettings,
} from "./generated/AppInfo";

async function desktopInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    throw new Error("Der native Status ist nur in der Desktop-App verfügbar.");
  return invoke<T>(command, args);
}

export async function getAppInfo(): Promise<AppInfo> {
  return desktopInvoke<AppInfo>("get_app_info");
}

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
export const selectAccount = (id: number) =>
  desktopInvoke<EngineSnapshot>("select_account", { id });
export type SettingsSection =
  "accounts" | "audio" | "general" | "contacts" | "about";
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
      case "credentialCleanupPending":
        return "Die Kontoänderung ist gespeichert. Ein altes Passwort konnte noch nicht aus dem Schlüsselbund gelöscht werden. Erlaube den Zugriff und starte die App erneut, damit die Löschung wiederholt wird.";
      case "storageFailure":
        return "Die Einstellungen konnten nicht gespeichert oder geladen werden. Prüfe den Zugriff auf den System-Schlüsselbund und den App-Datenordner. Versuche es danach erneut.";
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
        return "Die Gegenstelle unterstützt die angebotenen Audioformate nicht. Prüfe die Audioeinstellungen oder den SIP-Anbieter.";
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
