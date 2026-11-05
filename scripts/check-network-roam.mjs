import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const version = execFileSync("rustc", ["-vV"], {
  cwd: root,
  encoding: "utf8",
});
const host = /^host: (.+)$/m.exec(version)?.[1];
if (!host) throw new Error("Cannot determine the native Rust host");
const prefix = join(root, "native/install", host);
if (!existsSync(join(prefix, "sdk.json")))
  throw new Error("Native SDK missing; run bun run native:build first");

const windows = process.platform === "win32";
const build = mkdtempSync(join(tmpdir(), "sip-network-roam-"));
const run = (command, args) =>
  execFileSync(command, args, { cwd: root, stdio: "inherit" });
try {
  run("cmake", [
    "-S",
    join(root, "crates/baresip-adapter/native/tests"),
    "-B",
    build,
    `-DSIP_SDK_PREFIX=${prefix}`,
    "-DCMAKE_BUILD_TYPE=Release",
    ...(windows ? ["-A", "x64"] : []),
  ]);
  run("cmake", ["--build", build, "--config", "Release"]);
  run(
    join(
      build,
      windows ? "Release/network_roam_test.exe" : "network_roam_test",
    ),
    [],
  );
  run(
    join(
      build,
      windows ? "Release/call_actions_test.exe" : "call_actions_test",
    ),
    [],
  );
} finally {
  rmSync(build, { recursive: true, force: true });
}
