import { lstatSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(import.meta.dir, "..");
const args = process.argv.slice(2);
const allowed = new Set(["--all", "--dry-run"]);
for (const arg of args) {
  if (!allowed.has(arg))
    throw new Error(
      `Unknown argument: ${arg}. Usage: bun run clean [--all] [--dry-run]`,
    );
}

// Keep cleanup scoped to known, repository-local generated directories.
const paths = [
  "target",
  "apps/desktop/dist",
  "apps/desktop/src-tauri/gen",
  "artifacts",
  ".playwright-cli",
  "output/playwright",
];
if (args.includes("--all"))
  paths.push(
    "native/build",
    "native/install",
    "node_modules",
    "apps/desktop/node_modules",
  );

const dryRun = args.includes("--dry-run");
// Validate every parent before deleting anything; rmSync does not follow a final
// symlink, but would otherwise traverse a symlink earlier in a nested path.
for (const path of paths) {
  const parts = path.split("/");
  for (let depth = 1; depth < parts.length; depth++) {
    const parent = resolve(root, ...parts.slice(0, depth));
    if (lstatSync(parent, { throwIfNoEntry: false })?.isSymbolicLink())
      throw new Error(`Refusing cleanup through a symlink: ${parent}`);
  }
}
for (const path of paths) {
  if (!dryRun) rmSync(resolve(root, path), { recursive: true, force: true });
  console.log(`${dryRun ? "Would remove" : "Cleaned"} ${path}`);
}
