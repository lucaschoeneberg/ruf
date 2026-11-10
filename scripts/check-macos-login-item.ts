// Exercise the production ServiceManagement bridge with synthetic services only.
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

if (process.platform !== "darwin")
  throw new Error("This check requires macOS.");
const root = resolve(import.meta.dir, "..");
const temporary = mkdtempSync(join(tmpdir(), "ruf-login-item-"));
const executable = join(temporary, "login-item-test");
try {
  execFileSync(
    "clang",
    [
      "-fobjc-arc",
      "-fblocks",
      "-Wall",
      "-Wextra",
      "-Werror",
      "-mmacosx-version-min=14.0",
      join(root, "crates/baresip-adapter/native/tests/login_item.m"),
      "-framework",
      "AppKit",
      "-framework",
      "Foundation",
      "-framework",
      "ServiceManagement",
      "-o",
      executable,
    ],
    { cwd: root, stdio: "inherit", timeout: 60000 },
  );
  execFileSync(executable, [], { stdio: "inherit", timeout: 10000 });
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
