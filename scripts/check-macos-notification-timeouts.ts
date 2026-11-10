import { resolve } from "node:path";
// Deterministic native service-failure check, without requesting OS authorization.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";

assert.equal(process.platform, "darwin", "Requires macOS and clang");
const root = resolve(import.meta.dir, "..");
mkdirSync(new URL("../native/build/", import.meta.url), { recursive: true });
const binary = resolve(root, "native/build/notification-timeouts");
execFileSync(
  "clang",
  [
    "-fobjc-arc",
    "-fblocks",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-mmacosx-version-min=14.0",
    "crates/baresip-adapter/native/tests/notification_timeouts.m",
    "-framework",
    "Foundation",
    "-framework",
    "UserNotifications",
    "-o",
    binary,
  ],
  { cwd: root, stdio: "inherit", timeout: 60_000 },
);
execFileSync(binary, [], { cwd: root, stdio: "inherit", timeout: 2_000 });
