/** Authoring checks for the shared i18next V4/rust-i18n JSON resources. */
export function validateCatalogs(source: unknown, translation: unknown): void {
  const record = (value: unknown): Record<string, unknown> => {
    if (value === null || typeof value !== "object" || Array.isArray(value))
      throw new Error("Catalog must be an object");
    return value as Record<string, unknown>;
  };
  const left = record(source),
    right = record(translation);
  const keys = Object.keys(left).sort();
  if (JSON.stringify(keys) !== JSON.stringify(Object.keys(right).sort()))
    throw new Error("Catalog keys differ");
  const placeholders = (value: unknown): string => {
    if (typeof value !== "string" || !value.trim())
      throw new Error("Message must be nonempty text");
    const pattern = /%\{([a-zA-Z][a-zA-Z0-9_]*)(?:,\s*number)?\}/g;
    if (/[{}]/.test(value.replace(pattern, "")))
      throw new Error("Malformed message placeholder");
    return [...new Set([...value.matchAll(pattern)].map((match) => match[0]))]
      .sort()
      .join("|");
  };
  for (const key of keys) {
    if (placeholders(left[key]) !== placeholders(right[key]))
      throw new Error(`Placeholders differ: ${key}`);
    const plural = /_(one|other)$/.exec(key);
    if (plural) {
      const sibling =
        key.slice(0, -plural[0].length) +
        (plural[1] === "one" ? "_other" : "_one");
      if (!Object.hasOwn(left, sibling))
        throw new Error(`Missing plural form: ${sibling}`);
    }
  }
}
