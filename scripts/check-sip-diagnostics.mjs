// Requires the existing isolated Asterisk lab, never production credentials.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";

function run(enabled) {
  const env = {
    ...process.env,
    SIP_TEST_PASSWORD: "sip-lab-only-not-a-secret",
  };
  delete env.SIP_DIAGNOSTICS;
  if (enabled) env.SIP_DIAGNOSTICS = "1";
  const result = spawnSync(
    "cargo",
    [
      "test",
      "--locked",
      "-p",
      "baresip-adapter",
      "--features",
      "native",
      "nat_remote_hangup_without_server_contact_rewriting",
      "--",
      "--ignored",
    ],
    {
      encoding: "utf8",
      timeout: 60_000,
      env,
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  if (result.error) throw result.error;
  assert.equal(result.status, 0, "Synthetic lab remote-hangup test failed");
  return result.stdout + result.stderr;
}

// Native diagnostics go to stderr; capture both streams without printing packets.
const output = run(true);
for (const forbidden of [
  "Authorization:",
  "Contact:",
  "From:",
  "sip-lab-only-not-a-secret",
]) {
  assert.ok(
    !output.includes(forbidden),
    "Diagnostics must not dump SIP headers or credentials",
  );
}
assert.ok(
  output.includes("sip.diag rx=BYE"),
  "Missing sanitized remote BYE diagnostic",
);
assert.ok(
  output.includes("sip.diag event=closed"),
  "Missing native close diagnostic",
);
assert.match(
  output,
  /sip\.diag packet=tx status=200 method=BYE /,
  "Remote BYE must receive 200 OK through the translated signaling path",
);
assert.match(
  output,
  /sip\.diag packet=rx status=200 method=INVITE /,
  "Response status and method are needed to distinguish call setup from registration",
);
assert.match(
  output,
  /sip\.diag packet=tx status=0 method=INVITE .*contact=(?:loopback|private) contact-match=(?:yes|no)/,
  "INVITE must report its numeric local or learned mapped Contact",
);
assert.match(
  output,
  /sip\.diag packet=rx status=200 method=INVITE .*contact=loopback contact-match=yes/,
  "Lab PBX contact must match its published signaling source",
);
assert.match(
  output,
  /sip\.diag register-via received=present rport=valued ms=[0-9]+ call=[0-9]+/,
  "Successful registration must expose only Via parameter presence for NAT diagnostics",
);
assert.match(
  output,
  /sip\.diag register-contacts count=one mapped-target=present ms=[0-9]+ call=[0-9]+/,
  "Registration diagnostics must show whether the returned contacts include the mapped endpoint",
);
const lines = output
  .split(/\r?\n/)
  .filter((line) => line.startsWith("sip.diag "));
for (const line of lines) {
  assert.match(
    line,
    /^sip\.diag (?:[rt]x=(?:BYE|CANCEL|ACK|INVITE|response)|event=(?:active|closed|audio-error)|register-via received=(?:present|missing) rport=(?:valued|empty|missing)|register-contacts count=(?:none|one|many) mapped-target=(?:present|missing|unknown)|packet=(?:rx|tx) status=(?:0|[1-6][0-9]{2}) method=(?:INVITE|ACK|BYE|CANCEL|REGISTER|OPTIONS|UPDATE|PRACK|other) local=(?:tunnel|other|unknown) peer=(?:loopback|private|shared|other-ipv4|ipv6|unknown) contact=(?:none|loopback|private|shared|other-ipv4|ipv6|unknown) contact-match=(?:yes|no|unknown)) ms=[0-9]+ call=[0-9]+$/,
  );
}
assert.ok(!run(false).includes("sip.diag "), "Diagnostics must be opt-in");
console.log(
  `SIP_DIAGNOSTICS_OK: ${lines.length} allowlisted markers, disabled by default`,
);
