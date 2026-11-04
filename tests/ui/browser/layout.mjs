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
            registration: "registered",
            call: "idle",
            callId: null,
            lastError: null,
            accounts: [
              {
                id: 1,
                name: "Büro",
                username: "1000",
                registrar: "pbx.example.test",
                registration: "registered",
                removing: false,
                lastError: null,
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
            },
            ringtonePlaying: false,
          };
        if (command === "get_desktop_settings")
          return {
            autostartAvailable: false,
            autostartEnabled: false,
            notificationsEnabled: true,
            notificationsGranted: null,
          };
        if (command === "get_audio_devices") return { inputs: [], outputs: [] };
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
    for (const section of ["Allgemein", "SIP-Konten", "Ton", "Über Ruf"]) {
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
  await target.goto(`${origin}/?preview`);
  const preview = await target.locator(".softphone").boundingBox();
  if (preview.width !== 340)
    throw new Error("Browser preview must retain compact 340px shell");
  if (errors.length) throw new Error(errors.join("\n"));
  await target.close();
  return { passed: true, screenshots: 17 };
}
