import { expect, test } from "bun:test";
import { execFileSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  assertNoQuarantineAttributes,
  cleanAppStoreBundle,
  validateProvisioningProfile,
} from "../../scripts/prepare-app-store.ts";

const profile = () => ({
  Platform: ["OSX"],
  ExpirationDate: "2027-01-01T00:00:00Z",
  TeamIdentifier: ["TESTTEAM01"],
  Entitlements: {
    "com.apple.application-identifier": "TESTTEAM01.org.rufapp.desktop",
    "com.apple.developer.team-identifier": "TESTTEAM01",
    "get-task-allow": false,
  },
  DeveloperCertificates: ["A".repeat(40)],
});
const now = new Date("2026-11-06T00:00:00Z");
const validate = (value: Parameters<typeof validateProvisioningProfile>[0]) =>
  validateProvisioningProfile(value, "org.rufapp.desktop", "A".repeat(40), now);

test("store signing accepts only the exact app and included certificate", () => {
  expect(validate(profile())).toBe("TESTTEAM01");
  const other = profile();
  other.Entitlements["com.apple.application-identifier"] =
    "TESTTEAM01.other.app";
  expect(() => validate(other)).toThrow("Bundle ID");
  const certificate = profile();
  certificate.DeveloperCertificates = ["B".repeat(40)];
  expect(() => validate(certificate)).toThrow("certificate");
});

test("store signing rejects development, expired and non-macOS profiles", () => {
  const development = profile();
  development.Entitlements["get-task-allow"] = true;
  expect(() => validate(development)).toThrow("distribution");
  expect(() =>
    validate({ ...profile(), ProvisionedDevices: ["test-device"] }),
  ).toThrow("distribution");
  expect(() => validate({ ...profile(), ProvisionsAllDevices: true })).toThrow(
    "distribution",
  );
  expect(() =>
    validate({ ...profile(), ExpirationDate: "2026-01-01T00:00:00Z" }),
  ).toThrow("expired");
  expect(() => validate({ ...profile(), Platform: ["iOS"] })).toThrow("macOS");
});

test("store signing rejects inconsistent team entitlement", () => {
  const wrong = profile();
  wrong.Entitlements["com.apple.developer.team-identifier"] = "OTHERTEAM1";
  expect(() => validate(wrong)).toThrow("team");
});

test.skipIf(process.platform !== "darwin")(
  "store packaging removes downloaded-profile quarantine without following outside symlinks",
  () => {
    const fixture = mkdtempSync(join(tmpdir(), "ruf-store-quarantine-"));
    try {
      const app = join(fixture, "Ruf.app");
      const contents = join(app, "Contents");
      mkdirSync(contents, { recursive: true });
      const profilePath = join(contents, "embedded.provisionprofile");
      const outside = join(fixture, "outside-download");
      writeFileSync(profilePath, "synthetic signed profile bytes");
      writeFileSync(outside, "outside the bundle");
      symlinkSync(outside, join(contents, "outside-link"));
      for (const path of [app, contents, profilePath, outside])
        execFileSync("/usr/bin/xattr", [
          "-w",
          "com.apple.quarantine",
          "0083;65000000;Ruf packaging test;",
          path,
        ]);
      expect(() => assertNoQuarantineAttributes(app)).toThrow("quarantine");
      cleanAppStoreBundle(app);
      expect(() => assertNoQuarantineAttributes(app)).not.toThrow();
      expect(
        execFileSync("/usr/bin/xattr", ["-lrs", app], { encoding: "utf8" }),
      ).not.toContain("com.apple.quarantine");
      expect(readFileSync(profilePath, "utf8")).toBe(
        "synthetic signed profile bytes",
      );
      expect(
        execFileSync(
          "/usr/bin/xattr",
          ["-p", "com.apple.quarantine", outside],
          { encoding: "utf8" },
        ),
      ).toContain("Ruf packaging test");
    } finally {
      rmSync(fixture, { recursive: true, force: true });
    }
  },
);
