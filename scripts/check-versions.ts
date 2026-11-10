import { resolve } from "node:path";
import { execFileSync } from "node:child_process";

const root = resolve(import.meta.dir, "..");
const manifest: typeof import("../package.json") = await Bun.file(
  new URL("../package.json", import.meta.url),
).json();
const version = manifest.version;
const bunVersion = (
  await Bun.file(new URL("../.bun-version", import.meta.url)).text()
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
  await Bun.file(new URL("../rust-toolchain.toml", import.meta.url)).text(),
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
const frontend: { version: string } = await Bun.file(
  new URL("../apps/desktop/package.json", import.meta.url),
).json();
const tauri: { version: string } = await Bun.file(
  new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url),
).json();
const metadata: {
  packages: { id: string; version: string; rust_version: string | null }[];
  workspace_members: string[];
} = JSON.parse(
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
