// Capture the actual Svelte UI with isolated synthetic native-shaped IPC.
// Run: bun scripts/capture-app-store.ts
// Output: ignored artifacts/app-store/screenshots, 2880x1800 RGB PNGs.
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  mkdirSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { installAppStoreFixtures } from "../tests/ui/browser/app-store-fixtures.ts";

const root = resolve(import.meta.dir, "..");
const output = join(root, "artifacts/app-store/screenshots");
const temporary = mkdtempSync(join(tmpdir(), "ruf-app-store-"));
const cli = [
  "--package",
  "@playwright/cli@0.1.23",
  "playwright-cli",
  `-s=ruf-store-${process.pid}`,
];
const cache =
  process.platform === "darwin"
    ? join(homedir(), "Library/Caches/ms-playwright")
    : process.platform === "win32"
      ? join(process.env.LOCALAPPDATA ?? homedir(), "ms-playwright")
      : join(
          process.env.XDG_CACHE_HOME ?? join(homedir(), ".cache"),
          "ms-playwright",
        );
const binaries = [
  "chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  "chrome-mac-x64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  "chrome-linux/chrome",
  "chrome-linux64/chrome",
  "chrome-win/chrome.exe",
  "chrome-win64/chrome.exe",
];
const executable =
  process.env.SIP_UI_BROWSER_EXECUTABLE ??
  (existsSync(cache)
    ? readdirSync(cache)
        .filter((name) => name.startsWith("chromium-"))
        .sort()
        .reverse()
        .flatMap((name) => binaries.map((binary) => join(cache, name, binary)))
        .find(existsSync)
    : undefined);
