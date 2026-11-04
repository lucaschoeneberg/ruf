import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";

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

test("incoming caller name and number are shown as plain text", () => {
  const { body } = render(CallPanel, {
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
  const { body } = render(CallPanel, {
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
  ["earlyMedia", "Frühe Medienverbindung"],
  ["held", "Gespräch gehalten"],
] as const)
  test(`${state} remains visible with guarded hangup and escaped identity`, () => {
    const { body } = render(CallPanel, {
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
    expect(body).toContain("Empfangener Tastenton: #");
    if (state === "held") expect(body).toContain("02:34");
    else expect(body).not.toContain("02:34");
  });
