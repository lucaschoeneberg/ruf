/** A user-facing version heading with short, plain-text changes. */
export interface ChangelogEntry {
  title: string;
  changes: string[];
}

/** Read the changelog's limited Markdown format without rendering remote HTML. */
export function parseChangelog(markdown: string): ChangelogEntry[] {
  if (markdown.length > 100_000) throw new Error("Changelog is too large.");
  const entries: ChangelogEntry[] = [];
  for (const line of markdown.split(/\r?\n/)) {
    if (line.startsWith("## ")) {
      const title = line.slice(3).trim();
      if (title) entries.push({ title, changes: [] });
    } else if (line.startsWith("- ")) {
      const change = line.slice(2).trim();
      if (change) entries.at(-1)?.changes.push(change);
    }
  }
  if (!entries.length || entries.some((entry) => !entry.changes.length))
    throw new Error("No version notes found.");
  return entries;
}

/** Load a public GitHub raw changelog; never send app credentials or a referrer. */
export async function loadChangelog(
  source: string,
  signal: AbortSignal,
  fetcher: typeof fetch = fetch,
): Promise<ChangelogEntry[]> {
  const url = new URL(source);
  if (
    url.protocol !== "https:" ||
    url.hostname !== "raw.githubusercontent.com" ||
    url.port ||
    url.username ||
    url.password ||
    url.search ||
    url.hash ||
    !url.pathname.endsWith(".md")
  )
    throw new Error("Expected a public GitHub raw Markdown URL.");
  const response = await fetcher(url.href, {
    signal,
    credentials: "omit",
    referrerPolicy: "no-referrer",
    cache: "no-cache",
  });
  if (!response.ok) throw new Error("Changelog could not be loaded.");
  return parseChangelog(await response.text());
}
