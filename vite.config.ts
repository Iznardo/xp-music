import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

const host = process.env.TAURI_DEV_HOST;

// Opciones pensadas para Tauri: puerto fijo, sin limpiar pantalla (errores de Rust visibles)
// y sin vigilar src-tauri. Con `--mode mock` la UI usa datos simulados en vez del backend.
export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  clearScreen: false,
  resolve: {
    alias:
      mode === "mock"
        ? [{ find: /^(\.\.?\/)+(lib\/)?bindings$/, replacement: fileURLToPath(new URL("./src/lib/mock/bindings.mock.ts", import.meta.url)) }]
        : [],
  },
  server: {
    port: mode === "mock" ? 1430 : 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "es2022",
    outDir: "dist",
    emptyOutDir: true,
  },
}));
