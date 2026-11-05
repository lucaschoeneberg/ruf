// One feature/target selection shared by Tauri, Vite and the SBOM generator.
export function desktopContext(args, hostTarget, configuredTarget) {
  const values = (long, short) => {
    const result = [];
    for (let i = 0; i < args.length; i++) {
      const arg = args[i];
      if (arg.startsWith(`${long}=`))
        result.push(...arg.slice(long.length + 1).split(/[ ,]+/));
      else if (arg === long || arg === short) {
        while (i + 1 < args.length && !args[i + 1].startsWith("-"))
          result.push(...args[++i].split(/[ ,]+/));
      }
    }
    return result.filter(Boolean);
  };
  const targets = values("--target", "-t");
  if (targets.length > 1) throw new Error("Choose exactly one SBOM target.");
  const target = targets[0] ?? configuredTarget ?? hostTarget;
  if (
    !/^[a-z0-9_]+(?:-[a-z0-9_]+){2,}$/.test(target) ||
    target === "universal-apple-darwin"
  )
    throw new Error(
      "A separate SBOM is required for each concrete Rust target.",
    );
  if (args.includes("--"))
    throw new Error(
      "Pass desktop options before '--'; the wrapper supplies --locked.",
    );
  const features = values("--features", "-f");
  return {
    target,
    native: features.some(
      (feature) => feature === "native" || feature === "sip-desktop/native",
    ),
    profile:
      args.includes("--dev") || args.includes("--debug") || args.includes("-d")
        ? "debug"
        : "release",
    kind: "desktop",
  };
}
