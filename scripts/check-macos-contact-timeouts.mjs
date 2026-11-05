// Native Contacts failure check; never requests permission or reads real contacts.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

assert.equal(process.platform, "darwin", "Requires macOS and clang");
const root = fileURLToPath(new URL("../", import.meta.url));
mkdirSync(new URL("../native/build/", import.meta.url), { recursive: true });
const binary = fileURLToPath(
  new URL("../native/build/contact-timeouts", import.meta.url),
);
execFileSync(
  "clang",
  [
    "-fobjc-arc",
    "-fblocks",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-mmacosx-version-min=14.0",
    "crates/baresip-adapter/native/tests/contact_timeouts.m",
    "-framework",
    "AppKit",
    "-framework",
    "Foundation",
    "-framework",
    "Contacts",
    "-o",
    binary,
  ],
  { cwd: root, stdio: "inherit", timeout: 60_000 },
);
execFileSync(binary, [], { cwd: root, stdio: "inherit", timeout: 2_000 });
