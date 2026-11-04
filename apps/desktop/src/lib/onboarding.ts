import type { AccountConfig } from "./generated/AppInfo";

export type AccountField = "registrar" | "username" | "password";
export type AccountErrors = Partial<Record<AccountField, string>>;
const INTRODUCTION_KEY = "sip.introduction.v1";

/** Trim address/username paste artifacts; passwords are always passed unchanged. */
export function normalizeAccount(account: AccountConfig): AccountConfig {
  return {
    ...account,
    registrar: account.registrar.trim(),
    username: account.username.trim(),
  };
}

/** Early field feedback; native validation remains authoritative. */
export function validateAccount(account: AccountConfig): AccountErrors {
  const errors: AccountErrors = {};
  const [host = "", port, ...extra] = account.registrar.split(":");
  const validHost =
    host.length > 0 &&
    host.length <= 253 &&
    host
      .split(".")
      .every(
        (label) =>
          label.length > 0 &&
          label.length <= 63 &&
          !label.startsWith("-") &&
          !label.endsWith("-") &&
          /^[a-z0-9-]+$/i.test(label),
      );
  if (
    !validHost ||
    account.registrar.length > 255 ||
    extra.length > 0 ||
    (port !== undefined &&
      (!/^[0-9]+$/.test(port) || Number(port) < 1 || Number(port) > 65535))
  ) {
    errors.registrar =
      "Gib die Serveradresse an, optional mit :Port. Ohne sip:, Leerzeichen oder Pfad.";
  }
  if (!/^[a-z0-9_.+-]{1,80}$/i.test(account.username)) {
    errors.username =
      "Nutze den SIP-Benutzernamen aus deinen Zugangsdaten (max. 80 Zeichen), keine E-Mail-Adresse.";
  }
  if (
    !account.password ||
    new TextEncoder().encode(account.password).length > 1024 ||
    account.password.includes("\0")
  ) {
    errors.password =
      "Gib dein SIP-Passwort ein (max. 1024 Bytes, ohne Nullzeichen).";
  }
  return errors;
}

/** Store only a non-sensitive introduction flag; blocked storage is non-fatal. */
export function hasSeenIntroduction(
  storage: Pick<Storage, "getItem">,
): boolean {
  try {
    return storage.getItem(INTRODUCTION_KEY) === "seen";
  } catch {
    return false;
  }
}
export function rememberIntroduction(storage: Pick<Storage, "setItem">): void {
  try {
    storage.setItem(INTRODUCTION_KEY, "seen");
  } catch {
    /* Session dismissal still applies. */
  }
}
