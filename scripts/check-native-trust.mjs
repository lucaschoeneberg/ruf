// Run the exact Linux C verifier branch on a generated certificate chain.
import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../", import.meta.url));
const target = {
  "darwin-arm64": "aarch64-apple-darwin",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "linux-arm64": "aarch64-unknown-linux-gnu",
}[`${process.platform}-${process.arch}`];
if (!target)
  throw new Error("Linux branch C harness requires macOS/Linux compiler");
const build = join(root, "native/build/linux-trust-test");
rmSync(build, { recursive: true, force: true });
mkdirSync(build, { recursive: true });
execFileSync("python3", [
  join(root, "tests/native/tls_peer.py"),
  build,
  "certificates",
]);
const source = readFileSync(
  join(root, "crates/baresip-adapter/native/security.c"),
  "utf8",
);
const branch = source.match(
  /#else\n(static int platform_verify[\s\S]*?)\n#endif\nstatic int verify_server/,
);
if (!branch) throw new Error("Linux verifier branch not found");
const extracted = join(build, "linux-verifier.c");
writeFileSync(extracted, branch[1]);
const hash = execFileSync(
  "openssl",
  ["x509", "-hash", "-noout", "-in", join(build, "ca.pem")],
  { encoding: "utf8" },
).trim();
const anchors = join(build, "anchors");
mkdirSync(anchors);
copyFileSync(join(build, "ca.pem"), join(anchors, `${hash}.0`));
const sdk = join(root, "native/install", target);
const binary = join(build, "verify");
execFileSync(process.env.CC ?? "cc", [
  "-std=c11",
  `-DLINUX_TRUST_SOURCE="${extracted}"`,
  `-I${join(sdk, "include")}`,
  join(root, "tests/native/security_linux_trust.c"),
  `-L${join(sdk, "lib")}`,
  "-lssl",
  "-lcrypto",
  "-o",
  binary,
]);
execFileSync(
  binary,
  [join(build, "ca.pem"), join(build, "server.pem"), anchors],
  { stdio: "inherit" },
);
