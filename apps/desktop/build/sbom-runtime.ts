// Build scripts run with the workspace's pinned Bun runtime, without extra dependencies.
declare const Bun: {
  env: Record<string, string | undefined>;
  file(path: URL | string): {
    name: string;
    text(): Promise<string>;
    arrayBuffer(): Promise<ArrayBuffer>;
  };
  spawnSync(
    args: string[],
    options: { cwd: string },
  ): { success: boolean; stdout: Uint8Array; stderr: Uint8Array };
  resolveSync(module: string, parent: string): string;
  CryptoHasher: new (algorithm: string) => {
    update(data: string | ArrayBuffer): { digest(encoding: "hex"): string };
  };
};
export const runtime = Bun;
export const root = new URL("../../../", import.meta.url);
export const read = (path: string) => runtime.file(new URL(path, root)).text();
export function command(args: string[]): string {
  const result = runtime.spawnSync(args, { cwd: runtime.file(root).name });
  if (!result.success)
    throw new Error(
      `SBOM: ${args[0]} failed: ${new TextDecoder().decode(result.stderr)}`,
    );
  return new TextDecoder().decode(result.stdout);
}
export const sha256 = (content: string | ArrayBuffer) =>
  new runtime.CryptoHasher("sha256").update(content).digest("hex");
