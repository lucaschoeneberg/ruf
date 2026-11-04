import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
process.chdir(root);
const compose = ["compose", "-f", "tests/interop/asterisk/compose.yml"];
const run = (exe, args, options = {}) =>
  execFileSync(exe, args, { stdio: "inherit", timeout: 180_000, ...options });
mkdirSync("native/build", { recursive: true });
// A new file per process prevents a previous good recording from hiding a failure.
const recording = join(root, "native/build", `echo-${process.pid}.wav`);
rmSync(recording, { force: true });
run("docker", [...compose, "up", "-d", "--wait", "--wait-timeout", "60"]);
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
const wav = readFileSync(recording);
assert.equal(wav.toString("ascii", 0, 4), "RIFF");
assert.equal(wav.toString("ascii", 8, 12), "WAVE");
let format, samples;
for (let offset = 12; offset + 8 <= wav.length;) {
  const name = wav.toString("ascii", offset, offset + 4);
  const size = wav.readUInt32LE(offset + 4);
  assert.ok(offset + 8 + size <= wav.length, "Truncated WAV chunk");
  if (name === "fmt ") format = wav.subarray(offset + 8, offset + 8 + size);
  if (name === "data") samples = wav.subarray(offset + 8, offset + 8 + size);
  offset += 8 + size + (size % 2);
}
assert.ok(format?.length >= 16 && samples, "WAV fmt/data chunks required");
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
    "native",
    "--lib",
    "stale_call_actions",
    "--",
    "--ignored",
  ],
  {
    env: { ...process.env, SIP_TEST_PASSWORD: "sip-lab-only-not-a-secret" },
  },
);
console.log(
  `Recording: ${recording}\nLab remains available; stop with bun run lab:down.`,
);
