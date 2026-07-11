import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri expects a fixed dev port and no auto-clearing of the terminal.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // Don't watch the Rust side; cargo handles that.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Produce a build compatible with the Windows webview (Chromium/Edge).
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
  },
});
