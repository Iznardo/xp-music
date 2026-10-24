import { mount } from "svelte";
import "./styles/tokens.css";
import "./styles/base.css";
import App from "./App.svelte";
import { commands } from "./lib/bindings";

// The WebView has no visible console: errors go to the app log.
addEventListener("error", (e) => void commands.uiLog("error", `${e.message} (${e.filename}:${e.lineno})`));
addEventListener("unhandledrejection", (e) => void commands.uiLog("error", `unhandled rejection: ${String(e.reason?.stack ?? e.reason)}`));

const target = document.getElementById("app");
if (!target) throw new Error("missing #app");

export default mount(App, { target });
void commands.uiLog("info", `UI mounted at ${location.hash || "(no route)"}`);
