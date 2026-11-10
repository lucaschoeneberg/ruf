import { normalizeDialNumber } from "./dialing";
import type { Contact, ContactPhone } from "./generated/AppInfo";

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

/** Filter and sort contacts without merging namesakes or mutating the source. */
export function searchContacts(
  contacts: readonly Contact[],
  query: string,
  favoritesOnly = false,
  favoritesFirst = true,
  matchGermanNumbers = true,
): Contact[] {
  const text = searchable(query.trim());
  const telephone = phoneSearch(text);
  const terms = text.split(/\s+/).filter(Boolean);
  const queryKey = telephone ? numberPrefix(query, matchGermanNumbers) : null;
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
          numbers.some((number) => number.includes(telephone)) ||
          contact.phones.some((phone) => {
            const key = numberPrefix(phone.number, matchGermanNumbers);
            return !!key && !!queryKey && key.includes(queryKey);
          })
        );
      return terms.every(
        (term) =>
          fields.includes(term) ||
          numbers.some((number) => number.includes(phoneSearch(term) ?? term)),
      );
    })
    .sort(
      (a, b) =>
        (favoritesFirst ? Number(b.favorite) - Number(a.favorite) : 0) ||
        a.name.localeCompare(b.name, "de", { sensitivity: "base" }) ||
        a.id.localeCompare(b.id),
    );
}

/** Regional keys are only for lookup; never use them as a dial destination. */
function numberPrefix(number: string, matchGerman: boolean): string | null {
  const value = normalizeDialNumber(number);
  if (!value) return null;
  if (value.startsWith("00") && /^\d+$/.test(value) && value.length >= 4)
    return `+${value.slice(2)}`;
  if (matchGerman && /^0[1-9][0-9]+$/.test(value))
    return `+49${value.slice(1)}`;
  return value;
}
export interface DialSuggestion {
  contact: Contact;
  phone: ContactPhone;
}
/** Suggestions are choices, not authenticated identities; ambiguous owners remain separate. */
export function dialSuggestions(
  contacts: readonly Contact[],
  number: string,
  matchGerman = true,
): DialSuggestion[] {
  const prefix = numberPrefix(number, matchGerman);
  if (!prefix || number.replace(/\D/g, "").length < 3) return [];
  const candidates = contacts
    .flatMap((contact) =>
      contact.phones.flatMap((phone) => {
        if (!phone.destination) return [];
        const key = numberPrefix(phone.number, matchGerman);
        return key?.startsWith(prefix)
          ? [{ contact, phone, exactMatch: key === prefix }]
          : [];
      }),
    )
    .sort(
      (a, b) =>
        Number(b.exactMatch) - Number(a.exactMatch) ||
        Number(b.contact.favorite) - Number(a.contact.favorite) ||
        a.contact.name.localeCompare(b.contact.name, "de"),
    );
  const suggestions: DialSuggestion[] = [];
  for (const { contact, phone } of candidates) {
    if (
      suggestions.some(
        (other) =>
          other.contact.id === contact.id &&
          other.phone.destination === phone.destination,
      )
    )
      continue;
    suggestions.push({ contact, phone });
    if (suggestions.length === 3) break;
  }
  return suggestions;
}
