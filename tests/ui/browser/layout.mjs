// Synthetic IPC only: screenshots and layout checks never open SIP/audio/keychains.
export default async function layoutRegressions(page) {
  const origin = new URL(page.url()).origin;
  const target = await page.context().newPage();
  await target.addInitScript(() => {
    window.isTauri = !location.search.includes("preview");
    const callbacks = new Map();
    let next = 1;
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "settings" },
        currentWebview: { label: "settings" },
      },
      transformCallback(fn) {
        const id = next++;
        callbacks.set(id, fn);
        return id;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command) {
        if (command === "get_app_info")
          return { version: "0.1.0", telephony: "linked" };
        if (command === "plugin:event|listen") return next++;
        if (command === "get_engine_snapshot")
          return {
            revision: 1,
            callEvents: [],
            callEventsDropped: 0,
            callConnectedAtMs: null,
            caller: null,
            callerContact: null,
            historyError: null,
            registration: "registered",
            call: "idle",
            callId: null,
            calls: [],
            lastCallFailure: null,
            lastError: null,
            accounts: Array.from(
              { length: location.search.includes("many-accounts") ? 8 : 1 },
              (_, index) => ({
                id: index + 1,
                name: index === 0 ? "Büro" : `Konto ${index + 1}`,
                username: "1000",
                transport: "udp",
                mediaSecurity: "disabled",
                authUsername: null,
                outboundProxy: null,
                registrar: "pbx.example.test",
                registration: "registered",
                removing: false,
                lastError: null,
                enabled: true,
                enabledChanging: false,
                registeredAtMs: null,
              }),
            ),
            selectedAccountId: 1,
            callAccountId: null,
            audioMode: "system",
            soundSettings: {
              inputDevice: "",
              outputDevice: "",
              ringDevice: "",
              ringtoneEnabled: true,
            },
            ringtonePlaying: false,
          };
        if (command === "get_desktop_settings")
          return {
            autostartAvailable: false,
            autostartEnabled: false,
            notificationsEnabled: true,
            notificationsGranted: null,
            notificationPermission: "unsupported",
          };
        if (command === "get_audio_devices") return { inputs: [], outputs: [] };
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
        if (command === "get_notification_permission") return "denied";
        if (
          command === "get_microphone_permission" ||
          command === "get_contacts_permission"
        )
          return "notDetermined";
        return null;
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    localStorage.setItem("sip.onboarding.dismissed.v1", "yes");
  });
  const errors = [];
  target.on("pageerror", (error) => errors.push(error.message));
  const check = async (label) => {
    const overflow = await target.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    );
    if (overflow) throw new Error(`${label}: horizontal overflow`);
    if (
      !(await target.evaluate(() =>
        getComputedStyle(document.body).fontFamily.includes("SIP UI"),
      ))
    )
      throw new Error(`${label}: shared font missing`);
  };
  await target.setViewportSize({ width: 600, height: 650 });
  await target.goto(origin);
  await target.getByRole("button", { name: "Tastatur", exact: true }).waitFor();
  const native = await target.locator(".softphone").boundingBox();
  if (
    native.x !== 0 ||
    native.y !== 0 ||
    native.width !== 600 ||
    native.height !== 650
  )
    throw new Error(
      `Native phone must fill its window, got ${JSON.stringify(native)}`,
    );
  await check("native phone");
  await target.screenshot({ path: "output/playwright/phone-native-wide.png" });
  for (const [width, height] of [
    [340, 500],
    [320, 440],
  ]) {
    await target.setViewportSize({ width, height });
    for (const view of ["Tastatur", "Kontakte", "Verlauf", "Starthilfe"]) {
      await target.getByRole("button", { name: view, exact: true }).click();
      await check(`${view} ${width}x${height}`);
      await target.screenshot({
        path: `output/playwright/phone-${view}-${width}.png`,
      });
    }
  }
  for (const [width, height] of [
    [900, 660],
    [740, 540],
  ]) {
    await target.setViewportSize({ width, height });
    await target.goto(`${origin}/?window=settings&section=general`);
    for (const section of [
      "Allgemein",
      "SIP-Konten",
      "Ton",
      "Mitteilungen",
      "Kontakte & Verlauf",
      "Datenschutz",
      "Diagnose & Hilfe",
      "Über Ruf",
    ]) {
      await target
        .getByRole("button", { name: new RegExp(`^${section}(?:\\s+\\d+)?$`) })
        .click();
      await target
        .locator('.settings-navigation button[aria-current="page"]')
        .filter({ hasText: section })
        .waitFor();
      await check(`${section} ${width}x${height}`);
      await target.screenshot({
        path: `output/playwright/settings-${section}-${width}.png`,
      });
    }
  }
  await target.setViewportSize({ width: 740, height: 540 });
  await target.goto(`${origin}/?window=settings&section=general&many-accounts`);
  await target.locator(".accounts-list button").last().waitFor();
  if (
    !(await target
      .locator(".accounts-list")
      .evaluate((el) => el.scrollHeight > el.clientHeight))
  )
    throw Error("Eight accounts must scroll independently");
  await target.locator(".accounts-list button").last().scrollIntoViewIfNeeded();
  await check("eight accounts at minimum window size");
  await target.screenshot({
    path: "output/playwright/settings-eight-accounts-740.png",
  });
  await target.setViewportSize({ width: 592, height: 432 });
  const zoom = await target.context().newCDPSession(target);
  await zoom.send("Emulation.setDeviceMetricsOverride", {
    width: 592,
    height: 432,
    deviceScaleFactor: 1.25,
    mobile: false,
  });
  await target.keyboard.press("Meta+f");
  if (
    !(await target
      .getByLabel("Konten und Bereiche suchen", { exact: true })
      .evaluate((el) => el === document.activeElement))
  )
    throw Error("Keyboard search unavailable with enlarged text");
  await check("settings at 125 percent zoom");
  await target.screenshot({ path: "output/playwright/settings-zoom-125.png" });
  if (
    !(await target
      .locator(".accounts-list")
      .evaluate((el) => el.clientHeight >= 48))
  )
    throw Error("Enlarged settings must keep an account row visible");
  const footer = await target.locator(".settings-sidebar-footer").boundingBox();
  if (!footer || footer.y + footer.height > 432)
    throw Error(
      `Enlarged settings must retain the sidebar footer: ${JSON.stringify({ footer, viewport: await target.evaluate(() => ({ width: innerWidth, height: innerHeight, zoom: getComputedStyle(document.body).zoom })) })}`,
    );
  await zoom.send("Emulation.clearDeviceMetricsOverride");
  await zoom.detach();
  await target.setViewportSize({ width: 740, height: 540 });
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
    await target.goto(
      `${origin}/?window=settings&section=${section}&many-accounts`,
    );
    await target.locator("h1").waitFor();
    await target.addStyleTag({ content: "body { zoom: 2; }" });
    if (section === "accounts")
      await target.locator(".account-advanced summary").click();
    const layout = await target.evaluate(() => {
      const areas = [
        ...document.querySelectorAll(
          ".settings-shell,.settings-sidebar,.settings-main,.settings-scroll,.setting-row",
        ),
      ];
      return areas
        .filter((el) => el.scrollWidth > el.clientWidth + 2)
        .map((el) => ({
          area: el.className,
          width: el.clientWidth,
          scroll: el.scrollWidth,
        }));
    });
    if (layout.length)
      throw Error(
        `${section}: content overflows at 200 percent: ${JSON.stringify(layout)}`,
      );
    const fields = target.locator(
      ".settings-scroll input:visible:not(:disabled), .settings-scroll select:visible:not(:disabled)",
    );
    for (const index of [...new Set([0, (await fields.count()) - 1])]) {
      if (index < 0 || !(await fields.count())) continue;
      const field = fields.nth(index);
      await field.focus();
      await field.scrollIntoViewIfNeeded();
      if (
        !(await field.evaluate((el) => {
          const box = el.getBoundingClientRect();
          const atCenter = document.elementFromPoint(
            box.x + box.width / 2,
            box.y + box.height / 2,
          );
          return (
            box.x >= 0 &&
            box.right <= innerWidth + 1 &&
            box.y >= 0 &&
            box.bottom <= innerHeight + 1 &&
            (atCenter === el || el.contains(atCenter))
          );
        }))
      )
        throw Error(
          `${section}: focused field is covered or outside the enlarged viewport`,
        );
    }
    await target
      .locator(".settings-scroll")
      .evaluate((el) => (el.scrollTop = 0));
    await target.screenshot({
      path: `output/playwright/settings-${section}-zoom-200.png`,
    });
    const sidebar = target.getByRole("complementary", {
      name: "Einstellungsnavigation",
    });
    await sidebar
      .getByRole("button", { name: "Über Ruf", exact: true })
      .focus();
    const navBox = await sidebar
      .getByRole("button", { name: "Über Ruf", exact: true })
      .boundingBox();
    if (!navBox || navBox.y < 0 || navBox.y + navBox.height > 541)
      throw Error(`${section}: navigation cannot be reached at 200 percent`);
    if (section === "accounts") {
      await target.getByLabel("Name", { exact: true }).focus();
      const input = await target
        .getByLabel("Name", { exact: true })
        .boundingBox();
      if (
        !input ||
        input.x < 0 ||
        input.x + input.width > 741 ||
        input.y < 0 ||
        input.y + input.height > 541
      )
        throw Error(
          "Account field cannot be focused within the enlarged viewport",
        );
    }
  }
  await target.setViewportSize({ width: 740, height: 540 });
  await target.goto(`${origin}/?preview`);
  const preview = await target.locator(".softphone").boundingBox();
  if (preview.width !== 340)
    throw new Error("Browser preview must retain compact 340px shell");
  if (errors.length) throw new Error(errors.join("\n"));
  await target.close();
  return { passed: true, screenshots: 35 };
}
