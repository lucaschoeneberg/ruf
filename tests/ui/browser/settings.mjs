// Run with the repository browser runner; the IPC fixture never contacts SIP.
export default async function settingsRegressions(
  page,
  baseUrl = new URL(page.url()).origin,
) {
  await page.addInitScript(() => {
    window.isTauri = true;
    const scenario = new URLSearchParams(location.search).get("scenario");
    const soundSettings = {
      inputDevice: "",
      outputDevice: "",
      ringDevice: scenario === "devices-failure" ? "saved-speaker" : "",
      ringtoneEnabled: true,
    };
    const saved = {
      id: 1,
      name: "Büro",
      registrar: "pbx.example.test",
      username: "alice",
      registration: "registered",
      lastError: null,
      removing: false,
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
    };
    const callbacks = new Map();
    const listeners = new Map();
    let callbackId = 0;
    window.__settingsTest = {
      snapshot,
      commands: [],
      closed: false,
      failDesktopSave: true,
      deviceAttempts: 0,
      holdSnapshot: false,
      resolveSnapshot: null,
      rejectSnapshot: null,
      holdSave: false,
      resolveSave: null,
      emit(payload) {
        payload = { ...payload, revision: this.snapshot.revision + 1 };
        this.snapshot = structuredClone(payload);
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
        if (command === "quit_app") {
          state.quit = true;
          return;
        }
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "get_engine_snapshot") {
          state.snapshot.revision++;
          const captured = structuredClone(state.snapshot);
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
          return {
            autostartEnabled: false,
            autostartAvailable: true,
            notificationsEnabled: scenario === "notification-permission",
            notificationsGranted:
              scenario === "notification-permission" ? false : null,
          };
        }
        if (command === "request_notification_permission") {
          return {
            autostartEnabled: false,
            autostartAvailable: true,
            notificationsEnabled: true,
            notificationsGranted: true,
          };
        }
        if (command === "get_audio_devices") {
          if (scenario === "devices-failure" && state.deviceAttempts++ === 0)
            throw {
              code: "nativeFailure",
              message: "Device enumeration failed",
            };
          return { inputs: [], outputs: [] };
        }
        if (command === "save_desktop_settings") {
          if (state.failDesktopSave)
            throw { code: "storageFailure", message: "Desktop save failed" };
          return {
            ...args,
            autostartAvailable: true,
            notificationsGranted: null,
          };
        }
        if (command === "save_sound_settings") {
          state.snapshot.revision++;
          state.snapshot.soundSettings = { ...args.sound };
          return structuredClone(state.snapshot);
        }
        if (command === "save_account") {
          if (state.holdSave) {
            state.snapshot.revision++;
            Object.assign(state.snapshot.accounts[0], {
              name: args.name,
              registrar: args.account.registrar,
              username: args.account.username,
            });
            const captured = structuredClone(state.snapshot);
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
          return structuredClone(state.snapshot);
        }
        if (command === "preview_ringtone") {
          state.snapshot.ringtonePlaying = true;
          state.emit(state.snapshot);
          return structuredClone(state.snapshot);
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
      await page.getByRole("button", { name: /Neuer Kontoname/ }).waitFor();
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
    "notification permission preserves preference drafts",
    "notification-permission",
    async () => {
      await page
        .getByRole("button", { name: "Allgemein", exact: true })
        .click();
      await page
        .getByLabel("Bei der Anmeldung starten", { exact: true })
        .check();
      await page
        .getByRole("button", { name: "Mitteilungen erlauben", exact: true })
        .click();
      await page
        .getByText("Mitteilungen sind erlaubt.", { exact: false })
        .waitFor();
      if (
        !(await page
          .getByLabel("Bei der Anmeldung starten", { exact: true })
          .isChecked())
      )
        throw Error(
          "Permission result overwrote unsaved login-start preference",
        );
      if (
        await page
          .getByRole("button", { name: "Allgemein speichern", exact: true })
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
    await page.getByRole("button", { name: "Allgemein", exact: true }).click();
    await page
      .getByLabel("Eingehende Anrufe anzeigen", { exact: true })
      .check();
    await page
      .getByRole("button", { name: "Allgemein speichern", exact: true })
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
      .getByRole("button", { name: "Allgemein speichern", exact: true })
      .click();
    await page
      .getByText("Allgemeine Einstellungen gespeichert.", { exact: true })
      .waitFor();
    if (
      !(await page
        .getByRole("button", { name: "Allgemein speichern", exact: true })
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
