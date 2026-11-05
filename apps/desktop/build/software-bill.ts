import type { Plugin } from "vite";
import type {
  SoftwareBill,
  SoftwareBuild,
  SoftwareComponent,
} from "../src/lib/software-bill.js";
import { command, read, root, runtime, sha256 } from "./sbom-runtime.js";
import { addFrontend } from "./software-bill-frontend.js";

interface CargoPackage {
  id: string;
  name: string;
  version: string;
  source: string | null;
  license: string | null;
  targets: { kind: string[] }[];
}
interface CargoNode {
  id: string;
  deps: { pkg: string; dep_kinds: { kind: "build" | "dev" | null }[] }[];
}
interface CargoMetadata {
  packages: CargoPackage[];
  resolve: { nodes: CargoNode[] };
}
export interface NativeSdk {
  target: string;
  sources: Record<string, { version: string; commit: string }>;
  patches: { reSipNat: string; baresipReferPolicy: string };
  modules: string[];
  openssl: { version: string };
}

/** Select preview context explicitly; desktop wrappers provide an authoritative selection. */
export function softwareBuild(): SoftwareBuild {
  const encoded = runtime.env.RUF_SBOM_CONTEXT;
  if (encoded) {
    const context = JSON.parse(encoded) as SoftwareBuild;
    if (
      !context.target ||
      typeof context.native !== "boolean" ||
      !["debug", "release"].includes(context.profile) ||
      context.kind !== "desktop"
    )
      throw new Error("Invalid desktop SBOM build context.");
    if (
      runtime.env.TAURI_ENV_TARGET_TRIPLE &&
      runtime.env.TAURI_ENV_TARGET_TRIPLE !== context.target
    )
      throw new Error("Tauri and SBOM targets disagree.");
    return context;
  }
  const target = command(["rustc", "-vV"]).match(/^host: (.+)$/m)?.[1];
  if (!target) throw new Error("Could not determine the Rust host target.");
  return { target, native: false, profile: "debug", kind: "preview" };
}

export function connect(bill: SoftwareBill, from: string, to: string): void {
  let node = bill.dependencies.find((item) => item.ref === from);
  if (!node) {
    node = { ref: from, dependsOn: [] };
    bill.dependencies.push(node);
  }
  if (from !== to && !node.dependsOn.includes(to)) node.dependsOn.push(to);
}

