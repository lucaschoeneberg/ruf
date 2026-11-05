// Run with the repository browser runner; the IPC fixture never contacts SIP.
export default async function settingsRegressions(
  page,
  baseUrl = new URL(page.url()).origin,
) {
  await page.addInitScript(() => {
    const syncDialogs = (snapshot) => {
      snapshot.lastCallFailure ??= null;
      if (snapshot.call === "idle" || !snapshot.callId) snapshot.calls = [];
      else {
        const previous = snapshot.calls?.find(
          (call) => call.id === snapshot.callId,
        );
        const selected = {
          id: snapshot.callId,
          accountId: snapshot.callAccountId,
          state: snapshot.call,
          destination: snapshot.caller
            ? null
            : (snapshot.destination ?? previous?.destination ?? "1234"),
          caller: snapshot.caller ?? null,
          connectedAtMs: snapshot.callConnectedAtMs ?? null,
          muted: false,
          localHeld: false,
          remoteHeld: snapshot.call === "held",
          holdPending: false,
          transfer: "idle",
          consultation: false,
          events: snapshot.callEvents ?? [],
          eventsDropped: 0,
          lastError: null,
          security: {
            signalingTransport: null,
            tlsPeerVerified: false,
            media: "unknown",
          },
          ...previous,
          state: snapshot.call,
          caller: snapshot.caller ?? null,
          connectedAtMs: snapshot.callConnectedAtMs ?? null,
          events: snapshot.callEvents ?? [],
        };
        snapshot.calls = [selected];
      }
      return snapshot;
    };
    window.isTauri = true;
    const scenario = new URLSearchParams(location.search).get("scenario");
    const soundSettings = {
      inputDevice: "",
      outputDevice: "",
      ringDevice: scenario === "devices-failure" ? "saved-speaker" : "",
      ringtoneEnabled: true,
      ringtone: "classic",
      ringtoneVolume: 100,
    };
    const saved = {
      id: 1,
      name: "Büro",
      transport: "udp",
      mediaSecurity: "disabled",
      authUsername: null,
      outboundProxy: null,
      registrar: "pbx.example.test",
      username: "alice",
      registration: "registered",
      lastError: null,
      removing: false,
      enabled: true,
      enabledChanging: false,
      registeredAtMs: 1730000000000,
    };
    const snapshot = {
      revision: 1,
      callConnectedAtMs: null,
      callEvents: [],
      callEventsDropped: 0,
      registration: "registered",
      call: "idle",
      callId: null,
      caller: null,
      callerContact: null,
      historyError: null,
      lastError: null,
      accounts: [saved],
      selectedAccountId: 1,
      callAccountId: null,
      audioMode: "system",
      soundSettings,
      ringtonePlaying: false,
      audioTest: null,
      audioTestError: null,
    };
    const credentialCode = {
      "credential-access": "credentialAccessRequired",
      "credential-cancel": "credentialAccessRequired",
      "credential-missing": "credentialMissing",
      "credential-failure": "credentialFailure",
      "credential-storage": "storageFailure",
    }[scenario];
    if (credentialCode) {
      saved.registration = "failed";
      saved.lastError = {
        code: credentialCode,
        message: "Synthetic local credential failure",
        sipStatus: null,
      };
      snapshot.registration = "failed";
      snapshot.lastError = saved.lastError;
    }
    const notificationPermission =
      scenario === "notification-permission"
        ? "notDetermined"
        : scenario === "notification-denied"
          ? "denied"
          : "unsupported";
    const desktop = {
      autostartEnabled: false,
      autostartAvailable: true,
      notificationsEnabled:
        scenario === "notification-permission" ||
        scenario === "notification-denied",
      notificationsGranted:
        notificationPermission === "unsupported" ? null : false,
      notificationPermission,
    };
    const callbacks = new Map();
    const listeners = new Map();
    let callbackId = 0;
    window.__settingsTest = {
      snapshot,
      desktop,
      preferences: {
        startHidden: true,
        showCallerInNotifications: true,
        contacts: { favoritesFirst: true, showOrganization: true },
        history: { recordingEnabled: true, retentionDays: null },
      },
      requestPermissionResult: "authorized",
      commands: [],
      closed: false,
      failDesktopSave: true,
      failTestNotification: false,
      failOpenNotificationSettings: false,
      failPreferences: scenario === "preferences-failure",
      deviceAttempts: 0,
      holdSnapshot: false,
      resolveSnapshot: null,
      rejectSnapshot: null,
      holdSave: false,
      resolveSave: null,
      emit(payload) {
        payload = { ...payload, revision: this.snapshot.revision + 1 };
        this.snapshot = structuredClone(syncDialogs(payload));
        this.event("phone-state-changed", payload);
      },
      event(event, payload = null) {
        const id = listeners.get(event);
        if (id) callbacks.get(id)({ event, payload });
      },
    };
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "settings" },
        currentWebview: { label: "settings" },
      },
      transformCallback(callback) {
        const id = ++callbackId;
        callbacks.set(id, callback);
        return id;
      },
      invoke: async (command, args) => {
        const state = window.__settingsTest;
        state.commands.push({ command, args });
        if (command === "plugin:event|listen") {
          listeners.set(args.event, args.handler);
          return callbackId;
        }
        if (command === "plugin:event|unlisten") return;
        if (command === "plugin:window|destroy") {
          state.closed = true;
          return;
        }
        if (command === "settings_ready") return;
        if (command === "open_about_link") {
          if (scenario === "about-link-failure")
            throw { code: "nativeFailure", message: "Browser unavailable" };
          return;
        }
        if (command === "quit_app") {
          state.quit = true;
          return;
        }
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot") {
          state.snapshot.revision++;
          const captured = structuredClone(syncDialogs(state.snapshot));
          if (state.holdSnapshot)
            return new Promise((resolve, reject) => {
              state.resolveSnapshot = () => resolve(captured);
              state.rejectSnapshot = reject;
            });
          return captured;
        }
        if (command === "get_desktop_settings") {
          if (scenario === "desktop-failure")
            throw {
              code: "storageFailure",
              message: "Preferences unavailable",
            };
          return structuredClone(state.desktop);
        }
        if (command === "get_preferences") {
          if (state.failPreferences)
            throw {
              code: "storageFailure",
              message: "Synthetic preference read failure",
            };
          return structuredClone(state.preferences);
        }
        if (command === "save_preferences") {
          if (
            args.preferences.history.retentionDays !== null &&
            !args.confirmHistoryDeletion
          )
            throw {
              code: "invalidState",
              message: "Retention requires confirmation",
            };
          state.preferences = structuredClone(args.preferences);
          return state.preferences;
        }
        if (command === "get_notification_permission") {
          if (scenario === "permission-failure")
            throw {
              code: "nativeFailure",
              message: "Synthetic permission read failure",
            };
          return state.desktop.notificationPermission;
        }
        if (command === "get_microphone_permission") {
          if (state.holdMicrophonePermission)
            return new Promise((resolve) => {
              state.resolveMicrophonePermission = () => resolve("authorized");
            });
          return "notDetermined";
        }
        if (command === "start_audio_test") {
          state.snapshot.audioTest = {
            id: "owned-test",
            kind: args.kind,
            level: 0.2,
          };
          return "owned-test";
        }
        if (command === "stop_audio_test") {
          if (state.snapshot.audioTest?.id === args.id)
            state.snapshot.audioTest = null;
          return;
        }
        if (command === "test_notification" && state.failTestNotification)
          throw {
            code: "nativeFailure",
            message: "Synthetic failed test delivery",
          };
        if (
          command === "open_notification_settings" &&
          state.failOpenNotificationSettings
        )
          throw {
            code: "nativeFailure",
            message: "Synthetic failed system settings open",
          };
        if (
          command === "open_notification_settings" ||
          command === "open_microphone_settings" ||
          command === "test_notification"
        )
          return;
        if (command === "get_diagnostics")
          return {
            version: "0.1.0",
            platform: "macos",
            telephony: "linked",
            accountsTotal: 1,
            accountsRegistered: 1,
            callActive: false,
            notificationPermission: state.desktop.notificationPermission,
            historyAvailable: true,
            lastError: null,
            microphonePermission: "notDetermined",
            audioDevicesAvailable: true,
            credentialError: null,
          };
        if (command === "copy_diagnostics" || command === "clear_history")
          return;
        if (command === "set_account_enabled") {
          state.snapshot.accounts[0].enabled = args.enabled;
          state.snapshot.accounts[0].registration = args.enabled
            ? "registered"
            : "unconfigured";
          state.snapshot.revision++;
          return structuredClone(syncDialogs(state.snapshot));
        }
        if (command === "select_account") {
          state.snapshot.selectedAccountId = args.id;
          state.snapshot.revision++;
          return structuredClone(syncDialogs(state.snapshot));
        }
        if (command === "request_notification_permission") {
          state.desktop.notificationPermission = state.requestPermissionResult;
          state.desktop.notificationsGranted =
            state.requestPermissionResult === "unsupported"
              ? null
              : state.requestPermissionResult === "authorized";
          return structuredClone(state.desktop);
        }
        if (command === "get_audio_devices") {
          if (scenario === "devices-failure" && state.deviceAttempts++ === 0)
            throw {
              code: "nativeFailure",
              message: "Device enumeration failed",
            };
          return {
            inputs: [],
            outputs: [
              {
                id: "draft-output",
                name: "Synthetic speaker",
                selectable: true,
              },
            ],
          };
        }
        if (command === "save_desktop_settings") {
          if (state.failDesktopSave)
            throw { code: "storageFailure", message: "Desktop save failed" };
          Object.assign(state.desktop, args);
          return structuredClone(state.desktop);
        }
        if (command === "save_sound_settings") {
          state.snapshot.revision++;
          state.snapshot.soundSettings = { ...args.sound };
          return structuredClone(syncDialogs(state.snapshot));
        }
        if (command === "reconnect_account") {
          if (scenario === "credential-cancel")
            throw {
              code: "credentialAccessRequired",
              message: "Synthetic cancelled permission",
              sipStatus: null,
            };
          if (args.id !== 1 || state.snapshot.call !== "idle")
            throw {
              code: "invalidState",
              message: "Account cannot reconnect now",
              sipStatus: null,
            };
          state.snapshot.revision++;
          state.snapshot.registration = "registering";
          state.snapshot.accounts[0].registration = "registering";
          state.snapshot.accounts[0].lastError = null;
          state.snapshot.lastError = null;
          return structuredClone(syncDialogs(state.snapshot));
        }
        if (command === "save_account") {
          if (state.holdSave) {
            state.snapshot.revision++;
            Object.assign(state.snapshot.accounts[0], {
              name: args.name,
              registrar: args.account.registrar,
              username: args.account.username,
              transport: args.account.transport,
              mediaSecurity: args.account.mediaSecurity,
              authUsername: args.account.authUsername,
              outboundProxy: args.account.outboundProxy,
            });
            const captured = structuredClone(syncDialogs(state.snapshot));
            return new Promise((resolve) => {
              state.resolveSave = () => resolve(captured);
            });
          }
          if (scenario === "busy-save")
            await new Promise((resolve) => {
              state.releaseSave = resolve;
            });
          throw { code: "storageFailure", message: "Save rejected" };
        }
        if (command === "stop_ringtone_preview") {
          if (scenario === "slow-preview-stop")
            await new Promise((resolve) => {
              state.releaseStop = resolve;
            });
          state.snapshot.revision++;
          state.snapshot.ringtonePlaying = false;
          return structuredClone(syncDialogs(state.snapshot));
        }
        if (command === "preview_ringtone") {
          state.snapshot.ringtonePlaying = true;
          state.emit(state.snapshot);
          return structuredClone(syncDialogs(state.snapshot));
        }
        throw Error(`Unexpected synthetic IPC command: ${command}`);
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  });
  const failures = [];
  const check = async (name, scenario, run) => {
    try {
      await page.goto(`${baseUrl}/?window=settings&scenario=${scenario}`);
      await page.getByLabel("Name", { exact: true }).waitFor();
      await page.waitForFunction(
        () =>
          !document.body.textContent.includes("Einstellungen werden geladen"),
      );
      await run();
    } catch (error) {
      failures.push(`${name}: ${error.message}`);
    }
  };
  await check(
    "test notification failure preserves known authorization and caller draft",
    "notification-permission",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.desktop.notificationPermission = "authorized";
        window.__settingsTest.failTestNotification = true;
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Erlaubt", { exact: true }).waitFor();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByRole("alert")
        .filter({ hasText: "Die Testmitteilung konnte nicht" })
        .waitFor();
      if (
        (await page.locator(".permission-state").textContent()).trim() !==
        "Erlaubt"
      )
        throw Error("Test delivery failure replaced known OS authorization");
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw Error("Delivery failure reset caller draft");
      if (
        await page
          .getByText("Die Testmitteilung wird in fünf Sekunden angefordert.", {
            exact: false,
          })
          .count()
      )
        throw Error("Failed delivery still claims to be pending");
      await page.evaluate(() => {
        window.__settingsTest.failTestNotification = false;
      });
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByText("Testmitteilung angefordert. Fokus und Systemregeln", {
          exact: false,
        })
        .waitFor();
      if (await page.getByRole("alert").count())
        throw Error("Successful retry retained old delivery error");
    },
  );
  await check(
    "opening system settings failure does not masquerade as permission query failure",
    "notification-denied",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.failOpenNotificationSettings = true;
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Im System blockiert", { exact: true }).waitFor();
      await page
        .getByRole("button", {
          name: "Systemeinstellungen öffnen",
          exact: true,
        })
        .click();
      await page
        .getByRole("alert")
        .filter({ hasText: "konnten nicht geöffnet" })
        .waitFor();
      if (
        (await page.locator(".permission-state").textContent()).trim() !==
        "Im System blockiert"
      )
        throw Error("Open failure erased denied authorization state");
    },
  );
  await check(
    "version search focuses the actual About heading",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("Version");
      await page
        .getByRole("button", {
          name: "Version und Software Über Ruf",
          exact: true,
        })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "about-heading",
        {},
        { timeout: 1500 },
      );
    },
  );
  await check(
    "account overview actions identify each account and headings follow the section",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("group", { name: "Büro", exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("heading", { name: "SIP-Konten", level: 1, exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("heading", { name: "Büro", level: 2, exact: true })
        .waitFor({ timeout: 1500 });
      await page
        .getByRole("heading", {
          name: "Kontoinformationen",
          level: 3,
          exact: true,
        })
        .waitFor({ timeout: 1500 });
    },
  );
  await check(
    "long valid account names cannot overflow the inner content or overlap actions",
    "",
    async () => {
      await page.setViewportSize({ width: 740, height: 540 });
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.accounts = Array.from({ length: 8 }, (_, i) => ({
          ...state.snapshot.accounts[0],
          id: i + 1,
          name: i === 0 ? "W".repeat(64) : "Auditkonto " + (i + 1),
        }));
        state.emit(state.snapshot);
      });
      const layout = await page
        .locator(".settings-scroll")
        .evaluate((el) => ({ scroll: el.scrollWidth, client: el.clientWidth }));
      if (layout.scroll > layout.client + 1)
        throw Error(
          `Inner settings overflow: ${layout.scroll}/${layout.client}`,
        );
      const row = page.locator(".account-overview .setting-row").first();
      const name = await row.locator("h3").boundingBox(),
        actions = await row.locator(".settings-actions").boundingBox();
      if (!name || !actions || name.x + name.width > actions.x)
        throw Error("Account name overlaps actions");
      await page.setViewportSize({ width: 900, height: 660 });
    },
  );
  await check(
    "every normal visible setting label resolves to its actual control",
    "",
    async () => {
      for (const section of [
        "general",
        "accounts",
        "audio",
        "notifications",
        "contacts",
        "privacy",
        "diagnostics",
        "about",
      ]) {
        await page.goto(`${baseUrl}/?window=settings&section=${section}`);
        await page.waitForFunction(
          () =>
            !document.body.textContent.includes("Einstellungen werden geladen"),
        );
        const missing = await page.evaluate(async () => {
          const { searchSettings } =
            await import("/src/lib/settings-navigation.ts");
          return [...document.querySelectorAll("main label[for]")]
            .filter(
              (el) =>
                el.getClientRects().length &&
                !el.closest("details:not([open])"),
            )
            .map((el) => ({ label: el.textContent.trim(), target: el.htmlFor }))
            .filter(
              ({ label, target }) =>
                !searchSettings(label).some(
                  (result) => result.target === target,
                ),
            );
        });
        if (missing.length)
          throw Error(
            `${section}: visible labels missing from search ${JSON.stringify(missing)}`,
          );
      }
    },
  );
  await check(
    "search opens the existing parent disclosure before focusing SBOM",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("SBOM");
      await page
        .getByRole("button", {
          name: "Projektinventar (SBOM) Über Ruf",
          exact: true,
        })
        .click();
      await page.waitForFunction(
        () => document.activeElement?.id === "software-inventory",
      );
      if (!(await page.locator("#software-inventory").isVisible()))
        throw Error("SBOM search target remains inside closed disclosure");
    },
  );
  await check(
    "About shows authorship, company links and offline licenses",
    "",
    async () => {
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page
        .locator("footer")
        .getByText("Autor: Mardu GmbH", { exact: true })
        .waitFor();
      if (
        await page
          .getByRole("list", { name: "Zentrale Softwarekomponenten" })
          .isVisible()
      )
        throw Error("Software details should start collapsed");
      for (const [label, link, href] of [
        ["Website", "company", "https://www.mardu.de/"],
        ["Impressum", "imprint", "https://www.mardu.de/publisher"],
        ["Datenschutz", "privacy", "https://www.mardu.de/privacy"],
      ]) {
        const anchor = page.getByRole("link", {
          name: new RegExp(`^${label}`),
        });
        if ((await anchor.getAttribute("href")) !== href)
          throw Error(`Incorrect ${label} URL`);
        await anchor.click();
        await page.waitForFunction(
          (link) =>
            window.__settingsTest.commands.some(
              (item) =>
                item.command === "open_about_link" && item.args.link === link,
            ),
          link,
        );
      }
      await page.getByText("Software & Lizenzen", { exact: true }).click();
      const overview = page.getByRole("list", {
        name: "Zentrale Softwarekomponenten",
      });
      for (const name of ["Svelte", "Tauri", "rusqlite", "IBM Plex Sans"])
        if (!(await overview.textContent()).includes(name))
          throw Error(`Missing central component: ${name}`);
      if ((await overview.locator("li").count()) > 8)
        throw Error(
          "The compact software overview should show only central components",
        );
      await page
        .getByText("Lizenztexte & Herkunftshinweise", { exact: true })
        .click();
      await page.getByText("Ruf · MIT-Lizenz", { exact: true }).click();
      const licenseText = page.getByRole("textbox", {
        name: "Ruf MIT-Lizenz",
        exact: true,
      });
      await licenseText.waitFor();
      if (
        !(await licenseText.inputValue()).includes(
          "Permission is hereby granted",
        )
      )
        throw Error("Bundled MIT license missing");
      await page
        .getByText("Lizenztexte & Herkunftshinweise", { exact: true })
        .click();
      await page.getByText("Projektinventar (SBOM)", { exact: true }).click();
      const packages = JSON.parse(
        await page
          .getByRole("textbox", {
            name: "SBOM als CycloneDX-JSON",
            exact: true,
          })
          .inputValue(),
      );
      if (
        packages.bomFormat !== "CycloneDX" ||
        !packages.components.some((item) => item.name === "tauri") ||
        packages.components.some((item) => item.name === "baresip")
      )
        throw Error("Portable preview must expose its own software inventory");
    },
  );
  await check(
    "consumer changelog loads GitHub text without rendering HTML",
    "",
    async () => {
      const route = "https://raw.githubusercontent.com/**";
      await page.route(route, (request) =>
        request.fulfill({
          status: 200,
          contentType: "text/plain",
          body: "# Was ist neu?\n\n## 0.2.0 · Beispiel\n\n- Kontakte schneller finden.\n- <img src=x onerror=alert(1)>\n",
        }),
      );
      try {
        await page
          .getByRole("button", { name: "Über Ruf", exact: true })
          .click();
        await page.getByText("Was ist neu?", { exact: true }).click();
        await page
          .getByRole("heading", { name: "0.2.0 · Beispiel", exact: true })
          .waitFor();
        await page
          .getByText("Kontakte schneller finden.", { exact: true })
          .waitFor();
        if (await page.locator(".changelog img").count())
          throw Error("Remote HTML was rendered");
      } finally {
        await page.unroute(route);
      }
    },
  );
  await check(
    "consumer changelog keeps bundled notes when GitHub is unavailable",
    "",
    async () => {
      const route = "https://raw.githubusercontent.com/**";
      await page.route(route, (request) => request.abort());
      try {
        await page
          .getByRole("button", { name: "Über Ruf", exact: true })
          .click();
        await page.getByText("Was ist neu?", { exact: true }).click();
        await page
          .getByRole("heading", { name: "0.1.0 · Vorschau", exact: true })
          .waitFor();
        await page
          .getByText("Online-Änderungen konnten nicht geladen werden.", {
            exact: false,
          })
          .waitFor();
      } finally {
        await page.unroute(route);
      }
    },
  );
  await check(
    "About reports browser failures without navigating away",
    "about-link-failure",
    async () => {
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page.getByRole("link", { name: /^Impressum/ }).click();
      await page
        .getByRole("alert")
        .filter({ hasText: "Die Seite konnte nicht geöffnet werden" })
        .waitFor();
      await page
        .getByRole("heading", { name: "Über Ruf", exact: true })
        .waitFor();
    },
  );
  await check(
    "credential reconnect is explicit, sends only id and preserves drafts",
    "credential-access",
    async () => {
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "Loading settings prompted for credential access automatically",
        );
      await page
        .getByLabel("Name", { exact: true })
        .fill("Nicht gespeicherter Name");
      await page
        .getByLabel("Passwort", { exact: true })
        .fill("unsaved-secret-draft");
      await page
        .getByRole("button", { name: "Erneut verbinden", exact: true })
        .click();
      await page.getByText("Wird verbunden …", { exact: true }).waitFor();
      const commands = await page.evaluate(() =>
        window.__settingsTest.commands.filter(
          ({ command }) => command === "reconnect_account",
        ),
      );
      if (
        commands.length !== 1 ||
        JSON.stringify(commands[0].args) !== '{"id":1}'
      )
        throw new Error(
          "Reconnect did not use exactly the existing account id",
        );
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "save_account",
          ),
        )
      )
        throw new Error("Reconnect saved unsaved metadata or password");
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
          "Nicht gespeicherter Name" ||
        (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
          "unsaved-secret-draft"
      )
        throw new Error("Successful reconnect discarded the unsaved draft");
    },
  );
  await check(
    "cancelled credential access keeps drafts and allows an explicit retry",
    "credential-cancel",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Entwurf nach Abbruch");
      await page
        .getByLabel("Passwort", { exact: true })
        .fill("unsaved-cancelled-draft");
      await page
        .getByRole("button", { name: "Erneut verbinden", exact: true })
        .click();
      await page.getByRole("alert").waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
          "Entwurf nach Abbruch" ||
        (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
          "unsaved-cancelled-draft"
      )
        throw new Error("Cancelled credential access discarded the draft");
      if (
        await page
          .getByRole("button", { name: "Erneut verbinden", exact: true })
          .isDisabled()
      )
        throw new Error("Cancelled access left explicit retry disabled");
      const commands = await page.evaluate(() =>
        window.__settingsTest.commands.filter(
          ({ command }) => command === "reconnect_account",
        ),
      );
      if (
        commands.length !== 1 ||
        JSON.stringify(commands[0].args) !== '{"id":1}'
      )
        throw new Error(
          "Cancellation retried automatically or sent the draft password",
        );
    },
  );
  await check(
    "credential reconnect is blocked while a call is present",
    "credential-access",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        Object.assign(f.snapshot, {
          call: "incoming",
          callId: "credential-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
      });
      const reconnect = page.getByRole("button", {
        name: "Erneut verbinden",
        exact: true,
      });
      await reconnect.waitFor();
      if (!(await reconnect.isDisabled()))
        throw new Error("Reconnect remained enabled during a call");
      await reconnect.dispatchEvent("click");
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "A disabled reconnect still invoked the credential command",
        );
    },
  );
  await check(
    "missing stored credential requires password entry before save",
    "credential-missing",
    async () => {
      if (
        await page
          .getByRole("button", { name: "Erneut verbinden", exact: true })
          .count()
      )
        throw new Error(
          "Missing password offered a futile credential-access retry",
        );
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page
        .getByText("Gib dein SIP-Passwort ein", { exact: false })
        .waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) =>
              command === "save_account" || command === "reconnect_account",
          ),
        )
      )
        throw new Error(
          "Missing credential initiated a save or prompt without a password",
        );
    },
  );
  await check(
    "continue editing restores focus to the changed field",
    "",
    async () => {
      const input = page.getByLabel("Name", { exact: true });
      await input.fill("Geänderter Entwurf");
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      const keep = page.getByRole("button", {
        name: "Weiter bearbeiten",
        exact: true,
      });
      await keep.waitFor();
      await keep.press("Enter");
      if (!(await input.evaluate((e) => e === document.activeElement)))
        throw new Error(
          "Continue editing did not return focus to the edited field",
        );
      if ((await input.inputValue()) !== "Geänderter Entwurf")
        throw new Error("Continue editing lost the draft");
      await page
        .getByRole("textbox", { name: "Konten und Bereiche suchen" })
        .fill("Ton");
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await keep.waitFor();
      await keep.press("Escape");
      if (!(await input.evaluate((e) => e === document.activeElement)))
        throw new Error("Escape did not restore the edited field");
    },
  );
  await check(
    "older event cannot re-enable account edits during a newer call",
    "",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Unveränderter Entwurf");
      await page.evaluate(() => {
        const f = window.__settingsTest;
        const older = structuredClone(f.snapshot);
        f.holdSnapshot = true;
        Object.assign(f.snapshot, {
          call: "incoming",
          callId: "new-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
        f.event("phone-state-changed", older);
      });
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw new Error(
          "Older idle event re-enabled edits during the incoming call",
        );
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Unveränderter Entwurf"
      )
        throw new Error("State ordering discarded the account draft");
    },
  );
  await check(
    "obsolete read failure preserves a newer call without an error",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.holdSnapshot = true;
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__settingsTest.rejectSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        Object.assign(f.snapshot, {
          call: "incoming",
          callId: "failure-call",
          callAccountId: 1,
        });
        f.emit(f.snapshot);
        f.rejectSnapshot({
          code: "nativeFailure",
          message: "Obsolete status failure",
        });
      });
      await page.evaluate(
        () => new Promise((resolve) => setTimeout(resolve, 0)),
      );
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw new Error("Read failure removed the valid incoming call");
      if (await page.getByRole("alert").count())
        throw new Error(
          "Obsolete read failure replaced the newer status with an error",
        );
    },
  );
  await check(
    "newer read survives an older event delivered while pending",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.older = structuredClone(f.snapshot);
        f.holdSnapshot = true;
        Object.assign(f.snapshot, {
          call: "incoming",
          callId: "read-call",
          callAccountId: 1,
        });
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await page.waitForFunction(
        () => window.__settingsTest.resolveSnapshot !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.event("phone-state-changed", f.older);
        f.resolveSnapshot();
      });
      await page.waitForFunction(() =>
        document.getElementById("name").matches(":disabled"),
      );
    },
  );
  await check(
    "newer account save survives an older event delivered while pending",
    "",
    async () => {
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.older = structuredClone(f.snapshot);
        f.holdSave = true;
        f.holdSnapshot = true;
      });
      await page.getByLabel("Name", { exact: true }).fill("Neuer Kontoname");
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.waitForFunction(
        () => window.__settingsTest.resolveSave !== null,
      );
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.event("phone-state-changed", f.older);
        f.resolveSave();
      });
      await page
        .getByText("Gespeichert. Die Anmeldung an der Telefonanlage läuft.", {
          exact: true,
        })
        .waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Neuer Kontoname"
      )
        throw new Error("Older event discarded the newer saved account name");
      await page
        .getByRole("navigation", { name: "SIP-Konten", exact: true })
        .getByRole("button", { name: /Neuer Kontoname/ })
        .waitFor();
    },
  );
  await check("failed account save keeps retry draft", "", async () => {
    await page
      .getByLabel("Passwort", { exact: true })
      .fill("synthetic-retry-draft");
    await page
      .getByRole("button", { name: "Speichern & verbinden", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    if (
      (await page.getByLabel("Passwort", { exact: true }).inputValue()) !==
      "synthetic-retry-draft"
    )
      throw Error("Password draft was lost");
  });
  await check(
    "desktop failure does not discard loaded accounts",
    "desktop-failure",
    async () => {
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !== "Büro"
      )
        throw Error("Engine state was discarded");
      if (await page.getByLabel("Name", { exact: true }).isDisabled())
        throw Error("Account recovery was disabled by preference failure");
    },
  );
  await check("clean account form follows external metadata", "", async () => {
    await page.evaluate(() => {
      const state = window.__settingsTest;
      state.snapshot.accounts[0].name = "Aktualisiert";
      state.emit(state.snapshot);
    });
    await page.waitForFunction(
      () => document.getElementById("name").value === "Aktualisiert",
      null,
      { timeout: 2000 },
    );
  });
  await check(
    "removed account cannot silently become a new account",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Mein Entwurf");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.accounts = [];
        state.emit(state.snapshot);
      });
      await page
        .getByText("Dieses Konto wurde entfernt.", { exact: false })
        .waitFor({ timeout: 2000 });
      if (
        !(await page
          .getByRole("heading", { name: "Konto entfernt", exact: true })
          .count())
      )
        throw Error("Removed form is labelled as new-account creation");
      if (
        !(await page
          .getByRole("button", { name: "Konto entfernt", exact: true })
          .isDisabled())
      )
        throw Error("Removed draft can be submitted as a new account");
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Mein Entwurf"
      )
        throw Error("Removed account draft was lost");
    },
  );
  await check(
    "enabled defaults retain an explicit first authorization action",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      if (
        !(await page
          .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
          .isChecked())
      )
        throw new Error("Saved enabled notification preference was lost");
      if (
        !(await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled())
      )
        throw new Error("Unchanged defaults were marked dirty");
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .waitFor();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "request_notification_permission",
          ),
        )
      )
        throw new Error(
          "Settings requested permission without an explicit action",
        );
    },
  );
  await check(
    "denied notifications explain OS recovery without promising another prompt",
    "notification-denied",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw new Error(
          "Denied permission still offers an ineffective permission prompt",
        );
      await page
        .getByText(
          "Mitteilungen sind in den Systemeinstellungen für Ruf deaktiviert.",
          { exact: false },
        )
        .waitFor();
    },
  );
  await check(
    "unsupported authorization response stays distinct from denial",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.evaluate(
        () => (window.__settingsTest.requestPermissionResult = "unsupported"),
      );
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .click();
      await page
        .getByText(
          "Systemmitteilungen sind in diesem Startmodus nicht verfügbar.",
          { exact: false },
        )
        .waitFor();
      if (
        await page
          .getByText(
            "Erlaube Mitteilungen für Ruf in den Systemeinstellungen.",
            { exact: false },
          )
          .count()
      )
        throw new Error(
          "Unsupported context was described as a denied OS preference",
        );
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw new Error("Unsupported context still offers a permission prompt");
    },
  );
  await check(
    "returning from OS permission settings refreshes status and preserves drafts",
    "notification-denied",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page.evaluate(() => {
        const f = window.__settingsTest;
        f.desktop.notificationPermission = "authorized";
        f.desktop.notificationsGranted = true;
        f.commands = [];
        window.dispatchEvent(new Event("focus"));
      });
      await page.waitForFunction(() =>
        window.__settingsTest.commands.some(
          ({ command }) => command === "get_notification_permission",
        ),
      );
      await page
        .getByText("Mitteilungen sind erlaubt. Fokus, Banner-Einstellungen", {
          exact: false,
        })
        .waitFor();
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw new Error("Authorization refresh overwrote the autostart draft");
      if (
        await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled()
      )
        throw new Error(
          "Authorization refresh reset the dirty preference baseline",
        );
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            ({ command }) => command === "request_notification_permission",
          ),
        )
      )
        throw new Error("A focus refresh requested permission automatically");
    },
  );
  await check(
    "notification permission preserves preference drafts",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .uncheck();
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .click();
      await page
        .getByText("Mitteilungen sind erlaubt. Fokus, Banner-Einstellungen", {
          exact: false,
        })
        .waitFor();
      if (
        await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked()
      )
        throw Error(
          "Permission result overwrote unsaved login-start preference",
        );
      if (
        await page
          .getByRole("button", { name: "Mitteilungen speichern", exact: true })
          .isDisabled()
      )
        throw Error("Permission result discarded dirty preference baseline");
      if (
        await page
          .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
          .count()
      )
        throw Error("Granted permission still shown as unavailable");
      const calls = await page.evaluate(
        () =>
          window.__settingsTest.commands.filter(
            (c) => c.command === "request_notification_permission",
          ).length,
      );
      if (calls !== 1)
        throw Error(
          "Permission was requested without an explicit single action",
        );
    },
  );
  await check(
    "OS rights remain readable despite account preference failure",
    "desktop-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Nicht verfügbar", { exact: true }).waitFor();
      if (
        !(await page
          .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
          .isEnabled())
      )
        throw Error("Independent OS check blocked by storage");
      if (
        !(await page
          .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
          .isDisabled())
      )
        throw Error("Failed preferences became editable defaults");
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "request_notification_permission",
          ),
        )
      )
        throw Error("Read-only permission query prompted");
    },
  );
  await check(
    "failed OS query stays unknown and retryable",
    "permission-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Prüfung fehlgeschlagen", { exact: true }).waitFor();
      if (await page.getByText("Im System blockiert", { exact: true }).count())
        throw Error("Failure invented denial");
      if (
        !(await page
          .getByRole("button", { name: "Freigabe erneut prüfen", exact: true })
          .isEnabled())
      )
        throw Error("Retry unavailable");
    },
  );
  await check(
    "explicit test notification works with app switch off and preserves call state",
    "",
    async () => {
      await page.evaluate(() => {
        window.__settingsTest.desktop.notificationPermission = "authorized";
      });
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      await page.getByText("Erlaubt", { exact: true }).waitFor();
      if (
        await page
          .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
          .isChecked()
      )
        throw Error("App switch unexpectedly enabled");
      await page
        .getByRole("button", { name: "Testmitteilung senden", exact: true })
        .click();
      await page
        .getByText("Testmitteilung angefordert. Fokus und Systemregeln", {
          exact: false,
        })
        .waitFor();
      const state = await page.evaluate(() => ({
        call: window.__settingsTest.snapshot.call,
        commands: window.__settingsTest.commands.map((c) => c.command),
      }));
      if (
        state.call !== "idle" ||
        state.commands.filter((c) => c === "test_notification").length !== 1 ||
        state.commands.includes("request_notification_permission")
      )
        throw Error("Test notice changed call or authorization");
    },
  );
  await check(
    "field search focuses without discarding edits in the same section",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Mein Entwurf");
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("SIP-Konto");
      await page
        .getByRole("button", {
          name: "SIP-Konto bearbeiten SIP-Konten",
          exact: true,
        })
        .click();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Mein Entwurf"
      )
        throw Error("Search reset same-section draft");
      if (
        !(await page
          .getByLabel("Name", { exact: true })
          .evaluate((e) => e === document.activeElement))
      )
        throw Error("Search failed to focus field");
      await page
        .getByRole("button", { name: "Datenschutz", exact: true })
        .click();
      await page
        .getByRole("group", { name: "Ungespeicherte Änderungen" })
        .waitFor();
    },
  );
  await check(
    "account disable preserves selection and exposes native state",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Aktivieren · Büro", exact: true })
        .waitFor();
      if (
        (await page.evaluate(
          () => window.__settingsTest.snapshot.selectedAccountId,
        )) !== 1
      )
        throw Error("Disable switched caller identity");
      await page
        .getByRole("button", { name: "Aktivieren · Büro", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Deaktivieren · Büro", exact: true })
        .waitFor();
    },
  );
  await check(
    "history retention requires confirmation and settings preserve draft",
    "",
    async () => {
      await page
        .getByRole("button", { name: "Kontakte & Verlauf", exact: true })
        .click();
      await page
        .getByLabel("Aufbewahrung", { exact: true })
        .selectOption({ label: "30 Tage" });
      await page
        .getByRole("button", {
          name: "Anzeige & Verlauf speichern",
          exact: true,
        })
        .click();
      await page.getByRole("alert").waitFor();
      await page
        .getByLabel("Löschung älterer Einträge bestätigen", { exact: true })
        .check();
      await page
        .getByRole("button", {
          name: "Anzeige & Verlauf speichern",
          exact: true,
        })
        .click();
      await page
        .getByText("Einstellungen gespeichert.", { exact: true })
        .waitFor();
      if (
        (await page.evaluate(
          () => window.__settingsTest.preferences.history.retentionDays,
        )) !== 30
      )
        throw Error("Retention not saved");
      await page
        .getByRole("button", { name: "Verlauf löschen …", exact: true })
        .click();
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "clear_history",
          ),
        )
      )
        throw Error("History deleted before confirmation");
      await page
        .getByRole("button", { name: "Abbrechen", exact: true })
        .click();
    },
  );
  await check(
    "late microphone permission cannot start capture after navigation",
    "",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.evaluate(() => {
        window.__settingsTest.holdMicrophonePermission = true;
      });
      await page
        .getByRole("button", { name: "Mikrofon testen", exact: true })
        .click();
      await page.waitForFunction(
        () =>
          typeof window.__settingsTest.resolveMicrophonePermission ===
          "function",
      );
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page.evaluate(() =>
        window.__settingsTest.resolveMicrophonePermission(),
      );
      await page.waitForTimeout(150);
      if (
        await page.evaluate(() =>
          window.__settingsTest.commands.some(
            (c) => c.command === "start_audio_test",
          ),
        )
      )
        throw Error("Late permission started microphone after leaving audio");
    },
  );
  await check(
    "speaker test uses draft call output and owned test stops on navigation",
    "",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByLabel("Lautsprecher", { exact: true })
        .selectOption("draft-output");
      await page
        .getByRole("button", { name: "Lautsprecher testen", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Audiotest stoppen", exact: true })
        .waitFor();
      await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
      await page
        .getByRole("button", { name: "Änderungen verwerfen", exact: true })
        .click();
      await page.waitForFunction(() =>
        window.__settingsTest.commands.some(
          (c) => c.command === "stop_audio_test" && c.args.id === "owned-test",
        ),
      );
      const test = await page.evaluate(() =>
        window.__settingsTest.commands.find(
          (c) => c.command === "start_audio_test",
        ),
      );
      if (test.args.kind !== "speaker" || test.args.device !== "draft-output")
        throw Error("Speaker test used wrong output");
    },
  );
  await check(
    "failed preference load can recover without reopening",
    "preferences-failure",
    async () => {
      await page
        .getByRole("button", { name: "Mitteilungen", exact: true })
        .click();
      if (
        !(await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isDisabled())
      )
        throw Error("Unknown preferences became editable");
      await page.evaluate(() => {
        window.__settingsTest.failPreferences = false;
      });
      await page
        .getByRole("button", {
          name: "Einstellungen erneut laden",
          exact: true,
        })
        .click();
      await page
        .getByLabel("Anruferdetails anzeigen", { exact: true })
        .waitFor({ state: "visible" });
      await page.waitForFunction(
        () =>
          !document.getElementById("notification-caller").disabled &&
          !document.getElementById("notification-caller").closest("fieldset")
            .disabled,
      );
      if (
        !(await page
          .getByLabel("Anruferdetails anzeigen", { exact: true })
          .isChecked())
      )
        throw Error("Recovery lost saved preference");
    },
  );
  await check("window close protects an account draft", "", async () => {
    await page.getByLabel("Name", { exact: true }).fill("Nicht verlieren");
    await page.evaluate(() =>
      window.__settingsTest.event("tauri://close-requested"),
    );
    await page
      .getByRole("group", { name: "Ungespeicherte Änderungen" })
      .waitFor({ timeout: 2000 });
    if (await page.evaluate(() => window.__settingsTest.closed))
      throw Error("Dirty window closed");
    await page
      .getByRole("button", { name: "Weiter bearbeiten", exact: true })
      .click();
    if (
      (await page.getByLabel("Name", { exact: true }).inputValue()) !==
      "Nicht verlieren"
    )
      throw Error("Kept draft was lost");
    await page.evaluate(() =>
      window.__settingsTest.event("tauri://close-requested"),
    );
    await page
      .getByRole("button", { name: "Änderungen verwerfen", exact: true })
      .click();
    await page.waitForFunction(() => window.__settingsTest.closed, null, {
      timeout: 2000,
    });
  });
  await check("application quit uses the draft guard", "", async () => {
    await page.getByLabel("Name", { exact: true }).fill("Nicht beenden");
    await page.evaluate(() =>
      window.__settingsTest.event("settings-quit-requested"),
    );
    await page
      .getByRole("group", { name: "Ungespeicherte Änderungen" })
      .waitFor({ timeout: 2000 });
    if (await page.evaluate(() => window.__settingsTest.quit))
      throw Error("Dirty application quit");
    await page
      .getByRole("button", { name: "Änderungen verwerfen", exact: true })
      .click();
    await page.waitForFunction(() => window.__settingsTest.quit, null, {
      timeout: 2000,
    });
  });
  await check("leaving sound stops the owned preview", "", async () => {
    await page.getByRole("button", { name: "Ton", exact: true }).click();
    await page
      .getByRole("button", { name: "Klingelton anhören", exact: true })
      .click();
    await page
      .getByRole("button", { name: "Hörprobe stoppen", exact: true })
      .waitFor();
    await page.getByRole("button", { name: "Über Ruf", exact: true }).click();
    await page.waitForFunction(
      () =>
        window.__settingsTest.commands.some(
          (item) => item.command === "stop_ringtone_preview",
        ),
      null,
      { timeout: 2000 },
    );
  });
  await check(
    "incoming call freezes and restores the account draft",
    "",
    async () => {
      await page.getByLabel("Name", { exact: true }).fill("Entwurf bleibt");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.call = "incoming";
        state.snapshot.callId = "synthetic-call";
        state.snapshot.callAccountId = 1;
        state.emit(state.snapshot);
      });
      if (!(await page.getByLabel("Name", { exact: true }).isDisabled()))
        throw Error("Call did not freeze edits");
      await page.evaluate(() => {
        const state = window.__settingsTest;
        state.snapshot.call = "idle";
        state.snapshot.callId = null;
        state.snapshot.callAccountId = null;
        state.emit(state.snapshot);
      });
      await page.waitForFunction(
        () => !document.getElementById("name").matches(":disabled"),
      );
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !==
        "Entwurf bleibt"
      )
        throw Error("Call discarded draft");
    },
  );
  await check("failed desktop save preserves draft for retry", "", async () => {
    await page
      .getByRole("button", { name: "Mitteilungen", exact: true })
      .click();
    await page
      .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
      .check();
    await page
      .getByRole("button", { name: "Mitteilungen speichern", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    if (
      !(await page
        .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
        .isChecked())
    )
      throw Error("Notification preference draft lost");
    await page.evaluate(() => {
      window.__settingsTest.failDesktopSave = false;
    });
    await page
      .getByRole("button", { name: "Mitteilungen speichern", exact: true })
      .click();
    await page
      .getByText("Einstellungen gespeichert.", { exact: true })
      .waitFor();
    if (
      !(await page
        .getByRole("button", { name: "Mitteilungen speichern", exact: true })
        .isDisabled())
    )
      throw Error("Successful save remained dirty");
  });
  await check(
    "audio enumeration retries and missing output can use system default",
    "devices-failure",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page.getByRole("alert").waitFor();
      await page
        .getByRole("button", { name: "Geräte aktualisieren", exact: true })
        .click();
      await page
        .getByText("Gespeichertes Gerät nicht verfügbar.", { exact: false })
        .waitFor();
      await page
        .getByLabel("Klingelton-Ausgabe", { exact: true })
        .selectOption("");
      await page
        .getByRole("button", { name: "Ton speichern", exact: true })
        .click();
      await page
        .getByText("Ton-Einstellungen gespeichert.", { exact: false })
        .waitFor();
      if (
        (await page
          .getByLabel("Klingelton-Ausgabe", { exact: true })
          .inputValue()) !== ""
      )
        throw Error("Missing output could not recover to system default");
    },
  );
  await check(
    "zero search results can be cleared without losing the current form",
    "",
    async () => {
      await page
        .getByLabel("Konten und Bereiche suchen", { exact: true })
        .fill("kein-treffer-xyz");
      await page.getByText("Keine Treffer", { exact: false }).waitFor();
      await page
        .getByRole("button", { name: "Suche leeren", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Allgemein", exact: true })
        .waitFor();
      if (
        (await page.getByLabel("Name", { exact: true }).inputValue()) !== "Büro"
      )
        throw Error("Search altered account form");
    },
  );
  await check(
    "window close waits while a save is in flight",
    "busy-save",
    async () => {
      await page
        .getByLabel("Name", { exact: true })
        .fill("Noch nicht gespeichert");
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .click();
      await page.waitForFunction(
        () => typeof window.__settingsTest.releaseSave === "function",
      );
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page.getByRole("alert").waitFor();
      if (await page.evaluate(() => window.__settingsTest.closed))
        throw Error("Window closed during pending save");
      await page.evaluate(() => window.__settingsTest.releaseSave());
      await page
        .getByRole("button", { name: "Speichern & verbinden", exact: true })
        .waitFor();
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page
        .getByRole("group", { name: "Ungespeicherte Änderungen" })
        .waitFor();
    },
  );
  await check(
    "closing while preview stops prevents late edits",
    "slow-preview-stop",
    async () => {
      await page.getByRole("button", { name: "Ton", exact: true }).click();
      await page
        .getByRole("button", { name: "Klingelton anhören", exact: true })
        .click();
      await page
        .getByRole("button", { name: "Hörprobe stoppen", exact: true })
        .waitFor();
      await page.evaluate(() =>
        window.__settingsTest.event("tauri://close-requested"),
      );
      await page.waitForFunction(
        () => typeof window.__settingsTest.releaseStop === "function",
      );
      const blocked = await page
        .getByLabel("Bei Anrufen klingeln", { exact: true })
        .isDisabled();
      await page.evaluate(() => window.__settingsTest.releaseStop());
      if (!blocked)
        throw Error("New edits were accepted after close was confirmed");
      await page.waitForFunction(() => window.__settingsTest.closed);
    },
  );
  if (failures.length) throw Error(failures.join("\n"));
}
