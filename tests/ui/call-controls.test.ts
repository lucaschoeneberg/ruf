import { formatMessage } from "../../apps/desktop/src/lib/i18n";
import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";
import type {
  CallSnapshot,
  CallSecurity,
} from "../../apps/desktop/src/lib/generated/AppInfo";
import {
  normalizeAccount,
  validateAccount,
} from "../../apps/desktop/src/lib/onboarding";
import { errorMessage as errorReference } from "../../apps/desktop/src/lib/api";

Bun.plugin({
  name: "call-controls-svelte",
  setup(build) {
    build.onLoad({ filter: /\.svelte$/ }, async ({ path }) => ({
      contents: compile(await Bun.file(path).text(), {
        filename: path,
        generate: "server",
      }).js.code,
      loader: "js",
    }));
  },
});
const { default: CallPanel } =
  await import("../../apps/desktop/src/lib/components/CallPanel.svelte");
function call(overrides: Partial<CallSnapshot> = {}): CallSnapshot {
  return {
    contact: null,
    id: "original",
    accountId: 2,
    state: "active",
    destination: "1234",
    caller: null,
    connectedAtMs: 1000,
    muted: false,
    localHeld: false,
    remoteHeld: false,
    holdPending: false,
    transfer: "idle",
    consultation: false,
    events: [],
    eventsDropped: 0,
    lastError: null,
    security: {
      signalingTransport: null,
      tlsPeerVerified: false,
      media: "unknown",
    },
    ...overrides,
  };
}
function panel(current = call(), calls = [current]) {
  return render(CallPanel, {
    props: {
      call: current,
      calls,
      elapsed: "02:00",
      busy: false,
      compact: false,
      onanswer() {},
      onhangup() {},
      async onmute() {
        return true;
      },
      onhold() {},
      ontransfer() {},
      onconsult() {},
      onswitch() {},
      oncancelconsult() {},
      oncompletetransfer() {},
    },
  }).body;
}
test("confirmed local and remote hold remain separate", () => {
  const body = panel(
    call({ localHeld: true, remoteHeld: true, state: "held" }),
  );
  expect(body).toContain("Von dir gehalten");
  expect(body).toContain("Gegenstelle hält");
  expect(body).toContain("Fortsetzen");
});
test("pending renegotiation has honest status and retains mute/hangup", () => {
  const body = panel(call({ holdPending: true }));
  expect(body).toContain("Gesprächsänderung wird abgestimmt");
  expect(body).toMatch(
    /aria-pressed="false"[^>]*>[^<]*Stummschalten|Stummschalten/,
  );
  expect(body).toContain("Auflegen");
  expect(body).toMatch(/disabled[^>]*>Halten/);
});
test("mute only changes after native confirmation", () => {
  expect(panel(call({ muted: true }))).toContain("Mikrofon einschalten");
  expect(panel(call({ muted: false }))).toContain("Stummschalten");
});
test("two conversations show both destinations and explicit completion actions", () => {
  const original = call({ localHeld: true });
  const child = call({ id: "child", consultation: true, destination: "5678" });
  const body = panel(original, [original, child]);
  expect(body).toContain("5678");
  expect(body).toContain("wechseln");
  expect(body).toContain("Rückfrage abbrechen");
  expect(body).toContain("Vermittlung abschließen");
});
test("dialing consultation permits return but cannot complete transfer", () => {
  const original = call({ localHeld: true });
  const child = call({
    id: "child",
    consultation: true,
    destination: "5678",
    state: "ringing",
  });
  const body = panel(child, [original, child]);
  expect(body).toContain("Rückfrage abbrechen");
  expect(body).toMatch(/disabled[^>]*>Vermittlung abschließen/);
});
for (const [security, text] of [
  [
    { signalingTransport: "tls", tlsPeerVerified: true, media: "sdesSrtp" },
    "TLS geprüft · SRTP",
  ],
  [
    { signalingTransport: "tls", tlsPeerVerified: true, media: "unprotected" },
    "TLS geprüft · Audio unverschlüsselt",
  ],
  [
    { signalingTransport: "tls", tlsPeerVerified: false, media: "sdesSrtp" },
    "TLS nicht bestätigt",
  ],
  [
    { signalingTransport: "tls", tlsPeerVerified: true, media: "unknown" },
    "TLS geprüft · Audioschutz unbekannt",
  ],
  [
    { signalingTransport: "udp", tlsPeerVerified: false, media: "unprotected" },
    "UDP · Audio unverschlüsselt",
  ],
  [
    { signalingTransport: "tcp", tlsPeerVerified: false, media: "unknown" },
    "TCP · Audioschutz unbekannt",
  ],
  [
    { signalingTransport: null, tlsPeerVerified: false, media: "unknown" },
    "Sicherheit noch nicht bestätigt",
  ],
] satisfies [CallSecurity, string][])
  test(`native security observation: ${text}`, () => {
    expect(panel(call({ security }))).toContain(text);
  });