/** Generate the selected non-dev Cargo graph; build-only/proc-macro components are excluded from runtime scope. */
export async function createSoftwareBill(
  context = softwareBuild(),
): Promise<SoftwareBill> {
  const manifest = JSON.parse(await read("package.json")) as {
    version: string;
  };
  const metadata = JSON.parse(
    command([
      "cargo",
      "metadata",
      "--locked",
      "--format-version",
      "1",
      "--filter-platform",
      context.target,
      ...(context.native ? ["--features", "sip-desktop/native"] : []),
    ]),
  ) as CargoMetadata;
  const packages = new Map(metadata.packages.map((item) => [item.id, item]));
  const nodes = new Map(metadata.resolve.nodes.map((item) => [item.id, item]));
  const app = metadata.packages.find((item) => item.name === "sip-desktop");
  if (!app) throw new Error("Desktop Cargo package is missing.");
  const ref = (pkg: CargoPackage) => `pkg:cargo/${pkg.name}@${pkg.version}`;
  const rootRef = ref(app);
  const bill: SoftwareBill = {
    bomFormat: "CycloneDX",
    specVersion: "1.6",
    version: 1,
    serialNumber: `urn:uuid:${crypto.randomUUID()}`,
    metadata: {
      timestamp: new Date().toISOString(),
      tools: {
        components: [
          { type: "application", name: "ruf-sbom", version: manifest.version },
        ],
      },
      component: {
        type: "application",
        "bom-ref": rootRef,
        name: "Ruf",
        version: manifest.version,
        publisher: "Mardu GmbH",
      },
      properties: [
        ...Object.entries(context).map(([name, value]) => ({
          name: `ruf:build:${name}`,
          value: String(value),
        })),
        {
          name: "ruf:build:rustc",
          value: command(["rustc", "--version"]).trim(),
        },
        {
          name: "ruf:input:Cargo.lock:sha256",
          value: sha256(await read("Cargo.lock")),
        },
        {
          name: "ruf:input:bun.lock:sha256",
          value: sha256(await read("bun.lock")),
        },
      ],
    },
    components: [],
    dependencies: [{ ref: rootRef, dependsOn: [] }],
  };
  const roles = new Map<string, boolean>();
  const visit = (id: string, runtimeRequired: boolean) => {
    const pkg = packages.get(id);
    if (!pkg) throw new Error("Unresolved Cargo dependency.");
    runtimeRequired &&= !pkg.targets.some((target) =>
      target.kind.includes("proc-macro"),
    );
    if (roles.has(id) && (roles.get(id) || !runtimeRequired)) return;
    roles.set(id, runtimeRequired);
    for (const dep of nodes.get(id)?.deps ?? []) {
      const kinds = dep.dep_kinds.filter((kind) => kind.kind !== "dev");
      if (!kinds.length) continue;
      const child = packages.get(dep.pkg);
      if (!child) throw new Error("Unresolved Cargo dependency.");
      connect(bill, ref(pkg), ref(child));
      visit(
        dep.pkg,
        runtimeRequired && kinds.some((kind) => kind.kind === null),
      );
    }
  };
  visit(app.id, true);
  for (const [id, required] of roles) {
    const pkg = packages.get(id)!;
    if (pkg.id === app.id) continue;
    bill.components.push({
      type: "library",
      "bom-ref": ref(pkg),
      name: pkg.name,
      version: pkg.version,
      ...(pkg.source ? { purl: ref(pkg) } : {}),
      scope: required ? "required" : "excluded",
      ...(pkg.license ? { licenses: [{ expression: pkg.license }] } : {}),
    });
  }
  if (context.native) {
    const sdk = JSON.parse(
      await read(`native/install/${context.target}/sdk.json`),
    ) as NativeSdk;
    const pins = JSON.parse(
      await read("native/sources.json"),
    ) as NativeSdk["sources"];
    const patches = {
      reSipNat: await read("native/patches/re-sip-nat.patch"),
      baresipReferPolicy: await read(
        "native/patches/baresip-refer-policy.patch",
      ),
    };
    if (
      JSON.stringify(sdk.sources) !== JSON.stringify(pins) ||
      JSON.stringify(sdk.patches) !== JSON.stringify(patches)
    )
      throw new Error(
        "Native SDK sources or patches are stale; run bun run native:build.",
      );
    addNative(bill, context, sdk);
    for (const name of ["baresip", "re", "ssl", "crypto"]) {
      const filename = context.target.includes("windows")
        ? `${name}.lib`
        : `lib${name}.a`;
      const content = await runtime
        .file(new URL(`native/install/${context.target}/lib/${filename}`, root))
        .arrayBuffer();
      const component = bill.components.find(
        (item) =>
          item.name ===
          (name === "ssl" || name === "crypto" ? "openssl" : name),
      )!;
      (component.properties ??= []).push({
        name: `ruf:archive:${filename}:sha256`,
        value: sha256(content),
      });
    }
  }
  return bill;
}

/** Add only native libraries whose matching SDK was selected for this build. */
export function addNative(
  bill: SoftwareBill,
  context: SoftwareBuild,
  sdk: NativeSdk,
): void {
  if (
    !context.native ||
    sdk.target !== context.target ||
    !/^\d+\.\d+\.\d+/.test(sdk.openssl?.version ?? "")
  )
    throw new Error(
      "A matching SDK with an exact OpenSSL version is required.",
    );
  const adapter = bill.components.find(
    (item) => item.name === "baresip-adapter",
  );
  if (!adapter)
    throw new Error("Native adapter is missing from the Cargo graph.");
  for (const name of ["baresip", "re"] as const) {
    const source = sdk.sources[name];
    if (!source?.version || !/^[0-9a-f]{40}$/.test(source.commit))
      throw new Error("Invalid SDK source pin.");
    const purl = `pkg:github/baresip/${name}@${source.commit}`;
    bill.components.push({
      type: "library",
      "bom-ref": purl,
      name,
      version: source.version,
      purl,
      scope: "required",
      licenses: [{ expression: "BSD-3-Clause" }],
      properties: [
        {
          name: "ruf:patch:sha256",
          value: sha256(
            name === "re"
              ? sdk.patches.reSipNat
              : sdk.patches.baresipReferPolicy,
          ),
        },
        ...(name === "baresip"
          ? [{ name: "ruf:native:modules", value: sdk.modules.join(",") }]
          : []),
      ],
    });
    connect(bill, adapter["bom-ref"], purl);
  }
  const purl = `pkg:generic/openssl@${sdk.openssl.version}`;
  bill.components.push({
    type: "library",
    "bom-ref": purl,
    name: "openssl",
    version: sdk.openssl.version,
    purl,
    scope: "required",
    licenses: [{ expression: "Apache-2.0" }],
  });
  connect(bill, adapter["bom-ref"], purl);
  const baresip = bill.components.find((item) => item.name === "baresip")!;
  const re = bill.components.find((item) => item.name === "re")!;
  connect(bill, baresip["bom-ref"], re["bom-ref"]);
  connect(bill, baresip["bom-ref"], purl);
  connect(bill, re["bom-ref"], purl);
}

