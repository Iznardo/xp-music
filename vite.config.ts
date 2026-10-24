import { defineConfig, type Plugin } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import path from "node:path";

const host = process.env.TAURI_DEV_HOST;

/**
 * Same policy the backend applies to the skin frame (src-tauri/src/skins.rs).
 * The frame has no origin of its own (sandbox), so 'self' is not enough: it is named explicitly.
 */
const skinCsp = (origin: string) =>
  `default-src 'none'; script-src ${origin} 'unsafe-eval'; style-src 'unsafe-inline'; img-src blob: data: http://thumb.localhost https:; connect-src ${origin}`;

/**
 * Only in the preview (`dev:mock`): serves the skin frame, the test bench and the
 * .wmz files from the `XP_SKINS_DIR` folder under `/skin/`, like the app's `skin`
 * protocol does.
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

// Options tailored for Tauri: fixed port, no screen clearing (Rust errors stay visible)
// and no watching of src-tauri. With `--mode mock` the UI uses simulated data instead of the backend.
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
    // Images always as a separate file (not base64 inside the JS): the logo
    // is only downloaded when opening "About".
    assetsInlineLimit: 0,
    outDir: "dist",
    emptyOutDir: true,
  },
}));
