import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ContactsPermission, ContactsSnapshot } from "./generated/AppInfo";

function desktop<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!isTauri())
    return Promise.reject(
      new Error("Kontakte sind nur in der macOS-App verfügbar."),
    );
  return invoke<T>(command, args);
}

export const getContacts = () => desktop<ContactsSnapshot>("get_contacts");
export const requestContactsAccess = () =>
  desktop<ContactsSnapshot>("request_contacts_access");
export const getContactsPermission = () =>
  desktop<ContactsPermission>("get_contacts_permission");
export const requestContactsPermission = () =>
  desktop<ContactsPermission>("request_contacts_permission");
export const openContactsSettings = () =>
  desktop<void>("open_contacts_settings");
export const setContactFavorite = (id: string, favorite: boolean) =>
  desktop<void>("set_contact_favorite", { id, favorite });

export const refreshContacts = () =>
  desktop<ContactsSnapshot>("refresh_contacts");
export const openContactsApp = () => desktop<void>("open_contacts_app");

/** Copy only after an explicit click; native fallback supports the macOS WebView. */
export async function copyPhoneNumber(number: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(number);
  } catch {
    await desktop<void>("copy_phone_number", { number });
  }
}
