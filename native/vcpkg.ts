import { execFileSync, spawnSync } from "node:child_process";

/** Fetch a missing pinned registry baseline without changing the checkout. */
export function ensureVcpkgBaseline(directory: string, baseline: string): void {
  const object = `${baseline}:versions/baseline.json`;
  const probe = spawnSync("git", ["cat-file", "-e", object], {
    cwd: directory,
    stdio: "ignore",
  });
  if (probe.status === 0) return;
  execFileSync(
    "git",
    ["fetch", "--no-tags", "--depth", "1", "origin", baseline],
    {
      cwd: directory,
      stdio: "inherit",
    },
  );
  execFileSync("git", ["cat-file", "-e", object], {
    cwd: directory,
    stdio: "inherit",
  });
}
