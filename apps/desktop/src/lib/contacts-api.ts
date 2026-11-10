import { desktopInvoke } from "./api";
import type { ContactsPermission, ContactsSnapshot } from "./generated/AppInfo";

function desktop<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return desktopInvoke<T>(
    command,
    args,
    "Kontakte sind nur in der macOS-App verfügbar.",
  );
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
export const openContactsApp = (id: string) =>
  desktop<void>("open_contacts_app", { id });

/** Copy only after an explicit click; native fallback supports the macOS WebView. */
export async function copyPhoneNumber(number: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(number);
  } catch {
    await desktop<void>("copy_phone_number", { number });
  }
}
