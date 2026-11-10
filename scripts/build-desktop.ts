import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import type { SoftwareBill } from "../apps/desktop/src/lib/software-bill.ts";
import { desktopContext } from "./desktop-context.ts";

await import("./check-versions.ts");

const root = resolve(import.meta.dir, "..");
const args = process.argv.slice(2);
const dev = args.includes("--dev");
const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(
  /^host: (.+)$/m,
)?.[1];
if (!host) throw new Error("Cannot determine Rust host target.");
const context = desktopContext(args, host, process.env.CARGO_BUILD_TARGET);
process.env.RUF_SBOM_CONTEXT = JSON.stringify(context);
process.env.RUF_SBOM_ENFORCE = dev ? "0" : "1";
const forwarded = args.filter((arg) => arg !== "--dev");
if (
  !forwarded.some(
    (arg) => arg === "--target" || arg === "-t" || arg.startsWith("--target="),
  )
)
  forwarded.push("--target", context.target);
process.chdir(join(root, "apps/desktop"));
const started = Date.now();
execFileSync(
  process.execPath,
  [
    join(root, "scripts/tauri-run.ts"),
    dev ? "dev" : "build",
    ...forwarded,
    "--",
    "--locked",
  ],
  { env: process.env, stdio: "inherit" },
);
if (!dev) {
  const cargo: { target_directory: string } = JSON.parse(
    execFileSync(
      "cargo",
      ["metadata", "--locked", "--no-deps", "--format-version", "1"],
      { cwd: root, encoding: "utf8" },
    ),
  );
  const output = join(cargo.target_directory, context.target, context.profile);
  const binary = context.target.includes("windows")
    ? "sip-desktop.exe"
    : "sip-desktop";
  const exported = join(output, "software-bill.cdx.json");
  execFileSync(join(output, binary), ["--export-sbom", exported], {
    stdio: "pipe",
  });
  const sbom = await Bun.file(exported).bytes();
  const bill: SoftwareBill = JSON.parse(new TextDecoder().decode(sbom));
  for (const [name, value] of Object.entries(context))
    if (
      !bill.metadata.properties.some(
        (property) =>
          property.name === `ruf:build:${name}` &&
          property.value === String(value),
      )
    )
      throw new Error(
        "Built SBOM context differs from the requested desktop build.",
      );
  const version = bill.metadata.component.version;
  const basename = `Ruf_${version}_${context.target}_${context.native ? "native" : "portable"}`;
  const digest = (bytes: Uint8Array): string =>
    new Bun.CryptoHasher("sha256").update(bytes).digest("hex");
  const installers: { file: string; sha256: string }[] = [];
  const evidence = {
    build: context,
    sbom: { file: `${basename}.cdx.json`, sha256: digest(sbom) },
    executable: {
      file: binary,
      sha256: digest(await Bun.file(join(output, binary)).bytes()),
    },
    installers,
  };
  const destinations = [output];
  if (!args.includes("--no-bundle")) {
    for (const [folder, extension] of [
      ["dmg", ".dmg"],
      ["nsis", ".exe"],
    ] as const) {
      const directory = join(output, "bundle", folder);
      let files;
      try {
        files = readdirSync(directory);
      } catch {
        continue;
      }
      for (const file of files) {
        const path = join(directory, file);
        if (!file.endsWith(extension) || statSync(path).mtimeMs < started)
          continue;
        evidence.installers.push({
          file,
          sha256: digest(await Bun.file(path).bytes()),
        });
        if (!destinations.includes(directory)) destinations.push(directory);
      }
    }
  }
  for (const directory of destinations) {
    mkdirSync(directory, { recursive: true });
    await Bun.write(join(directory, `${basename}.cdx.json`), sbom);
    await Bun.write(
      join(directory, `${basename}.build.json`),
      JSON.stringify(evidence, null, 2) + "\n",
    );
  }
  console.log(
    `SBOM and build hashes saved for ${context.target} (${context.native ? "native" : "portable"}).`,
  );
}
