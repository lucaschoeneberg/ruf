import { formatMessage } from "../../apps/desktop/src/lib/i18n";
import { expect, test } from "bun:test";
import { errorMessage as errorReference } from "../../apps/desktop/src/lib/api";
import type { ErrorCode } from "../../apps/desktop/src/lib/generated/AppInfo";

const cases: [ErrorCode, RegExp][] = [
  ["sipRedirect", /umgeleitet/i],
  ["sipAuthenticationFailed", /Zugangsdaten/],
  ["sipForbidden", /nicht erlaubt/],
  ["sipNotFound", /nicht gefunden/],
  ["sipTimeout", /rechtzeitig/],
  ["sipUnavailable", /nicht erreichbar/],
  ["sipBusy", /besetzt/],
  ["sipNotAcceptable", /Audioformate/],
  ["sipRejected", /abgelehnt/],
  ["sipFailure", /fehlgeschlagen/],
];
for (const [code, description] of cases)
  test(`${code} shows a useful German message without remote reason text`, () => {
    const message = errorMessage({
      code,
      message: "<script>remote secret sip:user:password@pbx.test</script>",
      sipStatus: 486,
    });
    expect(message).toMatch(description);
    expect(message).not.toContain("script");
    expect(message).not.toContain("password");
    expect(message).not.toContain("pbx.test");
  });

function errorMessage(error: unknown): string {
  return formatMessage("de", errorReference(error));
}
