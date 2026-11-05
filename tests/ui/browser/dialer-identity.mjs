// Synthetic identities only. These browser checks do not prove SIP/audio transport.
export default async function dialerIdentity(page, origin, capture = false) {
  await page.addInitScript(() => {
    if (location.search.includes("identity-flow") === false) return;
    window.isTauri = true;
    localStorage.setItem("sip.introduction.v1", "seen");
    let next = 1;
    const callbacks = new Map(),
      listeners = new Map();
    const contact = {
      id: "synthetic",
      source: "macos",
      name: "Mara Muster",
      organization: "",
      favorite: false,
      phones: [
        {
          label: "Arbeit",
          number: "+49 30 55501234",
          destination: "+493055501234",
        },
      ],
    };
    const account = {
      id: 1,
      name: "Testkonto",
      registrar: "pbx.example.test",
      username: "1000",
      enabled: true,
      enabledChanging: false,
      registeredAtMs: 1000,
      registration: "registered",
      lastError: null,
      removing: false,
      transport: "udp",
      mediaSecurity: "disabled",
      authUsername: null,
      outboundProxy: null,
    };
    const state = {
      revision: 1,
      registration: "registered",
      call: "idle",
      callId: null,
      callAccountId: null,
      selectedAccountId: 1,
      accounts: [account],
      calls: [],
      lastCallFailure: null,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      audioMode: "system",
      audioTest: null,
      audioTestError: null,
      ringtonePlaying: false,
      soundSettings: {
        inputDevice: "",
        outputDevice: "",
        ringDevice: "",
        ringtoneEnabled: true,
        ringtone: "classic",
        ringtoneVolume: 100,
      },
    };
    const fixture = {
      state,
      contact,
      permission: "authorized",
      commands: [],
      emit(event) {
        for (const [id, listener] of listeners)
          if (listener.event === event)
            callbacks.get(listener.handler)?.({
              id,
              event,
              payload: event === "phone-state-changed" ? this.project() : null,
            });
      },
      project() {
        state.revision++;
        const ref =
          this.permission === "authorized"
            ? { id: contact.id, source: contact.source, name: contact.name }
            : null;
        state.calls.forEach((call) => (call.contact = ref));
        state.callerContact = state.calls.length ? ref : null;
        return structuredClone(state);
      },
    };
    window.__identity = fixture;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(_event, id) {
        listeners.delete(id);
      },
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      transformCallback(fn) {
        const id = next++;
        callbacks.set(id, fn);
        return id;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args = {}) {
        fixture.commands.push({ command, args: structuredClone(args) });
        if (command === "plugin:event|listen") {
          const id = next++;
          listeners.set(id, args);
          return id;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_preferences")
          return {
            startHidden: true,
            showCallerInNotifications: true,
            contacts: {
              favoritesFirst: true,
              showOrganization: true,
              matchGermanNumbers: true,
            },
            history: { recordingEnabled: true, retentionDays: null },
          };
        if (command === "get_contacts" || command === "refresh_contacts")
          return {
            permission: fixture.permission,
            contacts: fixture.permission === "authorized" ? [contact] : [],
          };
        if (command === "get_notification_permission") return "authorized";
        if (command === "get_desktop_settings")
          return {
            notificationsEnabled: false,
            notificationPermission: "authorized",
            notificationsGranted: true,
            autostartAvailable: false,
            autostartEnabled: false,
          };
        if (command === "get_engine_snapshot") return fixture.project();
        if (command === "dial") {
          state.call = "active";
          state.callId = "identity-call";
          state.callAccountId = 1;
          state.callConnectedAtMs = Date.now();
          state.calls = [
            {
              contact: null,
              id: state.callId,
              accountId: 1,
              state: "active",
              destination: args.destination,
              caller: null,
              connectedAtMs: state.callConnectedAtMs,
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
                signalingTransport: "udp",
                tlsPeerVerified: false,
                media: "unprotected",
              },
            },
          ];
          return fixture.project();
        }
        if (command === "send_dtmf") {
          await new Promise((resolve) => setTimeout(resolve, 30));
          return fixture.project();
        }
        if (command === "get_history")
          return {
            entries: [
              {
                id: "old",
                accountId: 1,
                accountName: "Testkonto",
                accountUsername: "1000",
                accountRegistrar: "pbx.example.test",
                direction: "outgoing",
                number: "03055501234",
                remoteName: null,
                startedAtMs: Date.now() - 60000,
                answeredAtMs: Date.now() - 50000,
                endedAtMs: Date.now() - 40000,
                durationMs: 10000,
                outcome: "completed",
                sipStatus: null,
                callbackAccountId: 1,
                contact:
                  fixture.permission === "authorized"
                    ? {
                        id: contact.id,
                        source: contact.source,
                        name: contact.name,
                      }
                    : null,
              },
            ],
            total: 1,
            hasMore: false,
            recordingError: null,
          };
        throw new Error(`Unexpected identity fixture command ${command}`);
      },
    };
  });
  const assert = (ok, message) => {
    if (!ok) throw new Error(message);
  };
  await page.setViewportSize({ width: 340, height: 500 });
  await page.goto(`${origin}/?identity-flow=1`);
  const input = page.getByRole("textbox", { name: "Nummer oder Nebenstelle" });
  await input.fill("49 30 5550 1234");
  await input.press("Home");
  await page.getByRole("button", { name: "Plus einfügen" }).click();
  assert(
    (await input.inputValue()) === "+49 30 5550 1234",
    "leading plus/caret lost",
  );
  await input.fill("030 5550 1234");
  await page.getByRole("button", { name: /Mara Muster/ }).waitFor();
  assert(
    (await page.evaluate(
      () =>
        window.__identity.commands.filter((c) => c.command === "dial").length,
    )) === 0,
    "suggestion auto-dialed",
  );
  await input.press("End");
  await input.press("a");
  assert(
    (await input.inputValue()) === "030 5550 1234",
    "alphabetic typing accepted",
  );
  if (capture)
    await page.screenshot({
      path: "output/playwright/dialer-identity-340.png",
    });
  await page.getByRole("button", { name: "Anrufen", exact: true }).click();
  await page
    .getByRole("heading", { name: "Mara Muster", exact: true })
    .waitFor();
  assert(
    (await page.evaluate(
      () =>
        window.__identity.commands.find((c) => c.command === "dial").args
          .destination,
    )) === "03055501234",
    "dialing changed country or kept separators",
  );
  await page
    .getByRole("button", { name: "Gesprächstastatur einblenden" })
    .click();
  await page.evaluate(() => {
    for (const digit of ["5", "#"])
      document.querySelector(`[aria-label="Ton ${digit}"]`).click();
  });
  await page.waitForFunction(
    () =>
      window.__identity.commands.filter((c) => c.command === "send_dtmf")
        .length === 2,
  );
  const tones = await page.evaluate(() =>
    window.__identity.commands
      .filter((c) => c.command === "send_dtmf")
      .map((c) => [c.args.callId, c.args.digit]),
  );
  assert(
    JSON.stringify(tones) ===
      JSON.stringify([
        ["identity-call", "5"],
        ["identity-call", "#"],
      ]),
    "rapid tones lost/reordered or wrong dialog",
  );
  await page.setViewportSize({ width: 320, height: 440 });
  const hash = page.getByRole("button", { name: "Ton #", exact: true });
  await hash.scrollIntoViewIfNeeded();
  if (capture)
    await page.screenshot({ path: "output/playwright/dtmf-identity-320.png" });
  await page.evaluate(() => {
    window.__identity.contact.name = "Neuer Kontaktname";
    window.__identity.emit("contacts-changed");
    window.__identity.emit("phone-state-changed");
  });
  // A native contact refresh and state publication can arrive independently.
  await page.waitForTimeout(100);
  await page.evaluate(() => window.__identity.emit("phone-state-changed"));
  await page
    .getByRole("heading", { name: "Neuer Kontaktname", exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.__identity.permission = "denied";
    window.__identity.emit("contacts-changed");
  });
  await page
    .getByRole("heading", { name: "03055501234", exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.__identity.state.calls[0].localHeld = true;
    window.__identity.emit("phone-state-changed");
  });
  assert(await hash.isDisabled(), "held call permits tones");
  await page.evaluate(() => {
    const f = window.__identity;
    f.state.calls = [];
    f.state.call = "idle";
    f.state.callId = null;
    f.emit("phone-state-changed");
  });
  await page.getByRole("button", { name: "Verlauf", exact: true }).click();
  await page.getByText("03055501234", { exact: true }).first().waitFor();
  await page.evaluate(() => {
    const f = window.__identity;
    f.permission = "authorized";
    f.contact.name = "Später hinzugefügt";
    f.emit("contacts-changed");
  });
  await page.getByText("Später hinzugefügt", { exact: true }).first().waitFor();
  assert(
    (await page.evaluate(
      () =>
        window.__identity.commands.filter((c) =>
          c.command.startsWith("request_contacts"),
        ).length,
    )) === 0,
    "automatic contact permission prompt",
  );
  return { passed: true };
}
