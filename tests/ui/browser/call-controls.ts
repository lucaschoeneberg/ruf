import type { EnglishSmoke } from "./english-smoke";
import type { Page } from "playwright-core";
import type {
  FixtureSnapshot,
  FixtureAccount,
  FixtureCallback,
  FixtureListener,
  FixtureCommand,
  FixtureCall,
} from "./types";
// Native-shaped synthetic IPC: these regressions never contact SIP/audio/keychains.
export default async function callControls(
  page: Page,
  baseUrl: string,
  capture = false,
  englishSmoke?: EnglishSmoke,
) {
  const target = await page.context().newPage();
  target.setDefaultTimeout(10000);
  const errors: string[] = [];
  target.on("pageerror", (error) => errors.push(error.message));
  await target.addInitScript(() => {
    window.isTauri = true;
    localStorage.setItem("sip.introduction.v1", "seen");
    const callbacks = new Map<number, FixtureCallback>(),
      listeners = new Map<number, FixtureListener>();
    let sequence = 1;
    const account = (id: number, name: string): FixtureAccount => ({
      id,
      name,
      registrar: "pbx.example.test",
      username: String(id),
      transport: "tls",
      mediaSecurity: "required",
      authUsername: "digest",
      outboundProxy: "proxy.example.test:5061",
      enabled: true,
      enabledChanging: false,
      registeredAtMs: 1000,
      registration: "registered",
      lastError: null,
      removing: false,
    });
    const dialog = (
      id: string,
      overrides: Partial<FixtureCall> = {},
    ): FixtureCall => ({
      id,
      accountId: 2,
      state: "active",
      destination: "9002",
      caller: null,
      connectedAtMs: Date.now() - 60000,
      muted: false,
      localHeld: false,
      remoteHeld: false,
      holdPending: false,
      transfer: "idle",
      consultation: false,
      events: [],
      eventsDropped: 0,
      lastError: null,
      security: {
        signalingTransport: null,
        tlsPeerVerified: false,
        media: "unknown",
      },
      ...overrides,
    });
    const settings = location.search.includes("window=settings");
    const state: FixtureSnapshot & { calls: FixtureCall[] } = {
      revision: 1,
      registration: "registered",
      call: "idle",
      callId: null,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      lastCallFailure: null,
      calls: settings ? [] : [dialog("parent")],
      accounts: [account(1, "Privat"), account(2, "Büro")],
      selectedAccountId: settings ? 2 : 1,
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
      audioTest: null,
      audioTestError: null,
    };
    const fixture: Window["__controls"] = {
      state,
      commands: [] as FixtureCommand[],
      contactsPermission: "notDetermined",
      fail: null,
      dialog,
      project(id = state.callId) {
        const current =
          state.calls.find((item) => item.id === id) ?? state.calls[0]!;
        state.call = current?.state ?? "idle";
        state.callId = current?.id ?? null;
        state.callAccountId = current?.accountId ?? null;
        state.callConnectedAtMs = current?.connectedAtMs ?? null;
        state.caller = current?.caller ?? null;
        state.lastError = current?.lastError ?? null;
        if (current) state.callEvents = current.events;
      },
      emit(id = state.callId) {
        this.project(id);
        state.revision++;
        for (const [key, listener] of listeners)
          if (listener.event === "phone-state-changed")
            callbacks.get(listener.handler)?.({
              id: key,
              event: listener.event,
              payload: structuredClone(state),
            });
      },
      request(action, payload = null) {
        for (const [key, listener] of listeners)
          if (listener.event === `phone-${action}-requested`)
            callbacks.get(listener.handler)?.({
              id: key,
              event: listener.event,
              payload,
            });
      },
      contactsChanged() {
        for (const [key, listener] of listeners)
          if (listener.event === "contacts-changed")
            callbacks.get(listener.handler)?.({
              id: key,
              event: listener.event,
              payload: null,
            });
      },
    };
    if (location.search.includes("retained-failure"))
      state.lastCallFailure = {
        callId: "ended-background",
        accountId: 1,
        error: {
          code: "mediaSecurityRequired",
          sipStatus: null,
          message: "safe",
        },
      };
    fixture.project("parent");
    window.__controls = fixture;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event: string, id: number) {
        listeners.delete(id);
      },
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: settings ? "settings" : "main" },
        currentWebview: { label: settings ? "settings" : "main" },
      },
      transformCallback(fn) {
        const id = sequence++;
        callbacks.set(id, fn);
        return id;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args = {}) {
        if (command === "get_localization")
          return {
            preference: "system",
            locale: window.__rufTestLanguage,
            revision: 0,
          };
        fixture.commands.push({ command, args: structuredClone(args) });
        if (command === "plugin:event|listen") {
          const id = sequence++;
          listeners.set(id, { event: args.event!, handler: args.handler! });
          return id;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot") {
          state.revision++;
          return structuredClone(state);
        }
        if (command === "get_history")
          return {
            entries: [],
            total: 0,
            hasMore: false,
            recordingError: null,
          };
        if (command === "get_preferences")
          return {
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
        if (command === "get_desktop_settings")
          return {
            autostartEnabled: false,
            autostartAvailable: true,
            notificationsEnabled: false,
            notificationPermission: "authorized",
            notificationsGranted: true,
          };
        if (command === "get_notification_permission") return "authorized";
        if (command === "settings_ready" || command === "stop_ringtone_preview")
          return structuredClone(state);
        if (command === "get_audio_devices") return { inputs: [], outputs: [] };
        if (command === "get_contacts")
          return {
            permission: fixture.contactsPermission,
            contacts:
              fixture.contactsPermission === "authorized"
                ? [
                    {
                      id: "contact",
                      source: "macos",
                      name: "Ada Beispiel",
                      organization: "Test",
                      favorite: true,
                      phones: [
                        {
                          label: "Büro",
                          number: "123 456",
                          destination: "123456",
                        },
                        { label: "Privat", number: "678", destination: "678" },
                      ],
                    },
                  ]
                : [],
          };
        if (command === "reconnect_account") {
          state.accounts.find((item) => item.id === args.id)!.registration =
            "registering";
          state.revision++;
          return structuredClone(state);
        }
        if (command === "save_account") {
          const saved = state.accounts.find((item) => item.id === args.id);
          const {
            password: _secret,
            audioMode: _mode,
            ...safe
          } = args.account!;
          if (saved) Object.assign(saved, safe, { name: args.name });
          else state.accounts.push({ ...account(3, args.name!), ...safe });
          state.revision++;
          return structuredClone(state);
        }
        const controls = [
          "set_call_muted",
          "set_call_held",
          "transfer_call",
          "start_consultation",
          "switch_call",
          "cancel_consultation",
          "complete_transfer",
          "hangup",
        ];
        if (controls.includes(command)) {
          const call = state.calls.find((item) => item.id === args.callId);
          if (!call)
            throw {
              code: "invalidState",
              message: "Synthetic stale token",
              sipStatus: null,
            };
          if (
            ["cancel_consultation", "complete_transfer"].includes(command) &&
            !call.consultation
          )
            throw {
              code: "invalidState",
              message: "Synthetic wrong child",
              sipStatus: null,
            };
          if (fixture.fail === command)
            throw {
              code: "nativeFailure",
              message: "Synthetic operation failure",
              sipStatus: null,
            };
          // Accepted queue intentionally does not change confirmed state.
          state.revision++;
          return structuredClone(state);
        }
        throw new Error("Unexpected controls IPC " + command);
      },
    };
  });
  if (englishSmoke) return englishSmoke(target, "call-controls", baseUrl);
  const assert = (value: unknown, message: string) => {
    if (!value) throw new Error(message);
  };
  const command = async (name: string) =>
    target.evaluate(
      (name) =>
        window.__controls.commands
          .filter((item) => item.command === name)
          .at(-1)!,
      name,
    );
  const emit = async (changes: Partial<FixtureCall>) =>
    target.evaluate((changes) => {
      Object.assign(window.__controls.state.calls[0]!, changes);
      window.__controls.emit();
    }, changes);
  await target.setViewportSize({ width: 340, height: 500 });
  await target.goto(baseUrl);
  await target.getByRole("heading", { name: "9002", exact: true }).waitFor();
  await target.evaluate(() => window.__controls.request("mute", "old-call"));
  assert(
    !(await command("set_call_muted")),
    "Stale menu token cannot mute the current call",
  );
  await target.evaluate(() => window.__controls.request("transfer", "parent"));
  await target.getByLabel("Zielnummer", { exact: true }).waitFor();
  assert(
    await target
      .getByLabel("Zielnummer", { exact: true })
      .evaluate((el) => el === document.activeElement),
    "Transfer shortcut focuses destination",
  );
  await target
    .getByRole("button", { name: "Vermittlungsziel schließen", exact: true })
    .click();
  await emit({ holdPending: true });
  await target.evaluate(() => window.__controls.request("hold", "parent"));
  assert(
    !(await command("set_call_held")),
    "Menu respects native hold recovery",
  );
  await emit({ holdPending: false });
  await target.evaluate(() => {
    window.__controls.contactsPermission = "authorized";
    window.__controls.request("find");
  });
  await target.locator(".contact-search input").waitFor();
  await target.waitForFunction(
    () =>
      document.querySelector(".contact-search input") ===
      document.activeElement,
  );
  await target.evaluate(() => {
    window.__controls.request("dialer");
    window.__controls.contactsPermission = "notDetermined";
    window.__controls.contactsChanged();
  });
  // Search and shortcuts must share the visible controls' native-state guards.
  const searchButton = target.getByRole("button", {
    name: "Ruf durchsuchen",
    exact: true,
  });
  await searchButton.click();
  const search = target.getByRole("dialog", {
    name: "Ruf durchsuchen",
  });
  if (capture)
    await target.screenshot({ path: "output/playwright/actions-340.png" });
  await target.setViewportSize({ width: 320, height: 440 });
  assert(
    !(await target.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    )),
    "Compact search has no horizontal overflow",
  );
  await search.getByRole("combobox").fill("xyz-unbekannt");
  await search.getByText(/Keine passenden Treffer/).waitFor();
  await search.getByRole("combobox").press("ArrowDown");
  await search.getByRole("combobox").press("Enter");
  assert(await search.isVisible(), "No-match Enter keeps search open");
  await search.getByRole("combobox").fill("mikrofon");
  await search.getByRole("combobox").press("Enter");
  assert(
    (await command("set_call_muted"))!.args.callId === "parent",
    "Search uses selected call token",
  );
  await target
    .getByText("Stummschaltung wird bestätigt …", { exact: true })
    .waitFor();
  await searchButton.click();
  await search.getByRole("combobox").fill("mikrofon");
  assert(
    (await search.getByRole("option").first().getAttribute("aria-disabled")) ===
      "true",
    "Search honors pending mute",
  );
  await search.getByRole("combobox").press("Escape");
  assert(
    await searchButton.evaluate((el) => el === document.activeElement),
    "Escape restores focus",
  );
  await emit({ muted: true });
  await emit({ muted: false });
  await target.evaluate(() => {
    window.__controls.commands = [];
  });
  assert(
    await target
      .getByText("Sicherheit noch nicht bestätigt", { exact: true })
      .isVisible(),
    "Configured TLS must not fabricate native badge",
  );
  await target
    .getByRole("button", { name: "Stummschalten", exact: true })
    .click();
  assert(
    (await command("set_call_muted"))!.args.callId === "parent",
    "Mute uses selected token",
  );
  assert(
    await target
      .getByRole("button", { name: "Stummschalten", exact: true })
      .isVisible(),
    "Mute queue acceptance is not native confirmation",
  );
  assert(
    await target
      .getByRole("button", { name: "Stummschalten", exact: true })
      .isDisabled(),
    "Mute waits for native acknowledgment after queue acceptance",
  );
  await target
    .getByRole("button", { name: "Stummschalten", exact: true })
    .click({ force: true });
  assert(
    (await target.evaluate(
      () =>
        window.__controls.commands.filter(
          (item) => item.command === "set_call_muted",
        ).length,
    )) === 1,
    "Repeated click cannot queue another mute from stale snapshot",
  );
  await emit({ muted: true });
  await target
    .getByRole("button", { name: "Mikrofon einschalten", exact: true })
    .waitFor();
  await target.evaluate(() => (window.__controls.fail = "set_call_muted"));
  await target
    .getByRole("button", { name: "Mikrofon einschalten", exact: true })
    .click();
  await target
    .getByRole("alert")
    .filter({ hasText: "Telefonie-Aktion ist fehlgeschlagen" })
    .waitFor();
  assert(
    await target
      .getByRole("button", { name: "Mikrofon einschalten", exact: true })
      .isEnabled(),
    "Failed mute request releases only its pending guard",
  );
  await target.evaluate(() => (window.__controls.fail = null));
  await target.getByRole("button", { name: "Halten", exact: true }).click();
  assert(
    (await command("set_call_held"))!.args.held === true,
    "Local hold intent",
  );
  await emit({
    holdPending: true,
    lastError: { code: "nativeFailure", message: "timeout", sipStatus: null },
  });
  assert(
    await target
      .getByRole("button", { name: "Halten", exact: true })
      .isDisabled(),
    "Pending recovery blocks conflicting actions",
  );
  assert(
    await target
      .getByRole("button", { name: "Mikrofon einschalten", exact: true })
      .isEnabled(),
    "Mute survives recovery",
  );
  assert(
    await target
      .getByRole("button", { name: "Auflegen", exact: true })
      .isEnabled(),
    "Hangup survives recovery",
  );
  await target
    .getByText("Gesprächsänderung wird abgestimmt …", { exact: true })
    .waitFor();
  await emit({ holdPending: false, lastError: null, remoteHeld: true });
  await target
    .getByText("Gegenstelle hält das Gespräch", { exact: true })
    .waitFor();
  await target.setViewportSize({ width: 320, height: 440 });
  await target
    .getByRole("button", { name: "Vermitteln …", exact: true })
    .click();
  await target.getByLabel("Zielnummer", { exact: true }).fill("4567");
  await target
    .getByRole("button", { name: "Direkt vermitteln", exact: true })
    .click();
  for (const name of [
    "Kontakt auswählen",
    "Direkt vermitteln",
    "Rückfrage starten",
  ]) {
    const button = target.getByRole("button", { name, exact: true });
    await button.scrollIntoViewIfNeeded();
    await button.focus();
    assert(
      await button.evaluate((element) => element === document.activeElement),
      name + " keyboard focus reachable",
    );
    const bounds = await button.boundingBox();
    const nav = await target.locator(".bottom-nav").boundingBox();
    assert(
      bounds !== null &&
        nav !== null &&
        bounds.y + bounds.height <= nav.y &&
        bounds.y >= 60,
      name + " is inside scroll content above navigation",
    );
  }
  assert(
    (await command("transfer_call"))!.args.destination === "4567",
    "Blind transfer uses explicit number",
  );
  assert(
    (await target.getByLabel("Zielnummer", { exact: true }).inputValue()) ===
      "4567",
    "Keep draft on accepted queue",
  );
  await emit({ transfer: "pending" });
  await target.getByText("Vermittlung läuft …", { exact: true }).waitFor();
  await emit({
    transfer: "failed",
    lastError: { code: "sipBusy", sipStatus: 486, message: "safe" },
  });
  assert(
    (await target.getByLabel("Zielnummer", { exact: true }).inputValue()) ===
      "4567",
    "Keep draft after transfer failure/timeout",
  );
  await target
    .getByRole("button", { name: "Kontakt auswählen", exact: true })
    .click();
  await target.getByText(/Kontakte sind nicht freigegeben/).waitFor();
  assert(
    !(await target.evaluate(() =>
      window.__controls.commands.some((item) =>
        /request_contacts/.test(item.command),
      ),
    )),
    "Transfer must not request contacts permission",
  );
  await target.evaluate(() => {
    window.__controls.contactsPermission = "authorized";
    window.__controls.contactsChanged();
  });
  await target
    .getByRole("button", { name: "Ada Beispiel · Privat: 678", exact: true })
    .click();
  assert(
    (await target.getByLabel("Zielnummer", { exact: true }).inputValue()) ===
      "678",
    "Use chosen contact's canonical destination",
  );
  await target
    .getByRole("button", { name: "Rückfrage starten", exact: true })
    .click();
  assert(
    (await command("start_consultation"))!.args.callId === "parent",
    "Consultation uses original owning dialog",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.calls[0]!.localHeld = true;
    f.state.calls[0]!.lastError = null;
    f.state.calls.push(
      f.dialog("child", {
        consultation: true,
        destination: "678",
        state: "ringing",
      }),
    );
    f.emit("child");
  });
  assert(
    await target
      .getByRole("button", { name: "Vermittlung abschließen", exact: true })
      .isDisabled(),
    "Cannot complete dialing child",
  );
  await target
    .getByRole("button", { name: "Rückfrage abbrechen & zurück", exact: true })
    .click();
  assert(
    (await command("cancel_consultation"))!.args.callId === "child",
    "Cancel uses child token",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.calls[1]!.state = "active";
    f.emit("child");
  });
  await target
    .getByRole("button", { name: "Zu 9002 wechseln", exact: true })
    .click();
  assert(
    (await command("switch_call"))!.args.callId === "parent",
    "Switch targets original",
  );
  await target.evaluate(() => window.__controls.emit("parent"));
  await target
    .getByRole("button", { name: "Rückfrage abbrechen & zurück", exact: true })
    .click();
  assert(
    (await command("cancel_consultation"))!.args.callId === "child",
    "Parent selection must not change cancel ownership",
  );
  await target
    .getByRole("button", { name: "Vermittlung abschließen", exact: true })
    .click();
  assert(
    (await command("complete_transfer"))!.args.callId === "child",
    "Parent selection must not change completion ownership",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.calls.pop();
    f.state.calls[0]!.localHeld = false;
    f.emit("parent");
  });
  await target.getByLabel("Zielnummer", { exact: true }).waitFor();
  assert(
    (await target.getByLabel("Zielnummer", { exact: true }).inputValue()) ===
      "678",
    "Original conversation keeps draft across child selection/return",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    f.contactsPermission = "denied";
    f.contactsChanged();
  });
  await target.getByText(/Kontakte sind nicht freigegeben/).waitFor();
  assert(
    (await target.getByRole("button", { name: /Ada Beispiel/ }).count()) === 0,
    "Revoked contacts disappear",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.lastCallFailure = {
      callId: "ended-background",
      accountId: 1,
      error: {
        code: "mediaSecurityRequired",
        sipStatus: null,
        message: "safe",
      },
    };
    f.emit();
  });
  await target.getByText(/Anruffehler · Privat:/).waitFor();
  await target
    .getByRole("button", { name: "Anruffehler schließen", exact: true })
    .click();
  await target.evaluate(() => window.__controls.emit());
  assert(
    (await target.getByText(/Anruffehler · Privat:/).count()) === 0,
    "Dismissed owner failure stays dismissed across revisions",
  );
  await emit({
    security: {
      signalingTransport: "tls",
      tlsPeerVerified: true,
      media: "unprotected",
    },
  });
  await target
    .getByText("TLS geprüft · Audio unverschlüsselt", { exact: true })
    .waitFor();
  await emit({
    security: {
      signalingTransport: "tls",
      tlsPeerVerified: true,
      media: "sdesSrtp",
    },
  });
  await target.getByText("TLS geprüft · SRTP", { exact: true }).waitFor();
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.callEvents = [
      { callId: "child", sequence: 1, event: { kind: "transferSucceeded" } },
    ];
    f.state.calls = [];
    f.state.lastCallFailure = null;
    f.emit();
  });
  await target.getByText("Vermittlung bestätigt.", { exact: true }).waitFor();
  await target.evaluate(() => {
    const f = window.__controls;
    f.state.calls = [f.dialog("replacement", { destination: "7000" })];
    f.emit("replacement");
  });
  await target
    .getByRole("button", { name: "Vermitteln …", exact: true })
    .click();
  assert(
    (await target.getByLabel("Zielnummer", { exact: true }).inputValue()) ===
      "",
    "New conversation clears guarded draft",
  );
  await target.goto(baseUrl + "?retained-failure");
  await target.getByText(/Anruffehler · Privat:/).waitFor();
  await target
    .getByRole("button", { name: "Anruffehler schließen", exact: true })
    .click();
  await target.reload();
  await target.getByText(/Anruffehler · Privat:/).waitFor();
  await target.goto(baseUrl + "?window=settings");
  await target.getByLabel("SIP-Transport", { exact: true }).waitFor();
  assert(
    (await target.getByLabel("SIP-Transport", { exact: true }).inputValue()) ===
      "tls",
    "Reopen persisted TLS",
  );
  await target.getByLabel("SIP-Transport", { exact: true }).selectOption("tcp");
  await target
    .getByRole("button", { name: "Speichern & verbinden", exact: true })
    .click();
  await target
    .getByText("SRTP erforderlich benötigt TLS als SIP-Transport.", {
      exact: true,
    })
    .waitFor();
  assert(
    !(await command("save_account")),
    "Invalid security policy cannot save",
  );
  await target.getByRole("button", { name: "Ton", exact: true }).click();
  await target
    .getByRole("group", { name: "Ungespeicherte Änderungen" })
    .waitFor();
  await target
    .getByRole("button", { name: "Weiter bearbeiten", exact: true })
    .click();
  await target.getByLabel("SIP-Transport", { exact: true }).selectOption("tls");
  await target.locator(".account-advanced summary").click();
  await target
    .getByLabel("Separate Anmeldekennung", { exact: true })
    .fill("other-digest");
  await target
    .getByLabel("Outbound-Proxy", { exact: true })
    .fill("other.example.test:5061");
  await target
    .getByRole("button", { name: "Speichern & verbinden", exact: true })
    .click();
  const save = await command("save_account");
  assert(
    save!.args.account!.transport === "tls" &&
      save!.args.account!.mediaSecurity === "required" &&
      save!.args.account!.authUsername === "other-digest" &&
      save!.args.account!.outboundProxy === "other.example.test:5061",
    "Save carries all security fields",
  );
  await target.getByRole("button", { name: "Ton", exact: true }).click();
  await target.getByRole("button", { name: /Büro.*2@pbx/ }).click();
  assert(
    (await target
      .getByLabel("Separate Anmeldekennung", { exact: true })
      .inputValue()) === "other-digest",
    "Reopen retains optional auth",
  );
  await target.evaluate(() => {
    const f = window.__controls;
    const a = f.state.accounts[1]!;
    a.registration = "failed";
    a.lastError = {
      code: "certificateRejected",
      sipStatus: null,
      message: "safe",
    };
    f.emit();
  });
  await target
    .getByRole("button", { name: "Erneut verbinden", exact: true })
    .click();
  assert(
    (await command("reconnect_account"))!.args.id === 2,
    "Reconnect carries saved identity without rewriting draft",
  );
  await target
    .getByRole("button", { name: "SIP-Konto hinzufügen", exact: true })
    .click();
  assert(
    (await target.getByLabel("SIP-Transport", { exact: true }).inputValue()) ===
      "udp",
    "Create keeps legacy-compatible default",
  );
  assert(
    (await target.getByLabel("Audioschutz", { exact: true }).inputValue()) ===
      "disabled",
    "Create default no media requirement",
  );
  if (capture) {
    // One bounded batched visual round after functional checks, native fixtures only.
    await target.goto(baseUrl);
    await target.getByRole("heading", { name: "9002", exact: true }).waitFor();
    await target.evaluate(() => {
      const f = window.__controls;
      f.state.calls[0]!.security = {
        signalingTransport: "tls",
        tlsPeerVerified: true,
        media: "sdesSrtp",
      };
      f.emit();
    });
    await target.screenshot({ path: "output/playwright/task3-phone-340.png" });
    await target.setViewportSize({ width: 320, height: 440 });
    await target
      .getByRole("button", { name: "Vermitteln …", exact: true })
      .click();
    await target.getByLabel("Zielnummer", { exact: true }).fill("4567");
    await target.screenshot({
      path: "output/playwright/task3-transfer-320.png",
    });
    await target.evaluate(() => {
      const f = window.__controls;
      f.state.calls[0]!.localHeld = true;
      f.state.calls.push(
        f.dialog("visual-child", {
          destination: "1234567890123456789012345678901234567890",
          consultation: true,
        }),
      );
      f.state.lastCallFailure = {
        callId: "background",
        accountId: 1,
        error: { code: "transportFailure", sipStatus: null, message: "safe" },
      };
      f.emit("parent");
    });
    await target.screenshot({
      path: "output/playwright/task3-consultation-failure-320.png",
    });
    await target
      .getByRole("button", { name: "Vermittlung abschließen", exact: true })
      .scrollIntoViewIfNeeded();
    await target.screenshot({
      path: "output/playwright/task3-consultation-actions-320.png",
    });
    assert(
      await target
        .getByRole("button", { name: "Auflegen", exact: true })
        .isVisible(),
      "Hangup remains reachable in small two-dialog view",
    );
    assert(
      !(await target.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      )),
      "320px consultation must not overflow",
    );
    await target.setViewportSize({ width: 900, height: 660 });
    await target.goto(baseUrl + "?window=settings");
    await target.getByLabel("SIP-Transport", { exact: true }).waitFor();
    await target.locator(".account-advanced summary").click();
    await target
      .getByLabel("SIP-Transport", { exact: true })
      .scrollIntoViewIfNeeded();
    await target.screenshot({
      path: "output/playwright/task3-security-settings-900.png",
    });
    await target.setViewportSize({ width: 740, height: 540 });
    await target.screenshot({
      path: "output/playwright/task3-security-settings-740.png",
    });
    assert(
      !(await target.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      )),
      "Narrow settings must not overflow",
    );
  }
  assert(errors.length === 0, "Browser errors: " + errors.join("; "));
  await target.close();
  return { passed: true };
}
