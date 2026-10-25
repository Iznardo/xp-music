// Windows and macOS smoke test: starts the app and waits for the log to say the UI is
// mounted and that it found yt-dlp/deno; then closes it. (Linux: smoke-linux.sh.)
//   node scripts/smoke-desktop.mjs EXECUTABLE [SECONDS]
import { spawn, spawnSync } from "node:child_process";
import { closeSync, openSync, readSync, readdirSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const [binary, seconds = "60"] = process.argv.slice(2);
if (!binary) {
  console.error("Usage: node scripts/smoke-desktop.mjs EXECUTABLE [SECONDS]");
  process.exit(2);
}

// Same path as AppDirs::logs (dirs::data_local_dir()/xp-music/logs).
const dataLocal = process.platform === "win32" ? process.env.LOCALAPPDATA
  : process.platform === "darwin" ? join(homedir(), "Library", "Application Support")
  : process.env.XDG_DATA_HOME ?? join(homedir(), ".local", "share");
const logs = join(dataLocal, "xp-music", "logs");
const expected = ["UI mounted", "tools found"];

const sizes = () => {
  try {
    return new Map(readdirSync(logs).map((name) => [name, statSync(join(logs, name)).size]));
  } catch {
    return new Map();
  }
};
// Only what is written after starting the app counts (there may be logs from earlier runs).
const before = sizes();
function newLog() {
  let text = "";
  for (const [name, size] of sizes()) {
    const from = before.get(name) ?? 0;
    if (size <= from) continue;
    const buffer = Buffer.alloc(size - from);
    const fd = openSync(join(logs, name), "r");
    try { readSync(fd, buffer, 0, buffer.length, from); } finally { closeSync(fd); }
    text += buffer.toString("utf8");
  }
  return text;
}

const child = spawn(binary, [], { stdio: "ignore" });
let exited = false;
child.on("exit", () => { exited = true; });

function stop() {
  if (exited) return;
  // On Windows the WebView2 processes have to be closed too.
  if (process.platform === "win32") spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"]);
  else child.kill("SIGKILL");
}

let log = "";
const deadline = Date.now() + Number(seconds) * 1000;
while (Date.now() < deadline && !exited) {
  log = newLog();
  if (expected.every((line) => log.includes(line))) {
    stop();
    console.log(`Smoke OK (${process.platform}): ${expected.join(" and ")}.`);
    process.exit(0);
  }
  await new Promise((resolve) => setTimeout(resolve, 500));
}
log = newLog();
stop();
console.error(`${exited ? "The app exited early" : "Timed out"}; app log:\n${log || "(empty)"}`);
process.exit(1);
