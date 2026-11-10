import { LocalizedError, message, type Message } from "./i18n";
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
  unavailableMessage: Message = message("errors.desktopOnly"),
): Promise<T> {
  if (!isTauri()) throw new LocalizedError(unavailableMessage);
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

export function errorMessage(error: unknown): Message {
  if (error instanceof LocalizedError) return error.translated;
  if (typeof error === "object" && error !== null && "code" in error) {
    switch (error.code) {
      case "invalidInput":
        return message("errors.invalidInput");
      case "notRegistered":
        return message("errors.notRegistered");
      case "busy":
        return message("errors.busy");
      case "invalidState":
        return message("errors.invalidState");
      case "engineUnavailable":
        return message("errors.engineUnavailable");
      case "credentialAccessRequired":
        return message("errors.credentialAccessRequired");
      case "credentialMissing":
        return message("errors.credentialMissing");
      case "credentialFailure":
        return message("errors.credentialFailure");
      case "credentialCleanupPending":
        return message("errors.credentialCleanupPending");
      case "storageFailure":
        return message("errors.storageFailure");
      case "certificateRejected":
        return message("errors.certificateRejected");
      case "transportFailure":
        return message("errors.transportFailure");
      case "mediaSecurityRequired":
        return message("errors.mediaSecurityRequired");
      case "sipRedirect":
        return message("errors.sipRedirect");
      case "sipAuthenticationFailed":
        return message("errors.sipAuthenticationFailed");
      case "sipForbidden":
        return message("errors.sipForbidden");
      case "sipNotFound":
        return message("errors.sipNotFound");
      case "sipTimeout":
        return message("errors.sipTimeout");
      case "sipUnavailable":
        return message("errors.sipUnavailable");
      case "sipBusy":
        return message("errors.sipBusy");
      case "sipNotAcceptable":
        return message("errors.sipNotAcceptable");
      case "sipRejected":
        return message("errors.sipRejected");
      case "sipFailure":
        return message("errors.sipFailure");
      case "nativeFailure":
        return message("errors.nativeFailure");
    }
  }
  return message("errors.unknown");
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