/** Reject dangling edges or components disconnected from the product root. */
export function validateSoftwareBill(bill: SoftwareBill): void {
  const ids = new Set([
    bill.metadata.component["bom-ref"],
    ...bill.components.map((item) => item["bom-ref"]),
  ]);
  if (ids.size !== bill.components.length + 1)
    throw new Error("Duplicate SBOM component identity.");
  for (const id of ids)
    if (!bill.dependencies.some((item) => item.ref === id))
      bill.dependencies.push({ ref: id, dependsOn: [] });
  for (const node of bill.dependencies) {
    if (!ids.has(node.ref) || node.dependsOn.some((id) => !ids.has(id)))
      throw new Error("Dangling SBOM dependency.");
    node.dependsOn.sort();
  }
  const graph = new Map(
    bill.dependencies.map((node) => [node.ref, node.dependsOn]),
  );
  const reachable = new Set<string>();
  const pending = [bill.metadata.component["bom-ref"]];
  while (pending.length) {
    const id = pending.pop()!;
    if (reachable.has(id)) continue;
    reachable.add(id);
    pending.push(...(graph.get(id) ?? []));
  }
  if ([...ids].some((id) => !reachable.has(id)))
    throw new Error("Disconnected SBOM component.");
  bill.components.sort((a, b) =>
    a["bom-ref"].localeCompare(b["bom-ref"], "en"),
  );
  bill.dependencies.sort((a, b) => a.ref.localeCompare(b.ref, "en"));
}

/** Bundle a build-specific SBOM; dev previews are explicitly identified as previews. */
export async function softwareBillPlugin(): Promise<Plugin> {
  const context = softwareBuild();
  const initial = await createSoftwareBill(context);
  const desktop = runtime.file(new URL("apps/desktop/", root)).name;
  const previewIds = [
    runtime.resolveSync("svelte", desktop),
    runtime.resolveSync("@tauri-apps/api/core", desktop),
  ];
  await addFrontend(initial, previewIds, () => []);
  initial.metadata.properties.push({
    name: "ruf:frontend:resolution",
    value: "development-preview",
  });
  validateSoftwareBill(initial);
  const moduleId = "virtual:software-bill";
  return {
    name: "ruf-software-bill",
    resolveId(id) {
      if (id === moduleId) return `\0${moduleId}`;
    },
    load(id) {
      if (id === `\0${moduleId}`)
        return `export default ${JSON.stringify(initial)};`;
    },
    configureServer(server) {
      server.middlewares.use("/software-bill.cdx.json", (_req, res) => {
        res.setHeader("Content-Type", "application/json");
        res.end(JSON.stringify(initial));
      });
    },
    async generateBundle(_options, bundle) {
      const output = await createSoftwareBill(context);
      output.metadata.properties.push({
        name: "ruf:frontend:resolution",
        value: "rendered-modules",
      });
      const ids = Object.values(bundle).flatMap((item) =>
        item.type === "chunk" ? Object.keys(item.modules) : [],
      );
      await addFrontend(output, ids, (id) => {
        const info = this.getModuleInfo(id);
        return [
          ...(info?.importedIds ?? []),
          ...(info?.dynamicallyImportedIds ?? []),
        ];
      });
      validateSoftwareBill(output);
      this.emitFile({
        type: "asset",
        fileName: "software-bill.cdx.json",
        source: JSON.stringify(output, null, 2),
      });
    },
  };
}
