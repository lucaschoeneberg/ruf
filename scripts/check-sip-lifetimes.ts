import { resolve } from "node:path";
// Native lifetime regressions with protected freed allocations, no OS audio.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

assert.equal(process.platform, "darwin", "Requires macOS GuardMalloc");
const root = resolve(import.meta.dir, "..");
const build = spawnSync(
  "cargo",
  [
    "test",
    "--locked",
    "-p",
    "baresip-adapter",
    "--features",
    "native",
    "--lib",
    "--no-run",
    "--message-format=json",
  ],
  {
    cwd: root,
    encoding: "utf8",
    timeout: 120_000,
    maxBuffer: 16 * 1024 * 1024,
  },
);
if (build.error) throw build.error;
assert.equal(build.status, 0, build.stderr);
const executable = build.stdout
  .split("\n")
  .filter(Boolean)
  .map((line) => JSON.parse(line))
  .find(
    (item) =>
      item.reason === "compiler-artifact" &&
      item.target.name === "baresip_adapter" &&
      item.profile.test &&
      item.executable,
  )?.executable;
assert.ok(executable, "Native adapter test executable was not produced");

const cases = [
  "late_refer_refusal_after_original_bye_releases_owner",
  "retired_transfer_failure_releases_peer_for_retry",
  "stopping_retired_transfer_does_not_wait_for_its_deadline",
];
for (const name of cases) {
  const result = spawnSync(
    executable,
    [
      "--exact",
      `sip_event_tests::reliability::scenarios::${name}`,
      "--test-threads=1",
      "--nocapture",
    ],
    {
      cwd: root,
      encoding: "utf8",
      timeout: 60_000,
      env: {
        ...process.env,
        DYLD_INSERT_LIBRARIES: "/usr/lib/libgmalloc.dylib",
      },
    },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0, `${name}: ${result.signal}\n${result.stderr}`);
  assert.match(result.stderr, /GuardMalloc\[/, "GuardMalloc did not load");
  assert.match(
    result.stdout,
    /1 passed; 0 failed/,
    "Exact test was not executed",
  );
}
console.log(
  `SIP_LIFETIME_GUARD_OK: ${cases.length} native lifetime regressions`,
);
