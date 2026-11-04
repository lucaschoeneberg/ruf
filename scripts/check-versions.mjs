import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const readJson = (path) =>
  JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const version = readJson("../package.json").version;
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
  throw new Error("npm and Cargo workspace versions must match");
}
if (tauri.version !== "../../../package.json")
  throw new Error("Tauri must read the root package version");
if (process.env.RELEASE_TAG && process.env.RELEASE_TAG !== `v${version}`) {
  throw new Error(`Release tag must be v${version}`);
}
console.log(
  `Version ${version} matches npm, Cargo and Tauri${process.env.RELEASE_TAG ? " and release tag" : ""}.`,
);
