import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

/**
 * Misma política que pone el backend al marco de los skins (src-tauri/src/skins.rs).
 * El marco no tiene origen propio (sandbox), así que 'self' no basta: se nombra.
 */
const skinCsp = (origin: string) =>
  `default-src 'none'; script-src ${origin} 'unsafe-eval'; style-src 'unsafe-inline'; img-src blob: data: http://thumb.localhost https:; connect-src ${origin}`;

/**
 * Solo en la vista previa (`dev:mock`): sirve el marco de los skins, el banco de
 * pruebas y los .wmz de la carpeta `XP_SKINS_DIR` bajo `/skin/`, como hace el
 * protocolo `skin` de la app.
 */
function skinDev(): Plugin {
  const frame = fileURLToPath(new URL("./src/skin-frame/", import.meta.url));
  const bench = fileURLToPath(new URL("./scripts/skin-bench/", import.meta.url));
  const skins = process.env.XP_SKINS_DIR ? path.resolve(process.env.XP_SKINS_DIR) : null;
  const list = (dir: string, base = ""): { id: string; name: string; bytes: number }[] =>
    fs.readdirSync(dir, { withFileTypes: true }).flatMap((d) => {
      const rel = base ? `${base}/${d.name}` : d.name;
      if (d.isDirectory()) return list(path.join(dir, d.name), rel);
      if (!/\.wmz$/i.test(d.name)) return [];
      return [{ id: rel, name: d.name.replace(/\.wmz$/i, ""), bytes: fs.statSync(path.join(dir, d.name)).size }];
    });
  return {
    name: "xp-skins-dev",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use("/skin", (req, res) => {
        const url = decodeURIComponent((req.url ?? "/").split("?")[0]!);
        const send = (status: number, type: string, body: string | Buffer) => {
          res.statusCode = status;
          res.setHeader("Content-Type", type);
          res.setHeader("Access-Control-Allow-Origin", "*");
          res.setHeader("Cache-Control", "no-store");
          res.end(body);
        };
        const file = /^\/(frame|bench)\/([\w-]+\.(html|js))$/.exec(url);
        if (file) {
          const p = path.join(file[1] === "frame" ? frame : bench, file[2]!);
          if (!fs.existsSync(p)) return send(404, "text/plain", "");
          if (file[1] === "frame") res.setHeader("Content-Security-Policy", skinCsp(`http://${req.headers.host}`));
          return send(200, file[3] === "html" ? "text/html" : "text/javascript", fs.readFileSync(p));
        }
        if (url === "/list") return send(200, "application/json", JSON.stringify(skins ? list(skins) : []));
        const wmz = /^\/wmz\/(.+\.wmz)$/i.exec(url);
        if (wmz && skins) {
          const p = path.resolve(skins, wmz[1]!);
          if (p.startsWith(skins + path.sep) && fs.existsSync(p)) return send(200, "application/octet-stream", fs.readFileSync(p));
        }
        send(404, "text/plain", "");
      });
    },
  };
}

// Opciones pensadas para Tauri: puerto fijo, sin limpiar pantalla (errores de Rust visibles)
// y sin vigilar src-tauri. Con `--mode mock` la UI usa datos simulados en vez del backend.
export default defineConfig(({ mode }) => ({
  plugins: [svelte(), ...(mode === "mock" ? [skinDev()] : [])],
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
    // Imágenes siempre como archivo aparte (no en base64 dentro del JS): el logo
    // solo se descarga al abrir "Acerca de".
    assetsInlineLimit: 0,
    outDir: "dist",
    emptyOutDir: true,
  },
}));
