import { expect, setDefaultTimeout, test } from "bun:test";
import { existsSync } from "node:fs";
import {
  createSoftwareBill,
  addNative,
  validateSoftwareBill,
} from "../../apps/desktop/build/software-bill";

// Cargo metadata may download another platform's dependencies on a cold cache.
setDefaultTimeout(120_000);

const portable = {
  target: "aarch64-apple-darwin",
  native: false,
  profile: "release",
  kind: "desktop",
} as const;

test("portable SBOM records the target and Cargo relationships without native libraries", async () => {
  const bill = await createSoftwareBill(portable);
  expect(bill.metadata.properties).toContainEqual({
    name: "ruf:build:target",
    value: portable.target,
  });
  expect(bill.metadata.properties).toContainEqual({
    name: "ruf:build:native",
    value: "false",
  });
  expect(
    bill.components.some((item) =>
      ["baresip", "re", "openssl"].includes(item.name),
    ),
  ).toBe(false);
  expect(
    bill.components.some(
      (item) => item.name === "windows-native-keyring-store",
    ),
  ).toBe(false);
  const tauri = bill.components.find((item) => item.name === "tauri")!;
  const root = bill.dependencies.find(
    (item) => item.ref === bill.metadata.component["bom-ref"],
  )!;
  expect(root.dependsOn).toContain(tauri["bom-ref"]);
  const ids = new Set([
    bill.metadata.component["bom-ref"],
    ...bill.components.map((item) => item["bom-ref"]),
  ]);
  for (const dependency of bill.dependencies) {
    expect(ids.has(dependency.ref)).toBe(true);
    for (const id of dependency.dependsOn) expect(ids.has(id)).toBe(true);
  }
  expect(JSON.stringify(bill)).not.toContain(process.cwd());
});

// Portable CI has no native SDK; desktop macOS CI builds it before UI tests.
test.skipIf(!existsSync("native/install/aarch64-apple-darwin/sdk.json"))(
  "native SBOM includes the SDK OpenSSL version and native linkage edges",
  async () => {
    const bill = await createSoftwareBill({ ...portable, native: true });
    const sdk = await Bun.file(
      "native/install/aarch64-apple-darwin/sdk.json",
    ).json();
    const ssl = bill.components.find((item) => item.name === "openssl")!;
    const baresip = bill.components.find((item) => item.name === "baresip")!;
    const adapter = bill.components.find(
      (item) => item.name === "baresip-adapter",
    )!;
    expect(ssl.version).toBe(sdk.openssl.version);
    expect(baresip.properties).toContainEqual({
      name: "ruf:native:modules",
      value: sdk.modules.join(","),
    });
    expect(
      bill.components
        .find((item) => item.name === "re")
        ?.properties?.some((item) => item.name === "ruf:native:modules"),
    ).toBe(false);
    expect(
      bill.dependencies.find((item) => item.ref === adapter["bom-ref"])
        ?.dependsOn,
    ).toContain(ssl["bom-ref"]);
    expect(
      bill.dependencies.find((item) => item.ref === baresip["bom-ref"])
        ?.dependsOn,
    ).toContain(ssl["bom-ref"]);
    expect(
      baresip.properties?.some((item) => item.name === "ruf:patch:sha256"),
    ).toBe(true);
  },
);

test("native SDK identity and linkage validation also run without an installed SDK", async () => {
  const bill = await createSoftwareBill(portable);
  const context = { ...portable, native: true };
  const sdk = {
    target: portable.target,
    sources: await Bun.file("native/sources.json").json(),
    openssl: { version: "3.5.9" },
    patches: {
      reSipNat: "NAT fixture",
      baresipReferPolicy: "REFER fixture",
      reCallControls: "re controls fixture",
      baresipCallControls: "baresip controls fixture",
      reSecurity: "re security fixture",
      baresipSecurity: "baresip security fixture",
      reReliability: "re reliability fixture",
      baresipReliability: "baresip reliability fixture",
    },
    modules: ["g711", "coreaudio"],
  };
  expect(() =>
    addNative(bill, context, { ...sdk, target: "wrong-target" }),
  ).toThrow();
  expect(() =>
    addNative(bill, context, { ...sdk, openssl: { version: "" } }),
  ).toThrow();
  const changedControls = structuredClone(bill);
  addNative(changedControls, context, {
    ...sdk,
    patches: {
      ...sdk.patches,
      baresipCallControls: "changed controls fixture",
    },
  });
  addNative(bill, context, sdk);
  const patchHash = (source: typeof bill, name: string) =>
    source.components
      .find((item) => item.name === name)
      ?.properties?.find((property) => property.name === "ruf:patch:sha256")
      ?.value;
  expect(patchHash(changedControls, "baresip")).not.toBe(
    patchHash(bill, "baresip"),
  );
  expect(patchHash(changedControls, "re")).toBe(patchHash(bill, "re"));
  validateSoftwareBill(bill);
  expect(bill.components.find((item) => item.name === "openssl")?.version).toBe(
    "3.5.9",
  );
  const adapter = bill.components.find(
    (item) => item.name === "baresip-adapter",
  )!;
  expect(
    bill.dependencies.find((item) => item.ref === adapter["bom-ref"])
      ?.dependsOn,
  ).toContain("pkg:generic/openssl@3.5.9");
});

test("native output fails closed when no matching SDK exists", async () => {
  await expect(
    createSoftwareBill({
      ...portable,
      target: "x86_64-unknown-freebsd",
      native: true,
    }),
  ).rejects.toThrow();
});

test("graph validation rejects dangling and disconnected components", async () => {
  const bill = await createSoftwareBill(portable);
  bill.dependencies.push({
    ref: bill.metadata.component["bom-ref"],
    dependsOn: ["missing"],
  });
  expect(() => validateSoftwareBill(bill)).toThrow("Dangling");
  const disconnected = await createSoftwareBill(portable);
  disconnected.components.push({
    type: "library",
    "bom-ref": "orphan",
    name: "orphan",
    version: "1",
  });
  expect(() => validateSoftwareBill(disconnected)).toThrow("Disconnected");
});

test("Windows selection includes Windows credential storage and excludes Apple's store", async () => {
  const bill = await createSoftwareBill({
    ...portable,
    target: "x86_64-pc-windows-msvc",
  });
  expect(
    bill.components.some(
      (item) => item.name === "windows-native-keyring-store",
    ),
  ).toBe(true);
  expect(
    bill.components.some((item) => item.name === "apple-native-keyring-store"),
  ).toBe(false);
});
