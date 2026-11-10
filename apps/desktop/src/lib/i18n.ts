import { createInstance, type ParseKeys } from "i18next";
import { derived, get, writable } from "svelte/store";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import de from "../../../../locales/de.json";
import en from "../../../../locales/en.json";
import type {
  AppLocale,
  LanguagePreference,
  LocalizationState,
} from "./generated/AppInfo";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: "translation";
    resources: { translation: typeof en };
    keySeparator: false;
    nsSeparator: false;
    enableSelector: false;
    strictKeyChecks: true;
    returnNull: false;
  }
}

export type MessageKey = ParseKeys<"translation">;
export type MessageParams = Readonly<Record<string, string | number>>;
export interface Message {
  readonly key: MessageKey;
  readonly params?: MessageParams;
}
export type MessageValue = string | Message;
// Local resources only: persistence/detection remain owned by the native service.
const translations = createInstance();
await translations.init({
  resources: { de: { translation: de }, en: { translation: en } },
  lng: "en",
  supportedLngs: ["de", "en"],
  fallbackLng: "en",
  load: "languageOnly",
  initAsync: false,
  keySeparator: false,
  nsSeparator: false,
  returnNull: false,
  interpolation: {
    prefix: "%{",
    suffix: "}",
    escapeValue: false, // Svelte/DOM and the native adapter escape at the output boundary.
    skipOnVariables: true,
  },
  missingInterpolationHandler: () => {
    throw new Error("Missing translation parameter");
  },
  parseMissingKeyHandler: (key) => {
    throw new Error(`Missing translation: ${key}`);
  },
});

/** Window-local projection of the native authoritative language. */
export const locale = writable<AppLocale>("en");
let revision = -1;
export const getLocale = (): AppLocale => get(locale);

/** Manual selection wins; otherwise negotiate the first supported base language. */
export function resolveLanguage(
  preference: LanguagePreference,
  languages: readonly string[],
): AppLocale {
  if (preference !== "system") return preference;
  const match = translations.services.languageUtils.getBestMatchFromCodes([
    ...languages,
  ]);
  return match &&
    translations.services.languageUtils.getLanguagePartFromCode(match) === "de"
    ? "de"
    : "en";
}

/** Ignore stale initial reads and retain the newest committed language revision. */
export function applyLocalization(state: LocalizationState): void {
  if (state.revision <= revision) return;
  revision = state.revision;
  locale.set(state.locale);
}

/** Store message identity rather than a translation captured before a language change. */
export function message(key: MessageKey, params?: MessageParams): Message {
  return params === undefined ? { key } : { key, params };
}

/** An app-authored failure whose public display stays reactive to the language. */
export class LocalizedError extends Error {
  constructor(public readonly translated: Message) {
    super(formatMessage(getLocale(), translated));
    this.name = "LocalizedError";
  }
}

/** Format plain text only. Callers retain Svelte/DOM escaping for supplied values. */
export function translate(
  language: AppLocale,
  key: MessageKey,
  params: MessageParams = {},
): string {
  return translations.t(key, { ...params, lng: language });
}

export function formatMessage(
  language: AppLocale,
  value: MessageValue | null | undefined,
): string {
  if (value == null) return "";
  return typeof value === "string"
    ? value
    : translate(language, value.key, value.params);
}
export const t = derived(
  locale,
  (language) => (key: MessageKey, params?: MessageParams) =>
    translate(language, key, params),
);
export const text = derived(
  locale,
  (language) => (value: MessageValue | null | undefined) =>
    formatMessage(language, value),
);

/** Refresh after a committed save or window focus; events remain the primary update path. */
export async function refreshLocalization(): Promise<void> {
  if (isTauri())
    applyLocalization(await invoke<LocalizationState>("get_localization"));
}

/** Subscribe before reading so startup replies cannot overwrite a newer save event. */
export async function initializeLocalization(): Promise<void> {
  const browserLanguages =
    typeof navigator === "undefined" ? [] : navigator.languages;
  if (!isTauri()) {
    locale.set(resolveLanguage("system", browserLanguages));
    return;
  }
  let unlisten: UnlistenFn | undefined;
  try {
    unlisten = await listen<LocalizationState>(
      "localization-changed",
      ({ payload }) => applyLocalization(payload),
    );
  } catch {
    /* A read can still initialize a window if event registration fails. */
  }
  try {
    await refreshLocalization();
  } catch {
    if (revision < 0) locale.set(resolveLanguage("system", browserLanguages));
  }
  const focus = () => {
    void refreshLocalization().catch(() => {});
  };
  window.addEventListener("focus", focus);
  window.addEventListener(
    "pagehide",
    () => {
      unlisten?.();
      window.removeEventListener("focus", focus);
    },
    { once: true },
  );
}
