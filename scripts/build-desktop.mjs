import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import {
  mkdirSync,
  readFileSync,
  readdirSync,
  writeFileSync,
  statSync,
} from "node:fs";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { desktopContext } from "./desktop-context.mjs";

await import("./check-versions.mjs");

const root = fileURLToPath(new URL("../", import.meta.url));
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
    join(root, "scripts/tauri-run.mjs"),
    dev ? "dev" : "build",
    ...forwarded,
    "--",
    "--locked",
  ],
  { env: process.env, stdio: "inherit" },
);
if (!dev) {
  const cargo = JSON.parse(
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
  const sbom = readFileSync(exported);
  const bill = JSON.parse(sbom);
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
  const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
  const evidence = {
    build: context,
    sbom: { file: `${basename}.cdx.json`, sha256: digest(sbom) },
    executable: {
      file: binary,
      sha256: digest(readFileSync(join(output, binary))),
    },
    installers: [],
  };
  const destinations = [output];
  if (!args.includes("--no-bundle")) {
    for (const [folder, extension] of [
      ["dmg", ".dmg"],
      ["nsis", ".exe"],
    ]) {
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
        evidence.installers.push({ file, sha256: digest(readFileSync(path)) });
        if (!destinations.includes(directory)) destinations.push(directory);
      }
    }
  }
  for (const directory of destinations) {
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, `${basename}.cdx.json`), sbom);
    writeFileSync(
      join(directory, `${basename}.build.json`),
      JSON.stringify(evidence, null, 2) + "\n",
    );
  }
  console.log(
    `SBOM and build hashes saved for ${context.target} (${context.native ? "native" : "portable"}).`,
  );
}