const config = join(temporary, "browser.json");
await Bun.write(
  config,
  JSON.stringify({
    browser: {
      browserName: "chromium",
      launchOptions: {
        headless: true,
        ...(executable ? { executablePath: executable } : {}),
      },
      contextOptions: {
        viewport: { width: 1440, height: 900 },
        deviceScaleFactor: 2,
      },
    },
  }),
);
const run = (...args: string[]) => {
  const result = execFileSync("bunx", [...cli, ...args], {
    cwd: root,
    encoding: "utf8",
    timeout: 180000,
  });
  if (result.includes("### Error")) throw new Error(result);
  return result;
};
const reservation = Bun.serve({
  hostname: "127.0.0.1",
  port: 0,
  fetch: () => new Response(),
});
const port = reservation.port;
reservation.stop(true);
const server = Bun.spawn(
  [
    "bun",
    "run",
    "--cwd",
    "apps/desktop",
    "dev",
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  { cwd: root, stdout: "ignore", stderr: "pipe" },
);
const errors = new Response(server.stderr).text();
let passed = false;
try {
  const url = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 30000;
  while (true) {
    try {
      if ((await fetch(url, { signal: AbortSignal.timeout(1000) })).ok) break;
    } catch {}
    if (Date.now() > deadline)
      throw new Error("Screenshot server did not start");
    await Bun.sleep(100);
  }
  mkdirSync(output, { recursive: true });
  run("open", url, `--config=${config}`);
  const capture = join(temporary, "capture.js");
  await Bun.write(
    capture,
    `async (page) => {
    await page.context().addInitScript(${installAppStoreFixtures.toString()});
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const capture = async (name, settings = false) => {
      await page.evaluate(async () => { await document.fonts.ready; document.activeElement?.blur(); });
      await page.waitForTimeout(250);
      const privacy = await page.evaluate(() => ({ text: document.body.innerText, commands: window.__appStoreFixture.commands }));
      if (errors.length) throw new Error(errors.join('\\n'));
      if (privacy.commands.some(({ command }) => command.startsWith('request_') || command === 'dial' || command === 'start_audio_test')) throw new Error('Unexpected permission/audio/call action');
      // Compose an unmodified browser capture inside an artwork-only window.
      // The actual product DOM and CSS are never changed for the screenshot.
      const image = await page.screenshot({omitBackground:false, animations:'disabled'});
      const stage = await page.context().newPage();
      await stage.setViewportSize({width:1440,height:900});
      await stage.setContent('<!doctype html><html><head><style>*{box-sizing:border-box}body{margin:0;width:1440px;height:900px;display:flex;align-items:center;justify-content:center;background:#18222f}main{overflow:hidden;border:1px solid #46505c;border-radius:10px;box-shadow:0 26px 90px #0009,0 3px 10px #0007;background:#0d1117}.titlebar{height:36px;background:#232a33;border-bottom:1px solid #3c444f;display:flex;align-items:center;justify-content:center;font:600 13px -apple-system,BlinkMacSystemFont,sans-serif;color:#cbd2db;position:relative}.controls{position:absolute;left:13px;display:flex;gap:8px}.controls i{height:11px;width:11px;border-radius:50%;background:#ff6059}.controls i:nth-child(2){background:#ffbd2e}.controls i:nth-child(3){background:#28c840}img{display:block;width:' + (settings ? 1100 : 480) + 'px;height:740px}</style></head><body><main><div class="titlebar"><span class="controls"><i></i><i></i><i></i></span>Ruf' + (settings ? ' – Einstellungen' : '') + '</div><img alt="Ruf" src="data:image/png;base64,' + image.toString('base64') + '"></main></body></html>');
      await stage.locator('img').evaluate(async image => {await image.decode()});
      await stage.screenshot({ path: ${JSON.stringify(output)} + '/' + name + '.png', fullPage:false, omitBackground:false, animations:'disabled' });
      await stage.close();
      return { name, text:privacy.text, commands:privacy.commands.map(({command}) => command) };
    };
    const results = [];
    await page.setViewportSize({width:480,height:740});
    await page.goto(${JSON.stringify(url)});
    await page.getByText('Angemeldet', {exact:true}).waitFor();
    await page.getByRole('textbox', {name:'Nummer oder Nebenstelle',exact:true}).fill('1201');
    await page.getByRole('button', {name:'Anrufen',exact:true}).waitFor({state:'visible'});
    results.push(await capture('01-telefon'));
    await page.goto(${JSON.stringify(url)});
    await page.getByText('Angemeldet', {exact:true}).waitFor();
    await page.evaluate(() => window.__appStoreFixture.setCall('active'));
    await page.getByText('Gespräch läuft', {exact:true}).waitFor();
    results.push(await capture('02-gespraech'));
    await page.goto(${JSON.stringify(url)});
    await page.getByRole('button', {name:'Kontakte',exact:true}).click();
    await page.getByRole('button', {name:'Details zu Anna Beispiel',exact:true}).click();
    await page.getByRole('group', {name:'Anna Beispiel: Kontaktdetails',exact:true}).waitFor();
    results.push(await capture('03-kontakte'));
    await page.goto(${JSON.stringify(url)});
    await page.getByRole('button', {name:'Verlauf',exact:true}).click();
    await page.getByText('Anna Beispiel', {exact:true}).waitFor();
    results.push(await capture('04-verlauf'));
    await page.setViewportSize({width:1100,height:740});
    await page.goto(${JSON.stringify(url)} + '/?window=settings&section=audio');
    await page.getByRole('heading', {name:'Ton',exact:true}).waitFor();
    await page.getByLabel('Mikrofon', {exact:true}).selectOption('headset-input');
    results.push(await capture('05-ton', true));
    return results;
  }`,
  );
  const result = run("run-code", `--filename=${capture}`);
  const captureResult = result.match(/### Result\n([^\n]+)/)?.[1];
  if (!captureResult) throw new Error("Missing screenshot privacy report");
  await Bun.write(join(output, "capture-report.json"), captureResult + "\n");
  for (const name of [
    "01-telefon",
    "02-gespraech",
    "03-kontakte",
    "04-verlauf",
    "05-ton",
  ]) {
    const png = Buffer.from(
      await Bun.file(join(output, `${name}.png`)).arrayBuffer(),
    );
    // PNG IHDR dimensions, bit depth and RGB color type (2 excludes alpha).
    if (
      png.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a" ||
      png.readUInt32BE(16) !== 2880 ||
      png.readUInt32BE(20) !== 1800 ||
      png[24] !== 8 ||
      png[25] !== 2
    )
      throw new Error(`Invalid store PNG format: ${name}`);
  }
  console.log(`APP_STORE_CAPTURE_OK ${output}`);
  passed = true;
} finally {
  try {
    run("close");
  } catch {}
  server.kill();
  await server.exited;
  const messages = await errors;
  if (!passed && messages) console.error(messages);
  rmSync(temporary, { recursive: true, force: true });
}
