import { execFileSync, spawnSync } from "node:child_process";

// Runner images can contain an older/shallow registry than the pinned manifest.
export function ensureVcpkgBaseline(directory, baseline) {
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
