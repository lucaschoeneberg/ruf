import { expect, test } from "bun:test";
import {
  parseChangelog,
  loadChangelog,
} from "../../apps/desktop/src/lib/changelog";

function mockFetch(
  handler: (...args: Parameters<typeof fetch>) => ReturnType<typeof fetch>,
): typeof fetch {
  return Object.assign(handler, { preconnect: fetch.preconnect });
}

test("consumer changelog keeps version headings and user-facing changes", () => {
  expect(
    parseChangelog(
      "# Änderungen\n\n## 0.1.0 · Vorschau\n\n- Kontakte direkt anrufen.\n- Klingelton separat einstellen.\n",
    ),
  ).toEqual([
    {
      title: "0.1.0 · Vorschau",
      changes: ["Kontakte direkt anrufen.", "Klingelton separat einstellen."],
    },
  ]);
  expect(() => parseChangelog("<!doctype html><h1>404</h1>")).toThrow();
});

test("remote changelog accepts only public GitHub raw URLs and omits credentials", async () => {
  let requested = "";
  let options: RequestInit | undefined;
  const fetcher = mockFetch(async (input, init) => {
    requested = String(input);
    options = init;
    return new Response(
      "# Änderungen\n\n## 0.2.0\n\n- Kontakte schneller finden.\n",
    );
  });
  await expect(
    loadChangelog(
      "https://example.com/CHANGELOG.md",
      new AbortController().signal,
      fetcher,
    ),
  ).rejects.toThrow();
  expect(requested).toBe("");
  const entries = await loadChangelog(
    "https://raw.githubusercontent.com/example/ruf/main/CHANGELOG.md",
    new AbortController().signal,
    fetcher,
  );
  expect(entries[0]?.title).toBe("0.2.0");
  expect(options?.credentials).toBe("omit");
  expect(options?.referrerPolicy).toBe("no-referrer");
});

test("failed and malformed GitHub responses do not become release notes", async () => {
  const signal = new AbortController().signal;
  const url = "https://raw.githubusercontent.com/example/ruf/main/CHANGELOG.md";
  await expect(
    loadChangelog(
      url,
      signal,
      mockFetch(async () => new Response("Not found", { status: 404 })),
    ),
  ).rejects.toThrow();
  await expect(
    loadChangelog(
      url,
      signal,
      mockFetch(async () => new Response("<html>Sign in</html>")),
    ),
  ).rejects.toThrow();
});
