import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const readJson = (path) =>
  JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const manifest = readJson("../package.json");
const version = manifest.version;
const bunVersion = readFileSync(
  new URL("../.bun-version", import.meta.url),
  "utf8",
).trim();
if (
  manifest.packageManager !== `bun@${bunVersion}` ||
  manifest.engines?.bun !== bunVersion
) {
  throw new Error(
    "Bun version pins in package.json and .bun-version must match",
  );
}
if (process.versions.bun !== bunVersion) {
  throw new Error(`Run this workspace with Bun ${bunVersion}`);
}
const rustVersion = /^channel\s*=\s*"([^"]+)"/m.exec(
  readFileSync(new URL("../rust-toolchain.toml", import.meta.url), "utf8"),
)?.[1];
if (!rustVersion) throw new Error("Missing pinned Rust toolchain channel");
for (const executable of ["rustc", "cargo"]) {
  const actual = execFileSync(executable, ["--version"], {
    cwd: root,
    encoding: "utf8",
  }).split(/\s+/)[1];
  if (actual !== rustVersion)
    throw new Error(
      `Run this workspace with Rust ${rustVersion}; ${executable} is ${actual}. Put the pinned Rustup toolchain ahead of system/Homebrew Rust on PATH.`,
    );
}
const frontend = readJson("../apps/desktop/package.json");
const tauri = readJson("../apps/desktop/src-tauri/tauri.conf.json");
const metadata = JSON.parse(
  execFileSync(
    "cargo",
    ["metadata", "--locked", "--no-deps", "--format-version", "1"],
    { cwd: root, encoding: "utf8" },
  ),
);
const members = metadata.packages.filter((pkg) =>
  metadata.workspace_members.includes(pkg.id),
);

if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version))
  throw new Error("Invalid application version");
if (
  frontend.version !== version ||
  members.some((pkg) => pkg.version !== version)
) {
  throw new Error("JavaScript and Cargo workspace versions must match");
}
if (members.some((pkg) => pkg.rust_version !== rustVersion))
  throw new Error(
    "Cargo workspace rust-version must match rust-toolchain.toml",
  );
if (tauri.version !== "../../../package.json")
  throw new Error("Tauri must read the root package version");
if (process.env.RELEASE_TAG && process.env.RELEASE_TAG !== `v${version}`) {
  throw new Error(`Release tag must be v${version}`);
}
console.log(
  `Version ${version} matches JavaScript, Cargo and Tauri${process.env.RELEASE_TAG ? " and release tag" : ""}.`,
);
