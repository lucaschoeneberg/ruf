import { message, type Message } from "./i18n";
import type { AccountConfig } from "./generated/AppInfo";

export type AccountField =
  | "registrar"
  | "username"
  | "password"
  | "transport"
  | "mediaSecurity"
  | "authUsername"
  | "outboundProxy";
export type AccountErrors = Partial<Record<AccountField, Message>>;
const INTRODUCTION_KEY = "sip.introduction.v1";

/** Trim address/username paste artifacts; passwords are always passed unchanged. */
export function normalizeAccount(account: AccountConfig): AccountConfig {
  return {
    ...account,
    registrar: account.registrar.trim(),
    username: account.username.trim(),
    authUsername: account.authUsername?.trim() || null,
    outboundProxy: account.outboundProxy?.trim() || null,
  };
}

/** Early field feedback; native validation remains authoritative. */
export function validateAccount(
  account: AccountConfig,
  keepSavedPassword = false,
): AccountErrors {
  const errors: AccountErrors = {};
  if (!validServer(account.registrar))
    errors.registrar = message("validation.registrar");
  if (account.outboundProxy != null && !validServer(account.outboundProxy))
    errors.outboundProxy = message("validation.outboundProxy");
  if (
    account.authUsername != null &&
    !/^[a-z0-9_.+-]{1,80}$/i.test(account.authUsername)
  )
    errors.authUsername = message("validation.authUsername");
  if (account.mediaSecurity === "required" && account.transport !== "tls")
    errors.mediaSecurity = message("validation.mediaSecurity");
  if (!/^[a-z0-9_.+-]{1,80}$/i.test(account.username)) {
    errors.username = message("validation.username");
  }
  if (
    (!account.password && !keepSavedPassword) ||
    new TextEncoder().encode(account.password).length > 1024 ||
    account.password.includes("\0")
  ) {
    errors.password = message("validation.password");
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

/** Shared restricted DNS/IPv4 host[:port] syntax for registrar and outbound proxy. */
function validServer(value: string): boolean {
  const [host = "", port, ...extra] = value.split(":");
  return (
    value.length <= 255 &&
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
      ) &&
    extra.length === 0 &&
    (port === undefined ||
      (/^[0-9]+$/.test(port) && Number(port) >= 1 && Number(port) <= 65535))
  );
}
