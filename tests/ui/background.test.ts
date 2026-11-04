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
