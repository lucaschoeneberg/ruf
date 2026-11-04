import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AccountConfig,
  AppInfo,
  EngineSnapshot,
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
export const dial = (destination: string) =>
  desktopInvoke<EngineSnapshot>("dial", { destination });
export const answer = () => desktopInvoke<EngineSnapshot>("answer");
export const hangup = () => desktopInvoke<EngineSnapshot>("hangup");

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
