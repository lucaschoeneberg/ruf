import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";
import { locale } from "../../apps/desktop/src/lib/i18n";

Bun.plugin({
  name: "localized-settings-svelte",
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
const { default: GeneralSettings } =
  await import("../../apps/desktop/src/lib/components/GeneralSettings.svelte");

test("language draft keeps its selection while labels follow the committed language", () => {
  const props = {
    autostart: false,
    startHidden: true,
    language: "de" as const,
    available: true,
    disabled: false,
    dirty: true,
    onsave() {},
    ondiscard() {},
  };
  locale.set("en");
  const english = render(GeneralSettings, { props }).body;
  expect(english).toContain("Language");
  expect(english).toContain('value="de" selected');
  expect(english).toContain("Save general settings");
  expect(english).toContain("Discard");
  locale.set("de");
  const german = render(GeneralSettings, { props }).body;
  expect(german).toContain("Sprache");
  expect(german).toContain('value="de" selected');
  expect(german).toContain("Allgemein speichern");
});

const { default: PermissionStatus } =
  await import("../../apps/desktop/src/lib/components/PermissionStatus.svelte");
const { message } = await import("../../apps/desktop/src/lib/i18n");

test("stored permission errors translate again after a language change", () => {
  const error = message("settings.contacts_check_failed");
  const props = {
    title: "Contacts",
    status: null,
    error,
    oncheck() {},
  };
  locale.set("en");
  expect(render(PermissionStatus, { props }).body).toContain(
    "Contacts permission could not be checked. Try again.",
  );
  locale.set("de");
  expect(render(PermissionStatus, { props }).body).toContain(
    "Die Kontakte-Freigabe konnte nicht geprüft werden. Versuche es erneut.",
  );
});

test("both offline release-note bundles retain every release and change", async () => {
  const { parseChangelog } =
    await import("../../apps/desktop/src/lib/changelog");
  const english = parseChangelog(
    await Bun.file(new URL("../../CHANGELOG.md", import.meta.url)).text(),
  );
  const german = parseChangelog(
    await Bun.file(new URL("../../CHANGELOG.de.md", import.meta.url)).text(),
  );
  expect(german.map((entry) => entry.title.match(/^\S+/)?.[0])).toEqual(
    english.map((entry) => entry.title.match(/^\S+/)?.[0]),
  );
  expect(german.map((entry) => entry.changes.length)).toEqual(
    english.map((entry) => entry.changes.length),
  );
  const manifest: { version: string } = await Bun.file(
    new URL("../../package.json", import.meta.url),
  ).json();
  expect(german[0]?.title).toBe(`${manifest.version} · Vorschau`);
  expect(english[0]?.title).toBe(`${manifest.version} · Preview`);
  expect(
    german.find((entry) => entry.title === "0.1.0 · Vorschau")?.changes[0],
  ).toContain("Durchsuche Ruf");
});
