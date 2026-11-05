// Real-browser regression checks via the pinned Playwright CLI; no app dependency.
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
  mkdirSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { installDialogFixtures } from "../tests/ui/browser/dialog-fixtures.mjs";
import callControls from "../tests/ui/browser/call-controls.mjs";
import phone from "../tests/ui/browser/phone.mjs";
import settings from "../tests/ui/browser/settings.mjs";
import layout from "../tests/ui/browser/layout.mjs";
import history from "../tests/ui/browser/history.mjs";
import contacts from "../tests/ui/browser/contacts.mjs";

const root = fileURLToPath(new URL("../", import.meta.url));
const temporary = mkdtempSync(join(tmpdir(), "sip-ui-flows-"));
const session = `sip-flows-${process.pid}`;
const cli = [
  "bunx",
  "--package",
  "@playwright/cli@0.1.23",
  "playwright-cli",
  `-s=${session}`,
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
const candidates = existsSync(cache)
  ? readdirSync(cache)
      .filter((name) => name.startsWith("chromium-"))
      .sort()
      .reverse()
      .flatMap((name) => binaries.map((binary) => join(cache, name, binary)))
  : [];
const executable =
  process.env.SIP_UI_BROWSER_EXECUTABLE ?? candidates.find(existsSync);
const config = join(temporary, "browser.json");
writeFileSync(
  config,
  JSON.stringify({
    browser: {
      browserName: "chromium",
      launchOptions: {
        headless: true,
        ...(executable ? { executablePath: executable } : {}),
      },
    },
  }),
);
const run = (...args) => {
  const result = execFileSync(cli[0], [...cli.slice(1), ...args], {
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
  {
    cwd: root,
    stdout: "ignore",
    stderr: "pipe",
    env: {
      ...process.env,
      VITE_RUF_CHANGELOG_URL:
        "https://raw.githubusercontent.com/example/ruf/main/CHANGELOG.md",
    },
  },
);
const serverErrors = new Response(server.stderr).text();
let passed = false;
const url = `http://127.0.0.1:${port}`;
try {
  const deadline = Date.now() + 30000;
  while (true) {
    try {
      if ((await fetch(url, { signal: AbortSignal.timeout(1000) })).ok) break;
    } catch {}
    if (Date.now() > deadline) throw new Error("UI test server did not start");
    await Bun.sleep(100);
  }
  mkdirSync(join(root, "output/playwright"), { recursive: true });
  run("open", url, `--config=${config}`);
  const fixturePath = join(temporary, "fixtures.js");
  writeFileSync(
    fixturePath,
    `async (page) => { await page.context().addInitScript(${installDialogFixtures.toString()}); }`,
  );
  run("run-code", `--filename=${fixturePath}`);

  for (const [name, test] of [
    ["contacts", contacts],
    ["history", history],
    ["phone", phone],
    ["call-controls", callControls],
    ["settings", settings],
    ["layout", layout],
  ]) {
    if (process.env.SIP_UI_FLOW && process.env.SIP_UI_FLOW !== name) continue;
    const path = join(temporary, `${name}.js`);
    writeFileSync(
      path,
      `async (page) => { const result = await (${test.toString()})(page, new URL(page.url()).origin, ${process.env.SIP_UI_CAPTURE === "1"}); if (result && result.passed === false) throw new Error(JSON.stringify(result)); return result ?? { passed: true }; }`,
    );
    const result = run("run-code", `--filename=${path}`);
    console.log(`UI_FLOW_OK ${name}`);
    if (result.includes('"passed": false')) throw new Error(result);
  }
  passed = true;
} finally {
  try {
    run("close");
  } catch {}
  server.kill();
  await server.exited;
  const errors = await serverErrors;
  if (!passed && errors) console.error(errors);
  rmSync(temporary, { recursive: true, force: true });
}
