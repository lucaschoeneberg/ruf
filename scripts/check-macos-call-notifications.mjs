// Exercise native OS storage with public fixture data and a separate bundle identity.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

assert.equal(process.platform, "darwin", "Requires macOS and clang");
const args = process.argv.slice(2);
assert.ok(
  args.every((arg) => arg === "--request-permission"),
  "Unknown argument",
);
const root = fileURLToPath(new URL("../", import.meta.url));
const bundle = join(root, "native/build/notifications-smoke.app");
const contents = join(bundle, "Contents");
const executable = join(contents, "MacOS/RufNotificationsSmoke");
mkdirSync(join(contents, "MacOS"), { recursive: true });
writeFileSync(
  join(contents, "Info.plist"),
  `<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.rufapp.notification-smoke</string>
<key>CFBundleExecutable</key><string>RufNotificationsSmoke</string>
<key>CFBundleName</key><string>Ruf Notification Test</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>1</string>
<key>LSMinimumSystemVersion</key><string>14.0</string>
<key>LSUIElement</key><true/></dict></plist>`,
);
const run = (command, parameters, timeout = 60_000) =>
  execFileSync(command, parameters, { cwd: root, stdio: "inherit", timeout });
run("clang", [
  "-fobjc-arc",
  "-fblocks",
  "-Wall",
  "-Wextra",
  "-Werror",
  "-mmacosx-version-min=14.0",
  "-I",
  "crates/baresip-adapter/native",
  "crates/baresip-adapter/native/call_notifications.m",
  "crates/baresip-adapter/native/tests/notifications_smoke.m",
  "-framework",
  "AppKit",
  "-framework",
  "Foundation",
  "-framework",
  "UserNotifications",
  "-o",
  executable,
]);
run("codesign", ["--force", "--sign", "-", bundle]);
// Foreground consent can take longer. No consent prompt without the explicit flag.
run(executable, args, args.length ? 180_000 : 30_000);
