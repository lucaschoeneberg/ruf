// Verify the signed deliverable; source plist checks cannot detect lost entitlements.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { resolve, join } from "node:path";

if (process.platform !== "darwin") {
  throw new Error("This check requires macOS and a signed Ruf.app bundle.");
}
const bundle = resolve(
  process.argv[2] ?? "target/release/bundle/macos/Ruf.app",
);
const description = execFileSync(
  "/usr/bin/plutil",
  [
    "-extract",
    "NSContactsUsageDescription",
    "raw",
    "-o",
    "-",
    join(bundle, "Contents/Info.plist"),
  ],
  { encoding: "utf8" },
).trim();
assert.ok(
  description.length > 0,
  "The app must explain its contact access request.",
);
const entitlements = execFileSync(
  "/usr/bin/codesign",
  ["--display", "--entitlements", "-", "--xml", bundle],
  { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
);
const signed = JSON.parse(
  execFileSync("/usr/bin/plutil", ["-convert", "json", "-o", "-", "-"], {
    encoding: "utf8",
    input: entitlements,
    stdio: ["pipe", "pipe", "pipe"],
  }),
);
assert.equal(
  signed["com.apple.security.personal-information.addressbook"],
  true,
  "The signed app is missing the macOS Address Book entitlement. Hardened Runtime can block the permission dialog.",
);
execFileSync("/usr/bin/codesign", ["--verify", "--deep", "--strict", bundle], {
  stdio: "pipe",
});
console.log(
  "MACOS_CONTACTS_BUNDLE_OK: usage description, Address Book entitlement and code signature verified.",
);
