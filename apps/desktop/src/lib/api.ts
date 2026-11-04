import { invoke, isTauri } from "@tauri-apps/api/core";
import type { AppInfo } from "./generated/AppInfo";

export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauri()) {
    throw new Error("Der native Status ist nur in der Desktop-App verfügbar.");
  }
  return invoke<AppInfo>("get_app_info");
}
