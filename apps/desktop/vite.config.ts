import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { softwareBillPlugin } from "./build/software-bill.js";

export default defineConfig({
  plugins: [svelte(), softwareBillPlugin()],
  clearScreen: false,
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: { target: ["es2022", "safari17"] },
});
