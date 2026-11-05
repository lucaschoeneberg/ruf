import { expect, test } from "bun:test";
import { compile } from "svelte/compiler";
import { render } from "svelte/server";

Bun.plugin({
  name: "settings-redesign-svelte",
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
const { settingsSections, searchSettings } =
  await import("../../apps/desktop/src/lib/settings-navigation");
const { default: PermissionStatus } =
  await import("../../apps/desktop/src/lib/components/PermissionStatus.svelte");

test("eight settings areas retain old route ids", () => {
  expect(settingsSections.map((s) => s.id)).toEqual([
    "general",
    "accounts",
    "audio",
    "notifications",
    "contacts",
    "privacy",
    "diagnostics",
    "about",
  ]);
});
test("field search identifies the microphone control without indexing secrets", () => {
  const results = searchSettings("Mikrofon");
  expect(
    results.some((r) => r.section === "audio" && r.target === "inputDevice"),
  ).toBe(true);
  expect(searchSettings("unfindable-password")).toEqual([]);
});
function permission(
  status: "authorized" | "denied" | "notDetermined" | "unsupported" | null,
  error: string | null = null,
) {
  return render(PermissionStatus, {
    props: {
      title: "Systemfreigabe",
      status,
      error,
      checking: false,
      requestLabel: "Mitteilungen erlauben",
      oncheck() {},
      onrequest() {},
      onopen() {},
    },
  }).body;
}
test("denied notification permission has a system recovery action and no second prompt", () => {
  const body = permission("denied");
  expect(body).toContain("Im System blockiert");
  expect(body).toContain("Systemeinstellungen öffnen");
  expect(body).not.toContain("Mitteilungen erlauben");
});
test("undecided permission offers only explicit authorization", () => {
  expect(permission("notDetermined")).toContain("Mitteilungen erlauben");
});
test("query failures remain unknown instead of claiming authorization or denial", () => {
  const body = permission(null, "Freigabe konnte nicht geprüft werden.");
  expect(body).toContain("konnte nicht geprüft");
  expect(body).toContain("Freigabe erneut prüfen");
  expect(body).not.toContain("Im System blockiert");
});
test("unsupported permission does not offer native authorization", () => {
  const body = permission("unsupported");
  expect(body).toContain("Nicht verfügbar");
  expect(body).not.toContain("Mitteilungen erlauben");
  expect(body).not.toContain("Systemeinstellungen öffnen");
});

for (const [query, section, target] of [
  ["Anruferdetails", "notifications", "notification-caller"],
  ["Klingellautstärke", "audio", "ringtone-volume"],
  ["Firma anzeigen", "contacts", "show-organization"],
  ["Aufbewahrung", "contacts", "history-retention"],
  ["Passwort", "accounts", "password"],
  ["Bei der Anmeldung im Hintergrund starten", "general", "start-hidden"],
]) {
  test(`visible setting ${query} finds its own control`, () => {
    expect(
      searchSettings(query).some(
        (result) => result.section === section && result.target === target,
      ),
    ).toBe(true);
  });
}
