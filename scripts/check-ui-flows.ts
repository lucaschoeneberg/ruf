// Real-browser regression checks via the pinned Playwright CLI; no app dependency.
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
import { installDialogFixtures } from "../tests/ui/browser/dialog-fixtures.ts";
import callControls from "../tests/ui/browser/call-controls.ts";
import shortcuts from "../tests/ui/browser/shortcuts.ts";
import globalSearch from "../tests/ui/browser/global-search.ts";
import dialerIdentity from "../tests/ui/browser/dialer-identity.ts";
import phone from "../tests/ui/browser/phone.ts";
import settings from "../tests/ui/browser/settings.ts";
import layout from "../tests/ui/browser/layout.ts";
import history from "../tests/ui/browser/history.ts";
import contacts from "../tests/ui/browser/contacts.ts";
import {
  englishSmoke,
  englishExpectedLabels,
} from "../tests/ui/browser/english-smoke.ts";
import localization from "../tests/ui/browser/localization.ts";

const root = resolve(import.meta.dir, "..");
const expectedEnglishLabels = englishExpectedLabels();
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
await Bun.write(
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
const run = (...args: string[]) => {
  const result = execFileSync(cli[0]!, [...cli.slice(1), ...args], {
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
      VITE_RUF_CHANGELOG_URL_DE:
        "https://raw.githubusercontent.com/example/ruf/main/CHANGELOG.de.md",
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
  const selectedLanguage = process.env.SIP_UI_LANGUAGE;
  if (
    selectedLanguage &&
    selectedLanguage !== "de" &&
    selectedLanguage !== "en"
  )
    throw new Error("SIP_UI_LANGUAGE must be de or en");
  const languages = selectedLanguage ? [selectedLanguage] : ["de", "en"];
  for (const [index, language] of languages.entries()) {
    if (index > 0) {
      run("close");
      run("open", url, `--config=${config}`);
    }
    // Separate contexts retain every original DE regression and isolate EN state.
    const fixturePath = join(temporary, `${language}-fixtures.js`);
    await Bun.write(
      fixturePath,
      `async (page) => { await page.context().addInitScript(${installDialogFixtures.toString()}, ${JSON.stringify(language)}); }`,
    );
    run("run-code", `--filename=${fixturePath}`);
    for (const [name, test] of [
      ["contacts", contacts],
      ["history", history],
      ["phone", phone],
      ["call-controls", callControls],
      ["shortcuts", shortcuts],
      ["global-search", globalSearch],
      ["dialer-identity", dialerIdentity],
      ["settings", settings],
      ["layout", layout],
    ] as const) {
      if (process.env.SIP_UI_FLOW && process.env.SIP_UI_FLOW !== name) continue;
      const path = join(temporary, `${language}-${name}.js`);
      await Bun.write(
        path,
        `async (page) => { const englishScenario = ${language === "en" ? `async (target, flow, origin) => (${englishSmoke.toString()})(target, flow, origin, ${JSON.stringify(expectedEnglishLabels)})` : "undefined"}; const result = await (${test.toString()})(page, new URL(page.url()).origin, ${process.env.SIP_UI_CAPTURE === "1"}, englishScenario); if (result && result.passed === false) throw new Error(JSON.stringify(result)); return result ?? { passed: true }; }`,
      );
      const result = run("run-code", `--filename=${path}`);
      console.log(`UI_FLOW_OK ${language} ${name}`);
      if (result.includes('"passed": false')) throw new Error(result);
    }
  }
  if (!process.env.SIP_UI_FLOW || process.env.SIP_UI_FLOW === "localization") {
    const path = join(temporary, "localization.js");
    await Bun.write(
      path,
      `async (page) => (${localization.toString()})(page, new URL(page.url()).origin)`,
    );
    const result = run("run-code", `--filename=${path}`);
    if (result.includes('"passed": false')) throw new Error(result);
    console.log("UI_FLOW_OK localization multiwindow");
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
