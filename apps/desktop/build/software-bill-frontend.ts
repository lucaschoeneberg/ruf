import type {
  SoftwareBill,
  SoftwareComponent,
} from "../src/lib/software-bill.js";
import { connect } from "./software-bill.js";
import { root, runtime, sha256 } from "./sbom-runtime.js";

interface NpmPackage {
  name: string;
  version: string;
  license?: string;
}

/** Map rendered Vite modules to their installed package identities and import edges. */
export async function addFrontend(
  bill: SoftwareBill,
  moduleIds: string[],
  imports: (id: string) => readonly string[],
): Promise<void> {
  const rootRef = bill.metadata.component["bom-ref"];
  const cache = new Map<string, Promise<SoftwareComponent | null>>();
  const owner = (id: string): Promise<SoftwareComponent | null> => {
    const path = id.replace(/\\/g, "/").replace(/^\0/, "").split("?")[0]!;
    const index = path.lastIndexOf("/node_modules/");
    if (index < 0) return Promise.resolve(null);
    const parts = path.slice(index + "/node_modules/".length).split("/");
    const name = parts[0]?.startsWith("@")
      ? parts.slice(0, 2).join("/")
      : parts[0];
    if (!name || name.startsWith(".")) return Promise.resolve(null);
    const directory = `${path.slice(0, index)}/node_modules/${name}`;
    if (!cache.has(directory))
      cache.set(
        directory,
        (async () => {
          const pkg = JSON.parse(
            await runtime.file(`${directory}/package.json`).text(),
          ) as NpmPackage;
          const slash = pkg.name.indexOf("/");
          const group = pkg.name.startsWith("@")
            ? pkg.name.slice(0, slash)
            : undefined;
          const purl = `pkg:npm/${pkg.name.replace("@", "%40")}@${pkg.version}`;
          return {
            type: "library",
            "bom-ref": purl,
            name: group ? pkg.name.slice(slash + 1) : pkg.name,
            ...(group ? { group } : {}),
            version: pkg.version,
            purl,
            scope: "required",
            ...(typeof pkg.license === "string"
              ? { licenses: [{ expression: pkg.license }] }
              : {}),
          };
        })(),
      );
    return cache.get(directory)!;
  };
  const owners = new Map<string, SoftwareComponent | null>();
  for (const id of moduleIds) owners.set(id, await owner(id));
  const selected = new Map<string, SoftwareComponent>();
  for (const component of owners.values())
    if (component) selected.set(component["bom-ref"], component);
  bill.components.push(...selected.values());
  for (const [id, component] of owners) {
    const from = component?.["bom-ref"] ?? rootRef;
    const visited = new Set<string>();
    const visit = async (target: string): Promise<void> => {
      if (visited.has(target)) return;
      visited.add(target);
      const targetOwner = await owner(target);
      const ref = targetOwner?.["bom-ref"] ?? rootRef;
      if (ref !== from && (ref === rootRef || selected.has(ref))) {
        connect(bill, from, ref);
        return;
      }
      for (const child of imports(target)) await visit(child);
    };
    for (const target of imports(id)) await visit(target);
  }
  // Dev-server previews have no rendered graph; identify their two frontend entry packages explicitly.
  if (moduleIds.every((id) => imports(id).length === 0))
    for (const ref of selected.keys()) connect(bill, rootRef, ref);
  for (const name of ["IBMPlexSans-Regular.ttf", "IBMPlexSans-SemiBold.ttf"]) {
    const hash = sha256(
      await runtime
        .file(new URL(`apps/desktop/public/fonts/${name}`, root))
        .arrayBuffer(),
    );
    const ref = `asset:fonts/${name}`;
    bill.components.push({
      type: "file",
      "bom-ref": ref,
      name,
      version: hash,
      scope: "required",
      hashes: [{ alg: "SHA-256", content: hash }],
      licenses: [{ expression: "OFL-1.1" }],
    });
    connect(bill, rootRef, ref);
  }
}
