import type { Page } from "playwright-core";
import type {
  AppPreferences,
  LocalizationState,
} from "../../../apps/desktop/src/lib/generated/AppInfo";
import type { FixtureArgs, FixtureSnapshot } from "./types";

declare global {
  interface Window {
    __rufLocaleFixture: {
      invoke(command: string, args: FixtureArgs): Promise<unknown>;
      emit(event: string, payload: unknown): void;
      rememberedPeer: Element | null;
      changelogAborted: boolean;
      changelogRequests: string[];
      releaseChangelog: (() => void) | null;
      failGermanChangelog: boolean;
    };
    __rufLocaleInvoke(command: string, args: FixtureArgs): Promise<unknown>;
  }
}

// Native-shaped authoritative persistence shared by independent real pages. No
// SIP, credentials, personal contacts, OS settings or system language is touched.
export default async function localizationRegressions(
  page: Page,
  baseUrl: string,
) {
  const context = page.context();
  const main = await context.newPage();
  const settings = await context.newPage();
  const observer = await context.newPage();
  const accountDraft = await context.newPage();
  const windows = [main, settings, observer, accountDraft];
  const errors: string[] = [];
  for (const target of windows) {
    target.setDefaultTimeout(10000);
    target.on("pageerror", (error) => errors.push(error.message));
  }
  let preferences: AppPreferences = {
    language: "system",
    startHidden: true,
    showCallerInNotifications: true,
    contacts: {
      favoritesFirst: true,
      showOrganization: true,
      matchGermanNumbers: true,
    },
    history: { recordingEnabled: true, retentionDays: null },
  };
  let localization: LocalizationState = {
    preference: "system",
    locale: "de",
    revision: 1,
  };
  const snapshot: FixtureSnapshot = {
    revision: 1,
    registration: "registered",
    call: "idle",
    callId: null,
    calls: [],
    caller: null,
    callerContact: null,
    lastCallFailure: null,
    historyError: null,
    lastError: null,
    callConnectedAtMs: null,
    callEvents: [],
    callEventsDropped: 0,
    accounts: [
      {
        id: 1,
        name: "Büro Original",
        username: "1000",
        registrar: "pbx.example.test",
        transport: "tls",
        mediaSecurity: "required",
        authUsername: null,
        outboundProxy: null,
        registration: "registered",
        lastError: null,
        removing: false,
        enabled: true,
        enabledChanging: false,
        registeredAtMs: 1000,
      },
    ],
    selectedAccountId: 1,
    callAccountId: null,
    audioMode: "system",
    soundSettings: {
      inputDevice: "",
      outputDevice: "",
      ringDevice: "",
      ringtoneEnabled: true,
      ringtone: "classic",
      ringtoneVolume: 100,
    },
    ringtonePlaying: false,
  };
  const commands: string[] = [];
  let holdSave = false;
  let saveStarted = false;
  let releaseSave: (() => void) | null = null;
  let failSave = false;
  const broadcast = async (event: string, payload: unknown) => {
    for (const target of windows)
      if (!target.isClosed() && target.url().startsWith(baseUrl))
        await target.evaluate(
          ({ event, payload }) =>
            window.__rufLocaleFixture.emit(event, payload),
          { event, payload },
        );
  };
  await context.exposeBinding(
    "__rufLocaleInvoke",
    async (_source, command: string, args: FixtureArgs) => {
      commands.push(command);
      switch (command) {
        case "get_localization":
          return structuredClone(localization);
        case "get_preferences":
          return structuredClone(preferences);
        case "save_preferences": {
          saveStarted = true;
          if (holdSave)
            await new Promise<void>((resolve) => {
              releaseSave = resolve;
            });
          if (failSave)
            throw {
              code: "storageFailure",
              message: "Synthetic localization persistence failure",
              sipStatus: null,
            };
          if (!args.preferences) throw new Error("Missing preferences");
          preferences = structuredClone(args.preferences);
          localization = {
            preference: preferences.language,
            locale:
              preferences.language === "system" ? "de" : preferences.language,
            revision: localization.revision + 1,
          };
          await broadcast("localization-changed", localization);
          return structuredClone(preferences);
        }
        case "get_app_info":
          return { version: "0.1.0", telephony: "linked" };
        case "get_engine_snapshot":
          return structuredClone(snapshot);
        case "get_desktop_settings":
          return {
            autostartEnabled: false,
            autostartAvailable: true,
            notificationsEnabled: true,
            notificationPermission: "authorized",
            notificationsGranted: true,
          };
        case "get_audio_devices":
          return { inputs: [], outputs: [] };
        case "get_notification_permission":
        case "get_microphone_permission":
        case "get_contacts_permission":
          return "authorized";
        case "get_contacts":
        case "refresh_contacts":
          return { permission: "authorized", contacts: [] };
        case "settings_ready":
        case "plugin:window|destroy":
          return null;
        default:
          throw new Error(
            `Unexpected localization fixture command: ${command}`,
          );
      }
    },
  );
  for (const target of windows)
    await target.addInitScript(() => {
      window.isTauri = true;
      localStorage.setItem("sip.introduction.v1", "seen");
      const callbacks = new Map<
        number,
        (event: { event: string; payload: unknown }) => void
      >();
      const listeners = new Map<number, { event: string; handler: number }>();
      let next = 1;
      window.__rufLocaleFixture = {
        rememberedPeer: null,
        changelogAborted: false,
        changelogRequests: [],
        releaseChangelog: null,
        failGermanChangelog: false,
        invoke: (command, args) => window.__rufLocaleInvoke(command, args),
        emit(event, payload) {
          for (const listener of listeners.values())
            if (listener.event === event)
              callbacks.get(listener.handler)?.({ event, payload });
        },
      };
      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: {
            label: location.search.includes("window=settings")
              ? "settings"
              : "main",
          },
          currentWebview: {
            label: location.search.includes("window=settings")
              ? "settings"
              : "main",
          },
        },
        transformCallback(callback) {
          const id = next++;
          callbacks.set(id, callback);
          return id;
        },
        unregisterCallback(id) {
          callbacks.delete(id);
        },
        async invoke(command, args) {
          if (command === "plugin:event|listen") {
            const id = next++;
            listeners.set(id, { event: args.event!, handler: args.handler! });
            return id;
          }
          if (command === "plugin:event|unlisten") {
            listeners.delete(Number(args.id));
            return;
          }
          const result = await window.__rufLocaleFixture.invoke(command, args);
          return command === "get_engine_snapshot"
            ? window.__rufTestSyncDialogs(result as FixtureSnapshot)
            : result;
        },
      };
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener(_event, id) {
          listeners.delete(id);
        },
      };
      const originalFetch = window.fetch.bind(window);
      Object.defineProperty(window, "fetch", {
        configurable: true,
        value: (input: RequestInfo | URL, init?: RequestInit) => {
          const url =
            typeof input === "string"
              ? input
              : input instanceof URL
                ? input.href
                : input.url;
          if (
            !url.startsWith(
              "https://raw.githubusercontent.com/example/ruf/main/CHANGELOG",
            )
          )
            return originalFetch(input, init);
          const fixture = window.__rufLocaleFixture;
          fixture.changelogRequests.push(url);
          if (url.endsWith("CHANGELOG.de.md")) {
            if (fixture.failGermanChangelog)
              return Promise.reject(
                new Error("Synthetic German changelog failure"),
              );
            // Deliberately resolve after abort to exercise the stale-success guard.
            return new Promise<Response>((resolve) => {
              init?.signal?.addEventListener("abort", () => {
                fixture.changelogAborted = true;
              });
              fixture.releaseChangelog = () =>
                resolve(
                  new Response(
                    "## 9.9.9 · Veraltet\n\n- Alte deutsche Antwort.\n",
                  ),
                );
            });
          }
          return Promise.resolve(
            new Response(
              "## 0.2.0 · English update\n\n- English online changes.\n",
            ),
          );
        },
      });
    });
  const assert = (condition: unknown, reason: string) => {
    if (!condition) throw new Error(reason);
  };
  const wait = async (condition: () => boolean) => {
    const deadline = Date.now() + 10000;
    while (!condition()) {
      if (Date.now() >= deadline)
        throw new Error("Timed out waiting for save fixture");
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  };
  try {
    await main.goto(baseUrl);
    const number = main.getByRole("textbox", {
      name: "Nummer oder Nebenstelle",
      exact: true,
    });
    await number.fill("314159");
    await settings.goto(`${baseUrl}/?window=settings&section=general`);
    await observer.goto(`${baseUrl}/?window=settings&section=about`);
    await accountDraft.goto(`${baseUrl}/?window=settings&account=1`);
    const draft = accountDraft.getByLabel("Name", { exact: true });
    await draft.fill("Entwurf <unverändert>");
    await observer.getByText("Was ist neu?", { exact: true }).click();
    await observer.waitForFunction(
      () => window.__rufLocaleFixture.releaseChangelog !== null,
    );
    await settings.getByLabel("Sprache", { exact: true }).selectOption("en");
    assert(
      (await settings.locator("html").getAttribute("lang")) === "de",
      "Draft language was applied before saving",
    );
    assert(
      (await main.locator("html").getAttribute("lang")) === "de",
      "Main window applied an unsaved language",
    );
    await settings
      .getByRole("button", { name: "Verwerfen", exact: true })
      .click();
    assert(
      (await settings.getByLabel("Sprache", { exact: true }).inputValue()) ===
        "system",
      "Discard did not restore system preference",
    );
    assert(
      !commands.includes("save_preferences"),
      "Discard persisted a language change",
    );

    // Keep a real-shaped live call and its visible DOM identity through the save.
    snapshot.call = "active";
    snapshot.callId = "retained-call";
    snapshot.callAccountId = 1;
    snapshot.callConnectedAtMs = Date.now() - 60000;
    snapshot.caller = { name: "Anrufer <Original>", number: "+4930123456" };
    snapshot.revision++;
    await main.evaluate(
      (value) =>
        window.__rufLocaleFixture.emit(
          "phone-state-changed",
          window.__rufTestSyncDialogs(value),
        ),
      snapshot,
    );
    await main
      .getByRole("heading", { name: "Anrufer <Original>", exact: true })
      .waitFor();
    await main.evaluate(() => {
      window.__rufLocaleFixture.rememberedPeer =
        document.querySelector(".call-peer");
    });
    // Exercise a poll explicitly: fast local runs must not hide inconsistent
    // fixture reads between the active-call event and a language save.
    const pollCall = () =>
      main.evaluate(async () => {
        const value = (await window.__TAURI_INTERNALS__.invoke(
          "get_engine_snapshot",
          {},
        )) as FixtureSnapshot;
        window.__rufLocaleFixture.emit("phone-state-changed", value);
        return value.calls?.some((call) => call.id === value.callId) ?? false;
      });
    assert(await pollCall(), "Polling lost the active call fixture");
    await settings.getByLabel("Sprache", { exact: true }).selectOption("en");
    holdSave = true;
    saveStarted = false;
    await settings
      .getByRole("button", { name: "Allgemein speichern", exact: true })
      .click();
    await wait(() => saveStarted && releaseSave !== null);
    assert(
      (await main.locator("html").getAttribute("lang")) === "de",
      "Language applied before persistence succeeded",
    );
    holdSave = false;
    releaseSave!();
    await Promise.all(
      windows.map((target) =>
        target.waitForFunction(() => document.documentElement.lang === "en"),
      ),
    );
    await settings.getByText("Settings saved.", { exact: true }).waitFor();
    assert(await pollCall(), "Polling lost the active call after localization");
    assert(
      (await draft.inputValue()) === "Entwurf <unverändert>",
      "Locale event reset an account draft",
    );
    assert(
      await main.evaluate(
        () =>
          window.__rufLocaleFixture.rememberedPeer ===
          document.querySelector(".call-peer"),
      ),
      "Language change remounted the active call",
    );
    await main
      .getByRole("heading", { name: "Anrufer <Original>", exact: true })
      .waitFor();
    await main.getByLabel("Call duration", { exact: true }).waitFor();
    await observer
      .getByRole("heading", { name: "0.2.0 · English update", exact: true })
      .waitFor();
    assert(
      await observer.evaluate(() => window.__rufLocaleFixture.changelogAborted),
      "Locale change did not abort the old changelog request",
    );
    await observer.evaluate(() =>
      window.__rufLocaleFixture.releaseChangelog!(),
    );
    await observer.waitForFunction(
      () =>
        !document.querySelector(".changelog")!.textContent.includes("9.9.9"),
    );
    assert(
      await observer
        .getByRole("heading", { name: "0.2.0 · English update", exact: true })
        .isVisible(),
      "Late German response replaced English changelog",
    );

    failSave = true;
    await settings.getByLabel("Language", { exact: true }).selectOption("de");
    await settings
      .getByRole("button", { name: "Save general settings", exact: true })
      .click();
    await settings.getByRole("alert").waitFor();
    assert(
      (await main.locator("html").getAttribute("lang")) === "en",
      "Failed save changed the committed language",
    );
    assert(
      localization.locale === "en" && preferences.language === "en",
      "Failed save mutated authoritative language",
    );
    assert(
      (await settings.getByLabel("Language", { exact: true }).inputValue()) ===
        "de",
      "Failed save discarded the language draft",
    );
    failSave = false;
    await observer.evaluate(() => {
      window.__rufLocaleFixture.failGermanChangelog = true;
    });
    await settings
      .getByRole("button", { name: "Save general settings", exact: true })
      .click();
    await Promise.all(
      windows.map((target) =>
        target.waitForFunction(() => document.documentElement.lang === "de"),
      ),
    );
    await observer
      .getByRole("heading", { name: "0.1.0 · Vorschau", exact: true })
      .waitFor();
    await observer
      .getByText("Online-Änderungen konnten nicht geladen werden.", {
        exact: false,
      })
      .waitFor();
    assert(
      (await observer
        .getByRole("heading", { name: "0.2.0 · English update", exact: true })
        .count()) === 0,
      "German failure retained an English changelog",
    );
    assert(
      (await draft.inputValue()) === "Entwurf <unverändert>",
      "Second locale event reset an account draft",
    );
    assert(
      !commands.some((command) =>
        ["dial", "hangup", "answer", "reject", "save_account"].includes(
          command,
        ),
      ),
      "Localization invoked a call or account mutation",
    );
    // End only the synthetic call to verify the hidden dial draft after both switches.
    snapshot.call = "idle";
    snapshot.callId = null;
    snapshot.caller = null;
    snapshot.callAccountId = null;
    snapshot.callConnectedAtMs = null;
    snapshot.revision++;
    await main.evaluate(
      (value) =>
        window.__rufLocaleFixture.emit(
          "phone-state-changed",
          window.__rufTestSyncDialogs(value),
        ),
      snapshot,
    );
    assert(
      (await number.inputValue()) === "314159",
      "Locale changes lost the dial draft",
    );
    assert(!errors.length, errors.join("\n"));
    return { passed: true };
  } finally {
    for (const target of windows) await target.close();
  }
}
