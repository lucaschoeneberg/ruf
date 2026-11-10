// Reuse the build's manifest/archive validation before executing cached SDK tests.
import { execFileSync } from "node:child_process";
import { resolve, join } from "node:path";
import { createSoftwareBill } from "../apps/desktop/build/software-bill.ts";

const target = process.argv[2];
if (!target) throw new Error("Pass the native SDK's Rust target");
await createSoftwareBill({
  target,
  native: true,
  profile: "release",
  kind: "desktop",
});
const root = resolve(import.meta.dir, "..");
const windows = process.platform === "win32";
const prefix = join(root, "native/install", target);
for (const test of ["test_sip_nat", "test_sip_nat_registration"])
  execFileSync(
    join(prefix, "test", windows ? "retest.exe" : "retest"),
    windows ? [test] : ["-r", test],
    { cwd: prefix, stdio: "inherit" },
  );
console.log(`Restored native SDK validated for ${target}`);
