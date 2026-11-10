// Render the authored vector with the existing browser and bundled Ruf fonts.
// macOS: bun apps/desktop/src-tauri/dmg/generate.ts
import { execFileSync } from "node:child_process";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { chromium } from "playwright-core";

if (process.platform !== "darwin")
  throw new Error(
    "DMG artwork generation requires macOS sips for Retina DPI metadata.",
  );
const root = resolve(import.meta.dir, "../../../..");
const file = (path: string) => Bun.file(join(root, path));
const base64 = async (path: string) =>
  Buffer.from(await file(path).arrayBuffer()).toString("base64");
const source = await Bun.file(join(import.meta.dir, "background.svg")).text();
const svg = source.replace(
  'href="../../public/ruf.svg"',
  `href="data:image/svg+xml;base64,${await base64("apps/desktop/public/ruf.svg")}"`,
);
const faces = await Promise.all(
  [
    ["Regular", 400],
    ["SemiBold", 600],
  ].map(
    async ([name, weight]) =>
      `@font-face{font-family:'SIP UI';font-weight:${weight};src:url(data:font/ttf;base64,${await base64(`apps/desktop/public/fonts/IBMPlexSans-${name}.ttf`)})}`,
  ),
);
const cache = join(homedir(), "Library/Caches/ms-playwright");
const executablePath =
  process.env.SIP_UI_BROWSER_EXECUTABLE ??
  [
    ...new Bun.Glob(
      "chromium-*/chrome-*/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
    ).scanSync({ cwd: cache, absolute: true }),
  ]
    .sort()
    .reverse()[0];
const browser = await chromium.launch({
  headless: true,
  ...(executablePath ? { executablePath } : {}),
});
try {
  const page = await browser.newPage({
    viewport: { width: 720, height: 480 },
    deviceScaleFactor: 2,
  });
  await page.setContent(
    `<style>${faces.join("")}body{margin:0}svg{display:block}</style>${svg}`,
  );
  await page.evaluate(() => document.fonts.ready);
  await page.locator("image").evaluate(async (image) => {
    const href = image.getAttribute("href");
    if (!href) throw new Error("Missing Ruf artwork");
    const loaded = new Image();
    loaded.src = href;
    await loaded.decode();
  });
  const output = join(import.meta.dir, "background.png");
  await page.screenshot({ path: output, animations: "disabled" });
  execFileSync(
    "sips",
    [
      "--setProperty",
      "dpiWidth",
      "144",
      "--setProperty",
      "dpiHeight",
      "144",
      output,
    ],
    { stdio: "pipe" },
  );
  console.log(
    "DMG_BACKGROUND_OK: 1440×960 Retina PNG, authored SVG, bundled IBM Plex Sans and canonical Ruf mark.",
  );
} finally {
  await browser.close();
}
