// Focused native policy tests; generated executables stay in ignored native/build.
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../", import.meta.url));
const target = {
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
}[`${process.platform}-${process.arch}`];
if (!target)
  throw new Error(
    "Focused C harness supports macOS arm64 and Linux; Windows requires its native compiler runner.",
  );
const upstream = process.argv.includes("--upstream-red");
const sdk = join(root, "native/install", target);
const patched = join(root, "native/build", target);
const output = join(root, "native/build/security-units");
mkdirSync(output, { recursive: true });
const frameworks =
  process.platform === "darwin"
    ? [
        "Security",
        "CoreFoundation",
        "CoreAudio",
        "AudioToolbox",
        "SystemConfiguration",
      ].flatMap((f) => ["-framework", f])
    : ["-lpthread", "-ldl", "-lm"];
for (const [test, macro, project, file] of [
  ["module", "SRTP_SOURCE", "baresip", "modules/srtp/srtp.c"],
  ["dialog", "DIALOG_SOURCE", "re", "src/sip/dialog.c"],
  ["request", "REQUEST_SOURCE", "re", "src/sip/request.c"],
  ["ctrans", "CTRANS_SOURCE", "re", "src/sip/ctrans.c"],
  ["strans", "STRANS_SOURCE", "re", "src/sip/strans.c"],
  ["peer", "PEER_SOURCE", "re", "src/sip/transp.c"],
]) {
  if (upstream && test === "peer") continue; // authority API has a recorded pre-fix wire RED
  const source = join(
    upstream
      ? join(root, "native/build/sources", project)
      : join(patched, `${project}-patched`),
    file,
  );
  const executable = join(output, `${test}-${upstream ? "red" : "green"}`);
  const extra = test === "module" ? [join(source, "../sdes.c")] : [];
  const args = [
    "-std=c11",
    "-DUSE_OPENSSL",
    "-DUSE_TLS",
    ...(upstream ? ["-DSECURITY_UPSTREAM_RED"] : []),
    `-D${macro}="${source}"`,
    `-I${join(sdk, "include")}`,
    `-I${join(sdk, "include/re")}`,
    join(root, `tests/native/security_${test}.c`),
    ...extra,
    `-L${join(sdk, "lib")}`,
    "-lbaresip",
    "-lre",
    "-lssl",
    "-lcrypto",
    "-lresolv",
    ...frameworks,
    "-o",
    executable,
  ];
  execFileSync(process.env.CC ?? "cc", args, {
    cwd: root,
    stdio: ["ignore", "inherit", "pipe"],
  });
  const result = spawnSync(executable, [], { cwd: root, encoding: "utf8" });
  if (upstream) {
    if (result.status === 0) throw new Error(`Expected upstream RED: ${test}`);
    console.log(
      `${test}: expected upstream failure (${result.signal ?? result.status})`,
    );
  } else {
    if (result.status !== 0) throw new Error(`${test}: ${result.stderr}`);
    process.stdout.write(result.stdout);
  }
}
