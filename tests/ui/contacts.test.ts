import { describe, expect, test } from "bun:test";
import type { Contact } from "../../apps/desktop/src/lib/generated/AppInfo";
import { searchContacts } from "../../apps/desktop/src/lib/contacts";

const contacts: Contact[] = [
  {
    id: "c",
    source: "macos",
    name: "Zoë Weiß",
    organization: "Beispiel Büro",
    favorite: false,
    phones: [
      {
        label: "Arbeit",
        number: "+49 (30) 123-456",
        destination: "+4930123456",
      },
    ],
  },
  {
    id: "b",
    source: "macos",
    name: "Anna Test",
    organization: "",
    favorite: false,
    phones: [],
  },
  {
    id: "a",
    source: "macos",
    name: "Anna Test",
    organization: "Andere Firma",
    favorite: true,
    phones: [{ label: "Intern", number: "456", destination: "456" }],
  },
];

describe("contact phonebook search", () => {
  test("searches Unicode names and companies without dropping namesakes", () => {
    expect(searchContacts(contacts, "zoe weiss").map((c) => c.id)).toEqual([
      "c",
    ]);
    expect(searchContacts(contacts, "beispiel buro").map((c) => c.id)).toEqual([
      "c",
    ]);
    expect(searchContacts(contacts, "Anna").map((c) => c.id)).toEqual([
      "a",
      "b",
    ]);
    expect(searchContacts(contacts, "nicht vorhanden")).toEqual([]);
  });
  test("finds numbers regardless of their display separators", () => {
    expect(searchContacts(contacts, "+49 30 123456").map((c) => c.id)).toEqual([
      "c",
    ]);
    expect(searchContacts(contacts, "123-456").map((c) => c.id)).toEqual(["c"]);
    expect(searchContacts(contacts, "456").map((c) => c.id)).toEqual([
      "a",
      "c",
    ]);
  });
  test("numeric searches still match names and companies", () => {
    const company = { ...contacts[1], organization: "Studio 54" };
    expect(searchContacts([company], "54").map((c) => c.id)).toEqual(["b"]);
  });
  test("sorts favorites first without mutating source order and filters explicitly", () => {
    expect(searchContacts(contacts, "").map((c) => c.id)).toEqual([
      "a",
      "b",
      "c",
    ]);
    expect(searchContacts(contacts, "", true).map((c) => c.id)).toEqual(["a"]);
    expect(searchContacts(contacts, "Zoë", true)).toEqual([]);
    expect(contacts.map((c) => c.id)).toEqual(["c", "b", "a"]);
  });
});

test("contact display preferences can use alphabetical order without changing favorites or company search", () => {
  const rows = contacts.map((contact) => ({
    ...contact,
    favorite: contact.id === "c",
  }));
  expect(searchContacts(rows, "").map((contact) => contact.id)).toEqual([
    "c",
    "a",
    "b",
  ]);
  expect(
    searchContacts(rows, "", false, false).map((contact) => contact.id),
  ).toEqual(["a", "b", "c"]);
  expect(
    searchContacts(rows, "", true, false).map((contact) => contact.id),
  ).toEqual(["c"]);
  expect(
    searchContacts(rows, "Beispiel Büro", false, false).map(
      (contact) => contact.id,
    ),
  ).toEqual(["c"]);
  expect(rows[0].favorite).toBe(true);
});
