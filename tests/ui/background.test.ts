import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";
import type {
  CallSnapshot,
  CallState,
  ErrorCode,
} from "../../apps/desktop/src/lib/generated/AppInfo";

Bun.plugin({
  name: "svelte-server-components",
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

function renderCall({
  props,
}: {
  props: {
    state: CallState;
    peer: string;
    number?: string | null;
    elapsed: string;
    busy: boolean;
    compact: boolean;
    receivedDtmf?: string;
    onanswer: () => void;
    onhangup: () => void;
  };
}) {
  const call: CallSnapshot = {
    id: "test-call",
    accountId: 1,
    state: props.state,
    destination: null,
    caller: { name: props.peer, number: props.number ?? null },
    connectedAtMs: 1000,
    muted: false,
    localHeld: false,
    remoteHeld: props.state === "held",
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
  };
  return render(CallPanel, {
    props: {
      call,
      elapsed: props.elapsed,
      busy: props.busy,
      compact: props.compact,
      receivedDtmf: props.receivedDtmf,
      onanswer: props.onanswer,
      onhangup: props.onhangup,
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
  });
}

test("incoming caller name and number are shown as plain text", () => {
  const { body } = renderCall({
    props: {
      state: "incoming",
      peer: "Ada <Büro>",
      number: "+4930123456",
      elapsed: "00:00",
      busy: false,
      compact: false,
      onanswer() {},
      onhangup() {},
    },
  });
  expect(body).toContain("+4930123456");
  expect(body).toContain("Ada &lt;Büro");
  expect(body).not.toContain("<Büro>");
  expect(body).toContain("Annehmen");
  expect(body).toContain("Ablehnen");
});

test("an answering call remains visible without another answer action", () => {
  const { body } = renderCall({
    props: {
      state: "answering",
      peer: "Anruf für Büro",
      elapsed: "00:00",
      busy: false,
      compact: false,
      onanswer() {},
      onhangup() {},
    },
  });
  expect(body).toContain("Wird angenommen");
  expect(body).not.toContain(">Annehmen</button>");
  expect(body).toContain("Auflegen");
});

for (const [state, label] of [
  ["earlyMedia", "Verbindung wird vorbereitet …"],
  ["held", "Gegenstelle hält das Gespräch"],
] as const)
  test(`${state} remains visible with guarded hangup and escaped identity`, () => {
    const { body } = renderCall({
      props: {
        state,
        peer: '<img src=x onerror="alert(1)">',
        number: "+4930123456",
        elapsed: "02:34",
        busy: false,
        compact: true,
        receivedDtmf: "#",
        onanswer() {},
        onhangup() {},
      },
    });
    expect(body).toContain(label);
    expect(body).toContain("&lt;img");
    expect(body).not.toContain("<img src=x");
    expect(body).toContain("Auflegen");
    expect(body).toContain("Empfangene Wähltaste: #");
    if (state === "held") expect(body).toContain("02:34");
    else expect(body).not.toContain("02:34");
  });

const { default: AccountSetup } =
  await import("../../apps/desktop/src/lib/components/AccountSetup.svelte");

function accountRecovery(code: ErrorCode, disabled = false) {
  return render(AccountSetup, {
    props: {
      account: {
        registrar: "pbx.example.test",
        username: "alice",
        password: "",
        audioMode: "system",
      },
      name: "Büro",
      existing: {
        id: 1,
        name: "Büro",
        registrar: "pbx.example.test",
        username: "alice",
        registration: "failed",
        lastError: { code, message: "Synthetic local error", sipStatus: null },
        removing: false,
      },
      busy: false,
      disabled,
      async onconfigure() {},
      onremove() {},
      onreconnect() {},
    },
  }).body;
}

test("account storage failure does not imply incorrect SIP credentials", () => {
  const body = accountRecovery("storageFailure");
  expect(body).toContain("App-Datenordner");
  expect(body).not.toContain("Prüfe die Zugangsdaten");
  expect(body).not.toContain("Erneut verbinden</button>");
});

for (const code of ["credentialAccessRequired", "credentialFailure"] as const)
  test(`${code} offers an explicit retry using the stored credential`, () => {
    const body = accountRecovery(code);
    expect(body).toContain("System-Schlüsselbund");
    expect(body).toContain("Erneut verbinden</button>");
    expect(body).not.toContain("Prüfe die Zugangsdaten");
  });

test("missing credential requires entering a password rather than retrying access", () => {
  const body = accountRecovery("credentialMissing");
  expect(body).toContain("Gib das Passwort ein");
  expect(body).not.toContain("Erneut verbinden</button>");
  expect(body).not.toContain("Leer lassen");
});

test("credential retry is disabled during a call", () => {
  expect(accountRecovery("credentialAccessRequired", true)).toMatch(
    /<button[^>]*disabled[^>]*>Erneut verbinden<\/button>/,
  );
});
