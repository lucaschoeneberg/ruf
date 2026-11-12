import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

// A real child-process fixture exercises scheduling/logging without SIP or Cargo.
const names = [
  "other::workspace_test",
  ...Array.from({ length: 7 }, (_, i) => `nat_tests::case_${i}`),
  "sip_event_tests::ordinary",
  "sip_event_tests::incoming_answer_without_ack_reports_failure",
  "sip_event_tests::controls_hold_deadline",
  "sip_event_tests::reliability::long_local_hold_resume_gets_full_media_grace_interval",
];

async function fixture(
  fail: boolean,
  writeFailure = false,
  inventory: readonly string[] = names,
) {
  const root = mkdtempSync(join(tmpdir(), "ruf-native-runner-"));
  const events = join(root, "events.jsonl");
  const cargo = join(root, "cargo.ts");
  const wrapper = join(root, "wrapper.ts");
  writeFileSync(
    cargo,
    `import { appendFileSync } from "node:fs";
const args = process.argv.slice(2);
const names = ${JSON.stringify(inventory)};
const events = ${JSON.stringify(events)};
const event = (kind, name) => appendFileSync(events, JSON.stringify({kind, name, pid: process.pid}) + "\\n");
if (args.includes("--no-run")) { event("build", "once"); process.exit(0); }
if (args.includes("--list")) { console.log(names.map(n => n + ": test").join("\\n")); process.exit(0); }
const separator = args.indexOf("--");
const before = args.slice(0, separator);
const after = args.slice(separator + 1);
const filter = before.at(-1) === "--lib" || !before.includes("--lib") ? "" : before.at(-1);
const skips = after.flatMap((arg, i) => arg === "--skip" ? [after[i + 1]] : []);
const selected = names.filter(n => (after.includes("--exact") ? n === filter : n.includes(filter)) && !skips.some(s => n.includes(s)));
event("start", filter);
process.stdout.write("live-before-exit");
process.stderr.write("stderr-before-exit");
await Bun.sleep(${writeFailure ? 1000 : 200});
for (const name of selected) event("test", name);
event("end", filter);
console.log("finished");
if (${fail} && selected.includes("nat_tests::case_2")) process.exit(7);
`,
  );
  writeFileSync(
    wrapper,
    `${
      writeFailure
        ? `import { mock } from "bun:test";
const fs = await import("node:fs");
mock.module("node:fs", () => ({...fs, writeSync() { throw new Error("fixture log write failed"); }}));`
        : ""
    }
const { checkNativeWorkspace } = await import(${JSON.stringify(resolve("scripts/check-native-workspace.ts"))});
const ok = await checkNativeWorkspace(["--adapter"], [process.execPath, ${JSON.stringify(cargo)}]);
${
  writeFailure
    ? `const running = fs.readFileSync(${JSON.stringify(events)}, "utf8").trim().split("\\n")
  .map(line => JSON.parse(line)).filter(record => record.kind === "start")
  .filter(record => { try { process.kill(record.pid, 0); return true; } catch { return false; } });
console.log("CHILDREN_RUNNING_AT_RETURN=" + running.length);`
    : ""
}
if (!ok) process.exitCode = 1;
`,
  );
  const child = Bun.spawn([process.execPath, wrapper], {
    stdout: "pipe",
    stderr: "pipe",
  });
  return { root, events, child };
}

test("native runner executes every discovered test once with bounded parallelism and live output", async () => {
  const { root, events, child } = await fixture(false);
  try {
    let output = "";
    let liveBeforeCompletion = false;
    const decoder = new TextDecoder();
    for await (const chunk of child.stdout) {
      output += decoder.decode(chunk, { stream: true });
      if (output.includes("live-before-exit")) {
        const lines = readFileSync(events, "utf8").trim().split("\n");
        const seen = lines.map((line) => JSON.parse(line) as { kind: string });
        if (
          seen.filter((x) => x.kind === "start").length >
          seen.filter((x) => x.kind === "end").length
        )
          liveBeforeCompletion = true;
      }
    }
    const stderr = await new Response(child.stderr).text();
    expect(await child.exited, stderr).toBe(0);
    const records = readFileSync(events, "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line) as { kind: string; name: string });
    expect(records.filter((x) => x.kind === "build")).toHaveLength(1);
    expect(
      records
        .filter((x) => x.kind === "test")
        .map((x) => x.name)
        .sort(),
    ).toEqual([...names].sort());
    let running = 0,
      maximum = 0;
    for (const record of records) {
      if (record.kind === "start") maximum = Math.max(maximum, ++running);
      if (record.kind === "end") --running;
    }
    expect(maximum).toBeGreaterThan(3);
    expect(maximum).toBeLessThanOrEqual(6);
    expect(running).toBe(0);
    expect(liveBeforeCompletion).toBe(true);
    expect(output).toContain("stderr-before-exit");
    expect(output).toContain("[native:");
    expect(output).toContain("NATIVE_CHECKS_OK");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("native runner awaits terminated children after an output failure", async () => {
  const { root, events, child } = await fixture(false, true);
  let logs: string | undefined;
  let pids: number[] = [];
  const alive = (pid: number) => {
    try {
      process.kill(pid, 0);
      return true;
    } catch {
      return false;
    }
  };
  try {
    const [output, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    logs = /Native checks failed; logs: (.+)/.exec(stderr)?.[1]?.trim();
    pids = readFileSync(events, "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line) as { kind: string; pid: number })
      .filter((record) => record.kind === "start")
      .map((record) => record.pid);
    expect(code).toBe(1);
    expect(stderr).toContain("fixture log write failed");
    expect(output).not.toContain("NATIVE_CHECKS_OK");
    expect(output).toContain("CHILDREN_RUNNING_AT_RETURN=0");
    expect(pids.length).toBeGreaterThan(0);
    expect(pids.filter(alive)).toEqual([]);
    expect(logs).toBeDefined();
  } finally {
    for (const pid of pids) if (alive(pid)) process.kill(pid);
    if (logs) rmSync(logs, { recursive: true, force: true });
    rmSync(root, { recursive: true, force: true });
  }
});

test("native runner waits for all groups and retains logs after one failure", async () => {
  const { root, events, child } = await fixture(true);
  let logs: string | undefined;
  try {
    const [output, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    expect(code).toBe(1);
    expect(output).not.toContain("NATIVE_CHECKS_OK");
    logs = /Native checks failed; logs: (.+)/.exec(stderr)?.[1]?.trim();
    expect(logs).toBeDefined();
    const records = readFileSync(events, "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line) as { kind: string; name: string });
    expect(
      records
        .filter((x) => x.kind === "test")
        .map((x) => x.name)
        .sort(),
    ).toEqual([...names].sort());
    expect(readFileSync(join(logs!, "nat-3.log"), "utf8")).toContain(
      "live-before-exit",
    );
  } finally {
    if (logs) rmSync(logs, { recursive: true, force: true });
    rmSync(root, { recursive: true, force: true });
  }
});

test("native runner rejects discovery without native modules", async () => {
  const { root, events, child } = await fixture(false, false, ["other::only"]);
  try {
    const [output, stderr, code] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    expect(code).toBe(1);
    expect(stderr).toContain("requires both NAT and SIP unit tests");
    expect(output).not.toContain("NATIVE_CHECKS_OK");
    expect(readFileSync(events, "utf8")).not.toContain('"start"');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
