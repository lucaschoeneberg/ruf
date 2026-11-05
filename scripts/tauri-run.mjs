// Start the native CLI with build variables already present in the OS environment.
import { createRequire } from "node:module";
const require = createRequire(
  new URL("../apps/desktop/package.json", import.meta.url),
);
await require("@tauri-apps/cli").run(
  process.argv.slice(2),
  "bun run desktop:build",
);
