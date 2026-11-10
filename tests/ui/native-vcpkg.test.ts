import { expect, test } from "bun:test";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { ensureVcpkgBaseline } from "../../native/vcpkg.ts";

test("vcpkg fetches a missing pinned baseline without changing its checkout", () => {
  const root = mkdtempSync(join(tmpdir(), "ruf-vcpkg-"));
  const origin = join(root, "origin");
  const runner = join(root, "runner");
  const git = (cwd: string, ...args: string[]) =>
    execFileSync("git", args, {
      cwd,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  try {
    mkdirSync(origin);
    git(origin, "init", "--quiet");
    git(origin, "config", "user.name", "Fixture");
    git(origin, "config", "user.email", "fixture@example.invalid");
    writeFileSync(join(origin, "README"), "old runner snapshot\n");
    git(origin, "add", ".");
    git(origin, "commit", "--quiet", "-m", "old snapshot");
    git(
      root,
      "clone",
      "--quiet",
      "--depth",
      "1",
      pathToFileURL(origin).href,
      runner,
    );
    const original = git(runner, "rev-parse", "HEAD");
    mkdirSync(join(origin, "versions"));
    const baselineJson =
      '{"default":{"openssl":{"baseline":"3.5.9","port-version":0}}}';
    writeFileSync(join(origin, "versions/baseline.json"), baselineJson);
    git(origin, "add", ".");
    git(origin, "commit", "--quiet", "-m", "pinned baseline");
    const baseline = git(origin, "rev-parse", "HEAD");
    expect(
      spawnSync(
        "git",
        ["cat-file", "-e", `${baseline}:versions/baseline.json`],
        { cwd: runner, stdio: "ignore" },
      ).status,
    ).not.toBe(0);
    ensureVcpkgBaseline(runner, baseline);
    expect(git(runner, "show", `${baseline}:versions/baseline.json`)).toBe(
      baselineJson,
    );
    expect(git(runner, "rev-parse", "HEAD")).toBe(original);
    git(runner, "remote", "set-url", "origin", join(root, "offline"));
    expect(() => ensureVcpkgBaseline(runner, baseline)).not.toThrow();
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
