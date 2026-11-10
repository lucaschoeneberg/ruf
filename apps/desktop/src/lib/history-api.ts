import { message } from "./i18n";
import { desktopInvoke } from "./api";
import type {
  EngineSnapshot,
  HistoryPage,
  HistoryQuery,
} from "./generated/AppInfo";

function desktop<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return desktopInvoke<T>(command, args, message("errors.historyOnly"));
}

export const getHistory = (query: HistoryQuery) =>
  desktop<HistoryPage>("get_history", { query });
export const deleteHistoryEntry = (id: string) =>
  desktop<void>("delete_history_entry", { id });
export const clearHistory = () => desktop<void>("clear_history");
export const callbackHistory = (id: string) =>
  desktop<EngineSnapshot>("callback_history", { id });
