import { expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  createSoftwareBill,
  validateSoftwareBill,
} from "../../apps/desktop/build/software-bill";
import { addFrontend } from "../../apps/desktop/build/software-bill-frontend";

test("rendered frontend graph records resolved package imports and excludes unused build tools", async () => {
  const directory = await mkdtemp(join(tmpdir(), "ruf-sbom-"));
  try {
    for (const [name, version] of [
      ["@fixture/ui", "1.2.0"],
      ["helper", "2.3.0"],
      ["unused-compiler", "9.0.0"],
    ]) {
      const path = join(directory, "node_modules", name!);
      await mkdir(path, { recursive: true });
      await writeFile(
        join(path, "package.json"),
        JSON.stringify({ name, version, license: "MIT" }),
      );
    }
    const source = join(directory, "src/main.ts");
    const ui = join(directory, "node_modules/@fixture/ui/index.js");
    const helper = join(directory, "node_modules/helper/index.js");
    const graph = new Map([
      [source, [ui]],
      [ui, [helper]],
      [helper, []],
    ]);
    const bill = await createSoftwareBill({
      target: "aarch64-apple-darwin",
      native: false,
      profile: "release",
      kind: "desktop",
    });
    await addFrontend(bill, [...graph.keys()], (id) => graph.get(id) ?? []);
    validateSoftwareBill(bill);
    const uiRef = "pkg:npm/%40fixture/ui@1.2.0";
    expect(
      bill.dependencies.find((item) => item.ref === uiRef)?.dependsOn,
    ).toEqual(["pkg:npm/helper@2.3.0"]);
    expect(
      bill.dependencies.find(
        (item) => item.ref === bill.metadata.component["bom-ref"],
      )?.dependsOn,
    ).toContain(uiRef);
    expect(
      bill.components.some((item) => item.name === "unused-compiler"),
    ).toBe(false);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
