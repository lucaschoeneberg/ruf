import assert from "node:assert/strict";
import {
  execFileSync,
  type ExecFileSyncOptionsWithStringEncoding,
} from "node:child_process";
import { chmodSync, mkdirSync, rmSync } from "node:fs";
import { resolve, join } from "node:path";

const root = resolve(import.meta.dir, "..");
process.chdir(root);
const compose = ["compose", "-f", "tests/interop/asterisk/compose.yml"];
const run = (
  exe: string,
  args: string[],
  options: Partial<ExecFileSyncOptionsWithStringEncoding> = {},
) =>
  execFileSync(exe, args, {
    encoding: "utf8",
    stdio: "inherit",
    timeout: 600_000,
    ...options,
  });
mkdirSync("native/build", { recursive: true });
// Fresh exclusive synthetic roots; never modify platform trust or reuse expired keys.
const certificates = join(root, "native/build/asterisk-certs");
rmSync(certificates, { recursive: true, force: true });
run("python3", ["tests/native/tls_peer.py", certificates, "certificates"]);
// The isolated container drops to its asterisk UID; only synthetic material is readable.
chmodSync(certificates, 0o755);
for (const file of ["server.pem", "server.key"])
  chmodSync(join(certificates, file), 0o644);
process.env.SIP_LAB_CERTS = certificates;
process.env.SIP_TEST_CERT_DIRECTORY = certificates;
// A new file per process prevents a previous good recording from hiding a failure.
const recording = join(root, "native/build", `echo-${process.pid}.wav`);
rmSync(recording, { force: true });
run("docker", [
  ...compose,
  "up",
  "-d",
  "--force-recreate",
  "--wait",
  "--wait-timeout",
  "60",
]);
// Keep explicit reloads for the bind-mounted fixtures; TLS transports were recreated.
run("docker", [
  ...compose,
  "exec",
  "-T",
  "asterisk",
  "asterisk",
  "-rx",
  "dialplan reload",
]);
run("docker", [
  ...compose,
  "exec",
  "-T",
  "asterisk",
  "asterisk",
  "-rx",
  "pjsip reload",
]);
const readiness = (command: string) =>
  run(
    "docker",
    [...compose, "exec", "-T", "asterisk", "asterisk", "-rx", command],
    { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );
assert.match(
  readiness("module show like res_srtp.so"),
  /res_srtp\.so.*Running/,
);
assert.match(readiness("pjsip show transports"), /transport-tls\s+tls/);
run(
  "cargo",
  [
    "run",
    "--locked",
    "-p",
    "baresip-adapter",
    "--features",
    "native",
    "--example",
    "asterisk_smoke",
  ],
  {
    env: {
      ...process.env,
      SIP_TEST_PASSWORD: "sip-lab-only-not-a-secret",
      SIP_TEST_RECORDING: recording,
      SIP_TEST_REGISTRAR: "127.0.0.1:5070",
      SIP_TEST_USERNAME: "1000",
      SIP_TEST_EXTENSION: "600",
    },
  },
);
const wav = Buffer.from(await Bun.file(recording).arrayBuffer());
assert.equal(wav.toString("ascii", 0, 4), "RIFF");
assert.equal(wav.toString("ascii", 8, 12), "WAVE");
let format: Buffer | undefined, samples: Buffer | undefined;
for (let offset = 12; offset + 8 <= wav.length;) {
  const name = wav.toString("ascii", offset, offset + 4);
  const size = wav.readUInt32LE(offset + 4);
  assert.ok(offset + 8 + size <= wav.length, "Truncated WAV chunk");
  if (name === "fmt ") format = wav.subarray(offset + 8, offset + 8 + size);
  if (name === "data") samples = wav.subarray(offset + 8, offset + 8 + size);
  offset += 8 + size + (size % 2);
}
assert.ok(
  format && format.length >= 16 && samples,
  "WAV fmt/data chunks required",
);
assert.equal(format.readUInt16LE(0), 1, "PCM audio required");
assert.equal(format.readUInt16LE(2), 1, "Mono audio required");
assert.equal(format.readUInt16LE(14), 16, "16 bit audio required");
const rate = format.readUInt32LE(4);
assert.equal(rate, 8000, "G.711 test expects 8 kHz");
const duration = samples.length / 2 / rate;
assert.ok(duration >= 3, `Recording too short: ${duration}s`);
// Discard startup/jitter-buffer fill, then verify returned tone rather than 200 OK.
const skip = rate;
let squares = 0,
  crossings = 0,
  previous = 0;
const count = samples.length / 2 - skip;
for (let index = skip; index < samples.length / 2; index++) {
  const sample = samples.readInt16LE(index * 2);
  squares += sample * sample;
  if (previous < 0 && sample >= 0) crossings++;
  previous = sample;
}
const rms = Math.sqrt(squares / count);
const frequency = (crossings * rate) / count;
assert.ok(rms > 500, `No useful returned audio: RMS ${rms}`);
assert.ok(
  frequency > 420 && frequency < 460,
  `Expected 440 Hz echo, received ${frequency} Hz`,
);
console.log(
  `AUDIO_OK duration=${duration.toFixed(2)}s RMS=${rms.toFixed(0)} frequency=${frequency.toFixed(1)}Hz`,
);
run(
  "cargo",
  [
    "test",
    "--locked",
    "-p",
    "baresip-adapter",
    "--features",
    "native-test-hooks",
    "--lib",
    "--",
    "--ignored",
    "--test-threads=1",
    "--nocapture",
  ],
  {
    env: { ...process.env, SIP_TEST_PASSWORD: "sip-lab-only-not-a-secret" },
  },
);
console.log(
  `Recording: ${recording}\nLab remains available; stop with bun run lab:down.`,
);

// --ignored excludes controlled TLS security/lifecycle cases: run them explicitly too.
run("cargo", [
  "test",
  "--locked",
  "-p",
  "baresip-adapter",
  "--features",
  "native-test-hooks",
  "--lib",
  "security_tests::",
  "--",
  "--test-threads=1",
]);

// Desktop hosts also cover saved-account restoration. The headless Linux lab
// intentionally needs no Tauri/WebKit system dependencies.
if (process.platform === "darwin" || process.platform === "win32") {
  run("cargo", [
    "test",
    "--locked",
    "-p",
    "sip-desktop",
    "--features",
    "native",
    "--lib",
    "phone::credential_recovery_tests::saved_account_accepts_first_incoming_call_after_each_restart",
    "--",
    "--exact",
    "--ignored",
    "--test-threads=1",
  ]);
} else {
  console.log("Desktop startup regression skipped on the headless Linux host.");
}
