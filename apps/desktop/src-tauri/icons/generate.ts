// Reuse the canonical Ruf vector mark; never upscale the old 512px bitmap.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dir, "../../../..");
const output = import.meta.dir;
const source = join(root, "apps/desktop/public/ruf.svg");
const temporary = mkdtempSync(join(tmpdir(), "ruf-icons-"));
const sizes = [16, 32, 48, 64, 128, 256, 512, 1024];
const generate = (destination: string, args: string[] = []) =>
  execFileSync(
    process.execPath,
    [
      join(root, "scripts/tauri-run.ts"),
      "icon",
      source,
      "--output",
      destination,
      ...args,
    ],
    { cwd: root, stdio: "inherit" },
  );

try {
  const native = join(temporary, "native");
  const png = join(temporary, "png");
  generate(native);
  generate(
    png,
    sizes.flatMap((size) => ["--png", String(size)]),
  );
  for (const filename of ["icon.icns", "icon.ico", "128x128@2x.png"])
    cpSync(join(native, filename), join(output, filename));
  for (const size of sizes.filter((size) => size !== 1024)) {
    const filename = `${size}x${size}.png`;
    cpSync(join(png, filename), join(output, filename));
  }
  // Tauri's default icon.png is 512px; retain the direct 1024px vector render.
  cpSync(join(png, "1024x1024.png"), join(output, "icon.png"));
  const artwork = join(root, "artifacts/app-store/icons");
  mkdirSync(artwork, { recursive: true });
  cpSync(join(output, "icon.png"), join(artwork, "ruf-1024.png"));
  cpSync(source, join(artwork, "ruf.svg"));
  console.log("Ruf desktop icons generated from the existing SVG mark.");
} finally {
  // Only this invocation's temporary Tauri outputs, including unused mobile icons.
  rmSync(temporary, { recursive: true, force: true });
}
