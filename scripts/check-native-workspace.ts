// Isolate native singletons by process; retain the complete Cargo suite.
import { execFileSync } from "node:child_process";
import { closeSync, mkdtempSync, openSync, rmSync, writeSync } from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const MAX_PROCESSES = 6;
const SIP_TIMEOUT =
  /timeout|deadline|without_ack|refused_compensation|bounded_failure|long_local_hold/;
type Group = { name: string; filter: string[] };

function groupsFor(tests: string[]): Group[] {
  const nat = tests.filter((test) => test.startsWith("nat_tests::"));
  const sip = tests.filter((test) => test.startsWith("sip_event_tests::"));
  if (!nat.length || !sip.length)
    throw new Error(
      "Native test discovery requires both NAT and SIP unit tests",
    );
  const timed = sip.filter((test) => SIP_TIMEOUT.test(test));
  const isolated = [
    ...nat.map((test, i) => ({ name: `nat-${i + 1}`, test })),
    ...timed.map((test, i) => ({ name: `sip-timeout-${i + 1}`, test })),
  ];
  const exact = ({ name, test }: (typeof isolated)[number]): Group => ({
    name,
    filter: ["--lib", test, "--", "--exact", "--test-threads=1"],
  });
  // Begin the longest known waits first; no protocol timer or assertion changes.
  const long = ({ test }: (typeof isolated)[number]) =>
    /blackout|long_local_hold/.test(test);
  return [
    ...isolated.filter(long).map(exact),
    {
      name: "remaining",
      filter: ["--", "--skip", "nat_tests::", "--skip", "sip_event_tests::"],
    },
    {
      name: "sip",
      filter: [
        "--lib",
        "sip_event_tests::",
        "--",
        "--test-threads=1",
        ...timed.flatMap((test) => ["--skip", test]),
      ],
    },
    ...isolated.filter((group) => !long(group)).map(exact),
  ];
}

/** Run the CLI checks; the command argument permits isolated process fixtures. */
export async function checkNativeWorkspace(
  args: string[],
  command: readonly [string, ...string[]] = ["cargo"],
): Promise<boolean> {
  if (args.some((arg) => arg !== "--adapter"))
    throw new Error("Only --adapter is supported (headless adapter checks).");
  const root = resolve(import.meta.dir, "..");
  const common = [
    "test",
    "--locked",
    ...(args.includes("--adapter")
      ? ["-p", "baresip-adapter", "--features", "native"]
      : ["--workspace", "--features", "sip-desktop/native"]),
  ];
  const cargo = (extra: string[]) => [...command.slice(1), ...common, ...extra];
  execFileSync(command[0], cargo(["--no-run"]), {
    cwd: root,
    stdio: "inherit",
  });
  const listing = execFileSync(
    command[0],
    cargo(["--lib", "--", "--list", "--format=terse"]),
    { cwd: root, encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] },
  );
  const tests = [
    ...new Set(
      listing
        .split(/\r?\n/)
        .filter((line) => line.endsWith(": test"))
        .map((line) => line.slice(0, -6)),
    ),
  ];
  const groups = groupsFor(tests);
  const directory = mkdtempSync(join(tmpdir(), "ruf-native-tests-"));
  const run = async ({ name, filter }: Group): Promise<boolean> => {
    let log: number | undefined;
    let passed = false;
    const started = Date.now();
    console.log(`[native:${name}] START ${filter.join(" ")}`);
    const stream = async (source: ReadableStream<Uint8Array>) => {
      const decoder = new TextDecoder();
      const display = (text: string) => {
        // Flush partial lines too: libtest prints the current test before waiting.
        for (const line of text.split(/\r?\n/))
          if (line) process.stdout.write(`[native:${name}] ${line}\n`);
      };
      for await (const chunk of source) {
        if (log !== undefined) writeSync(log, chunk);
        display(decoder.decode(chunk, { stream: true }));
      }
      display(decoder.decode());
    };
    try {
      log = openSync(join(directory, `${name}.log`), "w");
      const child = Bun.spawn([...command, ...common, ...filter], {
        cwd: root,
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
      });
      const stopOnError = async (output: ReadableStream<Uint8Array>) => {
        try {
          await stream(output);
        } catch (error) {
          // Keep this worker's slot until the child and both pipes have closed.
          child.kill();
          throw error;
        }
      };
      const [exit, stdout, stderr] = await Promise.allSettled([
        child.exited,
        stopOnError(child.stdout),
        stopOnError(child.stderr),
      ]);
      for (const result of [exit, stdout, stderr])
        if (result.status === "rejected") throw result.reason;
      const code = exit.status === "fulfilled" ? exit.value : 1;
      console.log(
        `[native:${name}] ${code === 0 ? "PASS" : "FAIL"} (${((Date.now() - started) / 1000).toFixed(1)}s, exit ${code})`,
      );
      passed = code === 0;
    } catch (error) {
      console.error(`[native:${name}] FAIL`, error);
    } finally {
      if (log !== undefined) {
        try {
          closeSync(log);
        } catch (error) {
          console.error(`[native:${name}] FAIL closing log`, error);
          passed = false;
        }
      }
    }
    return passed;
  };
  let next = 0;
  let success = true;
  await Promise.all(
    Array.from({ length: Math.min(MAX_PROCESSES, groups.length) }, async () => {
      while (next < groups.length) {
        const group = groups[next++];
        if (group && !(await run(group))) success = false;
      }
    }),
  );
  if (!success) {
    console.error(`Native checks failed; logs: ${directory}`);
  } else {
    rmSync(directory, { recursive: true, force: true });
    console.log("NATIVE_CHECKS_OK: all selected Cargo partitions passed");
  }
  return success;
}

if (import.meta.main && !(await checkNativeWorkspace(process.argv.slice(2))))
  process.exitCode = 1;
