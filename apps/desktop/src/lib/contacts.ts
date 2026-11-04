import type { Contact } from "./generated/AppInfo";

function searchable(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/ß/g, "ss");
}

function phoneSearch(value: string): string | null {
  if (!/^[0-9+*#()\s./-]+$/.test(value)) return null;
  const digits = value.replace(/[^0-9+*#]/g, "");
  return digits || null;
}

/** Filter source contacts without merging namesakes or changing the original order. */
export function searchContacts(
  contacts: readonly Contact[],
  query: string,
  favoritesOnly = false,
): Contact[] {
  const text = searchable(query.trim());
  const telephone = phoneSearch(text);
  const terms = text.split(/\s+/).filter(Boolean);
  return contacts
    .filter((contact) => {
      if (favoritesOnly && !contact.favorite) return false;
      const fields = searchable(`${contact.name} ${contact.organization}`);
      const numbers = contact.phones.map(
        (phone) => phoneSearch(phone.number) ?? searchable(phone.number),
      );
      if (telephone)
        return (
          fields.includes(text) ||
          numbers.some((number) => number.includes(telephone))
        );
      return terms.every(
        (term) =>
          fields.includes(term) ||
          numbers.some((number) => number.includes(phoneSearch(term) ?? term)),
      );
    })
    .sort(
      (a, b) =>
        Number(b.favorite) - Number(a.favorite) ||
        a.name.localeCompare(b.name, "de", { sensitivity: "base" }) ||
        a.id.localeCompare(b.id),
    );
}
