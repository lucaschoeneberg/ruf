import { expect, test } from "bun:test";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

test("clean removes build outputs, preserves inputs and supports preview and full cleanup", () => {
  const root = mkdtempSync(join(tmpdir(), "sip-clean-"));
  const outputs = [
    "target",
    "apps/desktop/dist",
    "apps/desktop/src-tauri/gen",
    "artifacts",
    ".playwright-cli",
    "output/playwright",
  ];
  const caches = [
    "native/build",
    "native/install",
    "node_modules",
    "apps/desktop/node_modules",
  ];
  const inputs = [
    ".env",
    "bun.lock",
    "Cargo.lock",
    "native/sources.json",
    "apps/desktop/src/main.ts",
    "output/keep.txt",
  ];
  try {
    mkdirSync(join(root, "scripts"));
    const script = join(root, "scripts/clean.ts");
    cpSync(resolve(import.meta.dir, "../../scripts/clean.ts"), script);
    for (const path of [...outputs, ...caches]) {
      mkdirSync(join(root, path), { recursive: true });
      writeFileSync(join(root, path, "fixture"), "generated");
    }
    for (const path of inputs) {
      mkdirSync(join(root, path, ".."), { recursive: true });
      writeFileSync(join(root, path), "keep");
    }
    const run = (...args: string[]) =>
      Bun.spawnSync([process.execPath, script, ...args], {
        cwd: tmpdir(),
      });
    expect(run("--unknown").exitCode).not.toBe(0);
    expect(run("--all", "--dry-run").exitCode).toBe(0);
    for (const path of [...outputs, ...caches, ...inputs])
      expect(existsSync(join(root, path))).toBe(true);

    expect(run().exitCode).toBe(0);
    for (const path of outputs)
      expect(existsSync(join(root, path))).toBe(false);
    for (const path of [...caches, ...inputs])
      expect(existsSync(join(root, path))).toBe(true);
    expect(run().exitCode).toBe(0);

    expect(run("--all").exitCode).toBe(0);
    for (const path of caches) expect(existsSync(join(root, path))).toBe(false);
    for (const path of inputs) expect(existsSync(join(root, path))).toBe(true);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("clean rejects symlinked parents before removing any output", () => {
  const fixture = mkdtempSync(join(tmpdir(), "sip-clean-symlink-"));
  const root = join(fixture, "repo");
  const external = join(fixture, "external");
  try {
    mkdirSync(join(root, "scripts"), { recursive: true });
    mkdirSync(join(root, "target"));
    mkdirSync(join(external, "playwright"), { recursive: true });
    const script = join(root, "scripts/clean.ts");
    cpSync(resolve(import.meta.dir, "../../scripts/clean.ts"), script);
    symlinkSync(external, join(root, "output"), "junction");
    for (const args of [["--dry-run"], []]) {
      const result = Bun.spawnSync([process.execPath, script, ...args], {
        cwd: tmpdir(),
      });
      expect(result.exitCode).not.toBe(0);
      expect(existsSync(join(root, "target"))).toBe(true);
      expect(existsSync(join(external, "playwright"))).toBe(true);
    }
  } finally {
    rmSync(fixture, { recursive: true, force: true });
  }
});
