import { resolve } from "node:path";
// Native Contacts failure check; never requests permission or reads real contacts.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";

assert.equal(process.platform, "darwin", "Requires macOS and clang");
const root = resolve(import.meta.dir, "..");
mkdirSync(new URL("../native/build/", import.meta.url), { recursive: true });
const binary = resolve(root, "native/build/contact-timeouts");
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
