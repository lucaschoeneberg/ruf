// Separate native singletons by process while retaining the complete Cargo suite.
import { execFileSync, spawn } from "node:child_process";
import {
  closeSync,
  mkdtempSync,
  openSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
if (args.some((arg) => arg !== "--adapter"))
  throw new Error("Only --adapter is supported (headless adapter checks).");
const root = fileURLToPath(new URL("../", import.meta.url));
const common = [
  "test",
  "--locked",
  ...(args.includes("--adapter")
    ? ["-p", "baresip-adapter", "--features", "native"]
    : ["--workspace", "--features", "sip-desktop/native"]),
];
execFileSync("cargo", [...common, "--no-run"], { cwd: root, stdio: "inherit" });
const directory = mkdtempSync(join(tmpdir(), "ruf-native-tests-"));
const groups = [
  ["remaining", ["--", "--skip", "nat_tests::", "--skip", "sip_event_tests::"]],
  ["nat", ["--lib", "nat_tests::", "--", "--test-threads=1"]],
  ["sip", ["--lib", "sip_event_tests::", "--", "--test-threads=1"]],
];
const results = await Promise.all(
  groups.map(async ([name, filter]) => {
    const path = join(directory, `${name}.log`);
    const log = openSync(path, "w");
    const started = Date.now();
    try {
      const code = await new Promise((resolve, reject) => {
        const child = spawn("cargo", [...common, ...filter], {
          cwd: root,
          stdio: ["ignore", log, log],
        });
        child.once("error", reject);
        child.once("exit", (code) => resolve(code ?? 1));
      });
      return { name, path, code, seconds: (Date.now() - started) / 1000 };
    } catch (error) {
      return { name, path, code: 1, error };
    } finally {
      closeSync(log);
    }
  }),
);
for (const result of results) {
  console.log(
    `::group::Native ${result.name} (${result.seconds?.toFixed(1) ?? "?"}s)`,
  );
  process.stdout.write(readFileSync(result.path));
  if (result.error) console.error(result.error);
  console.log("::endgroup::");
}
if (results.some((result) => result.code !== 0)) {
  console.error(`Native checks failed; logs: ${directory}`);
  process.exitCode = 1;
} else {
  rmSync(directory, { recursive: true, force: true });
  console.log("NATIVE_CHECKS_OK: all selected Cargo partitions passed");
}
