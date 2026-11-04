import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

// Resolve from the owning workspace, including installations without hoisting.
const require = createRequire(
  new URL("../apps/desktop/package.json", import.meta.url),
);
const { run } = require("@tauri-apps/cli");
process.chdir(fileURLToPath(new URL("../apps/desktop/", import.meta.url)));

// Tauri options precede '--'; Cargo alone consumes the immutable-lockfile flag.
await run(
  ["build", ...process.argv.slice(2), "--", "--locked"],
  "npm run desktop:build",
);
