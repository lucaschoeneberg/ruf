// Reuse real UI components and the existing isolated screenshot fixture.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright-core";
import { installAppStoreFixtures } from "../tests/ui/browser/app-store-fixtures.ts";

const root = resolve(import.meta.dir, "..");
const manifest: typeof import("../package.json") = await Bun.file(
  join(root, "package.json"),
).json();
const version = manifest.version;
const output = join(root, "docs/screenshots");
const scratch = join(root, "output/playwright/repository");
const media = join(root, "artifacts/release-preparation/media");
if (process.argv.length > 2) throw new Error("No arguments are supported.");
execFileSync("ffmpeg", ["-version"], { stdio: "ignore" });
for (const directory of [output, scratch, media])
  mkdirSync(directory, { recursive: true });
const cache = join(homedir(), "Library/Caches/ms-playwright");
const executablePath =
  process.env.SIP_UI_BROWSER_EXECUTABLE ??
  (existsSync(chromium.executablePath())
    ? chromium.executablePath()
    : existsSync(cache)
      ? [
          ...new Bun.Glob(
            "chromium-*/chrome-*/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
          ).scanSync({ cwd: cache, absolute: true }),
        ]
          .sort()
          .reverse()[0]
      : undefined);
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
const serverErrors = new Response(server.stderr).text();
const url = `http://127.0.0.1:${port}`;
let browser: Awaited<ReturnType<typeof chromium.launch>> | undefined;
let passed = false;
try {
  const deadline = Date.now() + 30000;
  while (true) {
    try {
      if ((await fetch(url, { signal: AbortSignal.timeout(1000) })).ok) break;
    } catch {}
    if (Date.now() > deadline)
      throw new Error("Repository preview server did not start.");
    await Bun.sleep(100);
  }
  browser = await chromium.launch({
    headless: true,
    ...(executablePath ? { executablePath } : {}),
  });
  const context = await browser.newContext({
    viewport: { width: 360, height: 560 },
    deviceScaleFactor: 2,
    locale: "en-GB",
    timezoneId: "Europe/Berlin",
    reducedMotion: "reduce",
  });
  await context.addInitScript({
    content: `window.__rufTestLanguage = "en"; (${installAppStoreFixtures.toString()})(${JSON.stringify(version)});`,
  });
  const page = await context.newPage();
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  // Block all off-origin traffic, including optional online changelogs.
  await context.route("**/*", (route) => {
    const requested = new URL(route.request().url());
    return requested.origin === url || requested.protocol === "data:"
      ? route.continue()
      : route.abort();
  });
  const frames: {
    name: string;
    heading: string;
    detail: string;
    image: string;
  }[] = [];
  const capture = async (name: string, heading: string, detail: string) => {
    await page.evaluate(() => document.fonts.ready);
    await page.evaluate(() => {
      if (document.activeElement instanceof HTMLElement)
        document.activeElement.blur();
    });
    await page.mouse.move(0, 0);
    await page.waitForTimeout(250);
    if (
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      )
    )
      throw new Error(`Horizontal overflow in ${name}`);
    const commands = await page.evaluate(() =>
      window.__appStoreFixture.commands.map((item) => item.command),
    );
    if (
      commands.some(
        (command) =>
          command.startsWith("request_") ||
          ["dial", "answer", "start_audio_test"].includes(command),
      )
    )
      throw new Error(`Unexpected permission/call/audio action in ${name}`);
    const png = await page.screenshot({
      path: join(output, `${name}.png`),
      animations: "disabled",
    });
    frames.push({
      name,
      heading,
      detail,
      image: `data:image/png;base64,${png.toString("base64")}`,
    });
  };
  await page.goto(url);
  await page.getByText("Registered", { exact: true }).waitFor();
  await page
    .getByRole("textbox", { name: "Number or extension", exact: true })
    .fill("1201");
  await capture(
    "dialer",
    "Choose your calling account.",
    "Keep up to eight SIP identities in one compact phone.",
  );
  await page.getByRole("button", { name: "Contacts", exact: true }).click();
  await page.getByText("Anna Beispiel", { exact: true }).waitFor();
  await capture(
    "contacts",
    "Find the person you need.",
    "Search macOS contacts and keep favorites close.",
  );
  await page.goto(url);
  await page.getByText("Registered", { exact: true }).waitFor();
  await page.evaluate(() => window.__appStoreFixture.setCall("active"));
  await page.getByText("Call in progress", { exact: true }).waitFor();
  await capture(
    "call",
    "Keep call controls within reach.",
    "Mute, hold, keypad and transfer controls in one view.",
  );
  await page.goto(url);
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page.getByText("Anna Beispiel", { exact: true }).waitFor();
  await capture(
    "history",
    "Pick up where you left off.",
    "Search local history and return calls through the original account.",
  );
  await page.setViewportSize({ width: 900, height: 660 });
  await page.goto(`${url}/?window=settings&section=audio`);
  await page.getByRole("heading", { name: "Sound", exact: true }).waitFor();
  await capture(
    "settings",
    "Set up your audio.",
    "Choose separate microphone, call and ringtone outputs.",
  );
  if (errors.length) throw new Error(errors.join("\n"));

  const stage = await browser.newPage({
    viewport: { width: 960, height: 640 },
    deviceScaleFactor: 1,
  });
  const font = Buffer.from(
    await Bun.file(
      join(root, "apps/desktop/public/fonts/IBMPlexSans-Regular.ttf"),
    ).arrayBuffer(),
  ).toString("base64");
  const semibold = Buffer.from(
    await Bun.file(
      join(root, "apps/desktop/public/fonts/IBMPlexSans-SemiBold.ttf"),
    ).arrayBuffer(),
  ).toString("base64");
  const icon = Buffer.from(
    await Bun.file(join(root, "apps/desktop/public/ruf.svg")).arrayBuffer(),
  ).toString("base64");
  const css = `@font-face{font-family:Plex;src:url(data:font/ttf;base64,${font})}@font-face{font-family:Plex;font-weight:600;src:url(data:font/ttf;base64,${semibold})}*{box-sizing:border-box}body{margin:0;background:#0d1117;color:#f0f6fc;font-family:Plex,sans-serif}main{height:640px;display:grid;grid-template-columns:1fr 360px;align-items:center;gap:34px;padding:32px 48px}.brand{display:flex;align-items:center;gap:12px;font-size:24px;font-weight:600}.brand img{width:48px}small{color:#58a6ff;font-size:13px}h1{font-size:34px;font-weight:600;line-height:1.12;margin:26px 0 18px}p{font-size:17px;color:#c9d1d9;line-height:1.5;margin:0}.window{overflow:hidden;border:1px solid #30363d;border-radius:8px;box-shadow:0 16px 40px #0008}.title{font-size:12px;color:#9198a1;background:#161b22;text-align:center;padding:7px}.screen{display:block;width:100%;height:auto}.note{font-size:12px;color:#9198a1;margin-top:28px}.tag{display:inline-block;margin-top:18px;border:1px solid #30363d;padding:6px 10px;border-radius:4px;font-size:13px;color:#c9d1d9}`;
  for (const [index, frame] of frames.entries()) {
    const layout =
      frame.name === "settings"
        ? "main{grid-template-columns:260px 1fr;gap:30px;padding:32px}h1{font-size:30px}p{font-size:16px}"
        : "";
    await stage.setContent(
      `<style>${css}${layout}</style><main><section><div class="brand"><img src="data:image/svg+xml;base64,${icon}" alt="">Ruf</div><small>${version} DEVELOPER PREVIEW</small><h1>${frame.heading}</h1><p>${frame.detail}</p><div class="tag">Rust · Tauri · Svelte · baresip</div><p class="note">UI preview with sample data.<br>No live call or audio in this walkthrough.</p></section><div class="window"><div class="title">Ruf</div><img class="screen" src="${frame.image}" alt="Ruf interface"></div></main>`,
    );
    await stage.evaluate(() => document.fonts.ready);
    await stage.locator("img").evaluateAll(async (images) => {
      await Promise.all(
        images.map((image) => (image as HTMLImageElement).decode()),
      );
    });
    await stage.screenshot({
      path: join(scratch, `frame-${String(index).padStart(2, "0")}.png`),
      animations: "disabled",
    });
  }
  const first = frames[0]!;
  await stage.setViewportSize({ width: 1280, height: 640 });
  await stage.setContent(
    `<style>${css}main{grid-template-columns:1fr 360px;gap:70px;padding:32px 90px}h1{font-size:52px}p{font-size:21px}</style><main><section><div class="brand"><img src="data:image/svg+xml;base64,${icon}" alt="">Ruf</div><h1>Your SIP account.<br>A compact desktop phone.</h1><p>Open source. Rust, Tauri and Svelte.<br>macOS and Windows targets.</p><div class="tag">${version} Developer Preview · Free</div><p class="note">github.com/mardu-systems/ruf<br>Interface preview with sample data.</p></section><div class="window"><div class="title">Ruf</div><img class="screen" src="${first.image}" alt="Ruf dialer"></div></main>`,
  );
  await stage.evaluate(() => document.fonts.ready);
  await stage.locator("img").evaluateAll(async (images) => {
    await Promise.all(
      images.map((image) => (image as HTMLImageElement).decode()),
    );
  });
  await stage.screenshot({
    path: join(output, "social-preview.png"),
    animations: "disabled",
  });
  execFileSync(
    "ffmpeg",
    [
      "-y",
      "-framerate",
      "1/6",
      "-i",
      join(scratch, "frame-%02d.png"),
      "-filter_complex",
      "split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse",
      "-loop",
      "0",
      join(output, "demo.gif"),
    ],
    { stdio: "pipe" },
  );
  execFileSync(
    "ffmpeg",
    [
      "-y",
      "-framerate",
      "1/6",
      "-i",
      join(scratch, "frame-%02d.png"),
      "-c:v",
      "libx264",
      "-r",
      "15",
      "-pix_fmt",
      "yuv420p",
      "-movflags",
      "+faststart",
      join(media, `ruf-${version}-ui-demo.mp4`),
    ],
    { stdio: "pipe" },
  );
  await Bun.write(
    join(scratch, "capture-report.json"),
    JSON.stringify(
      {
        sampleData: true,
        locale: "en",
        errors,
        frames: frames.map(({ name, heading }) => ({ name, heading })),
      },
      null,
      2,
    ) + "\n",
  );
  console.log(`REPOSITORY_MEDIA_OK ${output}`);
  passed = true;
} finally {
  await browser?.close();
  server.kill();
  await server.exited;
  const messages = await serverErrors;
  if (!passed && messages) console.error(messages);
}