test("account security validates TLS requirement and optional structured fields", () => {
  const account = {
    registrar: "pbx.test",
    username: "1000",
    password: " secret ",
    audioMode: "system" as const,
    transport: "udp" as const,
    mediaSecurity: "required" as const,
    authUsername: null,
    outboundProxy: null,
  };
  expect(validateAccount(account)).toHaveProperty("mediaSecurity");
  const normalized = normalizeAccount({
    ...account,
    transport: "tls",
    authUsername: " auth ",
    outboundProxy: "  ",
  });
  expect(normalized.authUsername).toBe("auth");
  expect(normalized.outboundProxy).toBeNull();
  expect(normalized.password).toBe(" secret ");
  expect(
    validateAccount({ ...normalized, outboundProxy: "sip:host;transport=udp" }),
  ).toHaveProperty("outboundProxy");
});
for (const [code, text] of [
  ["certificateRejected", "Zertifikat"],
  ["transportFailure", "Transport"],
  ["mediaSecurityRequired", "SRTP"],
] as const)
  test(`${code} has actionable safe explanation`, () => {
    const message = errorMessage({ code, message: "remote secret" });
    expect(message).toContain(text);
    expect(message).not.toContain("remote secret");
  });

test("native transfer lifecycle reports failure and confirmed success", () => {
  expect(panel(call({ transfer: "failed" }))).toContain(
    "Vermittlung fehlgeschlagen",
  );
  expect(panel(call({ transfer: "succeeded" }))).toContain(
    "Vermittlung bestätigt",
  );
});

test("single dialog names its owner independently of outgoing selection", () => {
  const current = call();
  const body = render(CallPanel, {
    props: {
      call: current,
      accounts: [
        {
          id: 2,
          name: "Büro",
          registrar: "pbx.test",
          username: "1000",
          enabled: true,
          enabledChanging: false,
          registeredAtMs: null,
          registration: "registered",
          lastError: null,
          removing: false,
          transport: "tls",
          mediaSecurity: "required",
          authUsername: null,
          outboundProxy: null,
        },
      ],
      elapsed: "00:01",
      busy: false,
      compact: false,
      onanswer() {},
      onhangup() {},
      async onmute() {
        return true;
      },
      onhold() {},
      ontransfer() {},
      onconsult() {},
      onswitch() {},
      oncancelconsult() {},
      oncompletetransfer() {},
    },
  }).body;
  expect(body.replace(/<[^>]*>/g, "").replace(/\s+/g, " ")).toContain(
    "Gespräch läuft · Büro",
  );
});

test("attended transfer error is visible while consultation is selected", () => {
  const original = call({
    localHeld: true,
    transfer: "failed",
    lastError: { code: "sipBusy", message: "Transfer failed", sipStatus: 486 },
  });
  const child = call({ id: "child", destination: "5678", consultation: true });
  const body = panel(child, [original, child]);
  expect(body).toContain("Vermittlung fehlgeschlagen");
});

function errorMessage(error: unknown): string {
  return formatMessage("de", errorReference(error));
}
