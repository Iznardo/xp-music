import { mount } from "svelte";
import "./styles/tokens.css";
import "./styles/base.css";
import App from "./App.svelte";
import { commands } from "./lib/bindings";

// La WebView no tiene consola visible: los errores van al log de la app.
addEventListener("error", (e) => void commands.uiLog("error", `${e.message} (${e.filename}:${e.lineno})`));
addEventListener("unhandledrejection", (e) => void commands.uiLog("error", `promesa rechazada: ${String(e.reason?.stack ?? e.reason)}`));

const target = document.getElementById("app");
if (!target) throw new Error("falta #app");

export default mount(App, { target });
void commands.uiLog("info", `UI montada en ${location.hash || "(sin ruta)"}`);
