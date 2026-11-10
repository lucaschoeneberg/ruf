// Verify the shipped signature, not just the source configuration.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { join } from "node:path";

assert.equal(
  process.platform,
  "darwin",
  "This check requires macOS codesign/plutil",
);
const bundle = process.argv[2];
assert.ok(bundle, "Pass the .app bundle path");
execFileSync("codesign", ["--verify", "--strict", bundle], { stdio: "pipe" });
const parsePlist = (bytes: string) =>
  JSON.parse(
    execFileSync("plutil", ["-convert", "json", "-o", "-", "-"], {
      input: bytes,
      encoding: "utf8",
    }),
  );
const signature = execFileSync(
  "codesign",
  ["-d", "--entitlements", "-", "--xml", bundle],
  { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
);
assert.ok(
  signature.trim(),
  "Signed app lacks microphone entitlements (hardened runtime blocks capture)",
);
const entitlements = parsePlist(signature);
assert.equal(
  entitlements["com.apple.security.device.audio-input"],
  true,
  "Signed app must permit native audio input",
);
const info = JSON.parse(
  execFileSync(
    "plutil",
    ["-convert", "json", "-o", "-", join(bundle, "Contents", "Info.plist")],
    { encoding: "utf8" },
  ),
);
assert.ok(
  typeof info.NSMicrophoneUsageDescription === "string" &&
    info.NSMicrophoneUsageDescription.trim(),
  "Microphone purpose string is required for macOS consent",
);
console.log(
  "MACOS_AUDIO_PERMISSION_OK: signed entitlement and microphone purpose string",
);
