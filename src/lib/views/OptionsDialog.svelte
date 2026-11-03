<script lang="ts">
  // Options in the style of an XP property sheet (FR-004-14).
  import { commands, type Appearance, type AudioDevice, type Language, type Settings, type Theme, type ToolVersions, type VizPreset } from "../bindings";
  import { settings } from "../stores/settings.svelte";
  import { auth } from "../stores/auth.svelte";
  import { local } from "../stores/local.svelte";
  import { ui } from "../stores/ui.svelte";
  import { updates } from "../stores/updates.svelte";
  import { t } from "../i18n.svelte";
  import logo from "../../assets/logo.webp";
  import DialogFrame from "../components/xp/DialogFrame.svelte";
  import { vizEntries } from "../viz/presets";
  import { customViz } from "../viz/custom.svelte";

  type Tab = "player" | "audio" | "keys" | "library" | "account" | "viz" | "about";
  type HotkeyName = keyof Settings["hotkeys"];
  const hotkeyNames: HotkeyName[] = ["playPause", "next", "previous", "show"];

  /// Turns the key press into an accelerator ("Ctrl+Alt+P"). Delete/Backspace clear it.
  function captureHotkey(name: HotkeyName, e: KeyboardEvent) {
    e.preventDefault();
    if (!draft) return;
    if (e.key === "Delete" || e.key === "Backspace") {
      draft.hotkeys[name] = null;
      return;
    }
    if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
    const key = e.code.startsWith("Key") ? e.code.slice(3) : e.code.startsWith("Digit") ? e.code.slice(5) : e.key.length === 1 ? e.key.toUpperCase() : e.key;
    const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
    // Without a modifier only F keys are accepted (otherwise any letter would be hijacked).
    if (!mods.length && !/^F\d+$/.test(key)) return;
    draft.hotkeys[name] = [...mods, key].join("+");
  }
  let tab = $state<Tab>(ui.optionsAbout ? "about" : "player");
  ui.optionsAbout = false;
  let draft = $state<Settings | null>(settings.current ? { ...settings.current } : null);
  let devices = $state<AudioDevice[]>([]);
  void commands.playerListDevices().then((d) => (devices = d));

  let versions = $state<ToolVersions | null>(null);
  let updateMsg = $state("");
  let checking = $state(false);
  $effect(() => {
    if (tab === "about" && !versions) void commands.toolsVersions().then((v) => (versions = v));
  });
  async function checkUpdates() {
    checking = true;
    updateMsg = t.options.about.checking;
    const r = await commands.toolsUpdateNow();
    checking = false;
    if (r.status === "error") updateMsg = r.error.message;
    else {
      updateMsg = r.data ? t.options.about.updated(r.data) : t.options.about.upToDate;
      if (r.data) versions = await commands.toolsVersions();
    }
  }

  const themes: Theme[] = ["Blue", "Olive", "Silver"];
  const appearances: Appearance[] = ["System", "Light", "Dark"];
  // Each language is named in itself, so it can be found whatever the current one is.
  const languages: [Language, string][] = [["en", "English"], ["es", "Español"]];
  const vizs = $derived(vizEntries(customViz.list.map((v) => v.name)));
  const vizLabel = (v: (typeof vizs)[number]) => v.patch.customVisualization ?? t.options.vizNames[v.patch.visualization!];
  let vizQuery = $state("");
  const vizShown = $derived(vizs.filter((v) => vizLabel(v).toLowerCase().includes(vizQuery.trim().toLowerCase())));
  let vizList = $state<HTMLDivElement>();
  const vizOn = (id: string) => (draft?.customVisualization && customViz.find(draft.customVisualization) ? `custom:${draft.customVisualization}` : draft?.visualization) === id;
  const vizCurrent = $derived(vizs.find((v) => vizOn(v.id)));
  function vizPick(v: (typeof vizs)[number]) {
    if (draft) Object.assign(draft, v.patch);
  }
  // Single-select listbox: the arrows move the selection, as in the skin chooser.
  function vizKey(e: KeyboardEvent) {
    const n = vizShown.length;
    const at = vizShown.findIndex((v) => v === vizCurrent);
    const to = { ArrowDown: Math.min(n - 1, at + 1), ArrowUp: Math.max(0, at - 1), Home: 0, End: n - 1 }[e.key];
    if (to === undefined || !n) return;
    e.preventDefault();
    vizPick(vizShown[to]!);
  }
  // The active row must be in view when the tab opens, the list is filtered or the arrows move.
  $effect(() => {
    if (tab === "viz") void vizCurrent, vizShown, vizList?.querySelector("[aria-selected=true]")?.scrollIntoView({ block: "nearest" });
  });

  async function apply() {
    if (!draft) return;
    const deviceChanged = draft.outputDevice !== settings.current?.outputDevice;
    // The folders are changed on their own (and right away): the draft's may be stale.
    await settings.update({ ...draft, localFolders: local.folders });
    if (deviceChanged) await commands.playerSetDevice(draft.outputDevice);
  }
  async function accept() {
    await apply();
    ui.optionsOpen = false;
  }
  function close() {
    ui.optionsOpen = false;
  }
  function connect() {
    ui.optionsOpen = false;
    ui.loginOpen = true;
  }
</script>

<DialogFrame title={t.options.title} onclose={close} width={490}>
  {#if draft}
    <div class="tabs" role="tablist">
      {#each Object.entries(t.options.tabs) as [id, label]}
        <button class="tab" class:active={tab === id} role="tab" aria-selected={tab === id} onclick={() => (tab = id as Tab)}>{label}</button>
      {/each}
    </div>
    <div class="page">
      {#if tab === "player"}
        <fieldset>
          <legend>{t.options.theme}</legend>
          {#each themes as th}
            <label><input type="radio" name="theme" checked={draft.theme === th} onchange={() => draft && (draft.theme = th)} /> {t.options.themes[th]}</label>
          {/each}
        </fieldset>
        <fieldset>
          <legend>{t.options.appearance}</legend>
          {#each appearances as ap}
            <label><input type="radio" name="appearance" checked={draft.appearance === ap} onchange={() => draft && (draft.appearance = ap)} /> {t.options.appearances[ap]}</label>
          {/each}
        </fieldset>
        <fieldset>
          <legend>{t.options.language}</legend>
          <select class="xp-input wide" value={draft.language ?? ""} onchange={(e) => draft && (draft.language = (e.currentTarget.value || null) as Language | null)}>
            <option value="">{t.options.languageAuto}</option>
            {#each languages as [id, name]}<option value={id}>{name}</option>{/each}
          </select>
        </fieldset>
        <fieldset>
          <legend>{t.options.title}</legend>
          <label><input type="checkbox" bind:checked={draft.closeToTray} /> {t.options.closeToTray}</label>
          <label><input type="checkbox" bind:checked={draft.startMinimized} /> {t.options.startMinimized}</label>
          <label><input type="checkbox" bind:checked={draft.launchAtLogin} /> {t.options.launchAtLogin}</label>
          <label><input type="checkbox" bind:checked={draft.notifications} /> {t.options.notifications}</label>
          <label><input type="checkbox" bind:checked={draft.xpSounds} /> {t.options.xpSounds}</label>
          <label><input type="checkbox" bind:checked={draft.pauseOnLock} /> {t.options.pauseOnLock}</label>
          <label><input type="checkbox" bind:checked={draft.discordPresence} /> {t.options.discordPresence}</label>
        </fieldset>
      {:else if tab === "keys"}
        <fieldset>
          <legend>{t.options.hotkeys}</legend>
          {#each hotkeyNames as name}
            <label class="hk">
              <span>{t.options.hotkeyNames[name]}</span>
              <input class="xp-input" readonly value={draft.hotkeys[name] ?? ""} placeholder="—" onkeydown={(e) => captureHotkey(name, e)} />
            </label>
          {/each}
          <p class="hint">{t.options.hotkeyHint}</p>
          <p class="hint">{t.options.mediaKeysNote}</p>
        </fieldset>
      {:else if tab === "audio"}
        <fieldset>
          <legend>{t.options.device}</legend>
          <select class="xp-input wide" value={draft.outputDevice ?? ""} onchange={(e) => draft && (draft.outputDevice = e.currentTarget.value || null)}>
            <option value="">{t.options.defaultDevice}</option>
            {#each devices as d}<option value={d.id}>{d.name}</option>{/each}
          </select>
        </fieldset>
        <fieldset>
          <legend>{t.tabs.library}</legend>
          <label><input type="checkbox" bind:checked={draft.normalizeVolume} /> {t.options.normalize}</label>
          <label><input type="checkbox" bind:checked={draft.reportHistory} /> {t.options.reportHistory}</label>
        </fieldset>
      {:else if tab === "library"}
        <fieldset>
          <legend>{t.local.folders}</legend>
          {#if local.folders.length}
            <ul class="folders">
              {#each local.folders as f (f)}
                <li><span title={f}>{f}</span><button class="xp-btn" onclick={() => local.removeFolder(f)}>{t.local.remove}</button></li>
              {/each}
            </ul>
          {:else}
            <p class="hint">{t.local.noFolders}</p>
          {/if}
          <div class="row">
            <button class="xp-btn" onclick={() => local.addFolder()}>{t.local.addFolder}</button>
            <button class="xp-btn" disabled={local.scan.scanning || !local.folders.length} onclick={() => local.rescan()}>{t.local.rescan}</button>
          </div>
          <p class="hint">{local.scan.scanning ? t.local.scanning(local.scan.done, local.scan.total) : t.local.foldersHint}</p>
        </fieldset>
      {:else if tab === "account"}
        <fieldset>
          <legend>{t.options.account}</legend>
          {#if auth.accountName}
            <p><b>{auth.accountName}</b>{auth.signedIn ? "" : ` — ${t.login.expired(auth.accountName)}`}</p>
            <div class="row">
              <button class="xp-btn" onclick={connect}>{t.options.connect}</button>
              <button class="xp-btn" onclick={() => commands.authLogout()}>{t.options.signOut}</button>
            </div>
          {:else}
            <p>{t.options.noAccount}</p>
            <button class="xp-btn" onclick={connect}>{t.options.connect}</button>
          {/if}
        </fieldset>
      {:else if tab === "viz"}
        <fieldset>
          <legend>{t.options.viz}</legend>
          <input class="wide vizsearch" type="search" placeholder={t.options.vizSearch} bind:value={vizQuery} />
          <div class="vizlist" role="listbox" tabindex="0" aria-label={t.options.viz} bind:this={vizList} onkeydown={vizKey}>
            {#each vizShown as v (v.id)}
              <button role="option" tabindex="-1" aria-selected={v === vizCurrent} class:sel={v === vizCurrent} onclick={() => vizPick(v)}>{vizLabel(v)}</button>
            {:else}
              <p class="hint">{t.options.vizNoMatch}</p>
            {/each}
          </div>
          <p class="hint">{t.options.vizCustomHint}</p>
          <div class="vizbtns">
            <button class="xp-btn" onclick={() => void commands.vizCustomOpenDir()}>{t.skins.openFolder}</button>
            <button class="xp-btn" onclick={() => void customViz.refresh()}>{t.skins.reload}</button>
          </div>
        </fieldset>
      {:else if tab === "about"}
        <div class="about">
          <p class="brand"><img src={logo} alt="" width="32" height="32" /><span><b>XP Music</b> — {t.options.about.version} {versions?.app ?? "…"}</span></p>
          <fieldset>
            <legend>{t.options.about.components}</legend>
            <p>yt-dlp: {versions ? (versions.ytDlp ?? t.options.about.unknown) : "…"} {versions?.ytDlpUpdated ? t.options.about.ytDlpUpdated : ""}</p>
            <p>Deno: {versions ? (versions.deno ?? t.options.about.unknown) : "…"}</p>
            <div class="row">
              <button class="xp-btn" disabled={checking} onclick={checkUpdates}>{t.options.about.checkUpdates}</button>
              <button class="xp-btn" onclick={() => commands.toolsOpenLogsDir()}>{t.options.about.openLogs}</button>
            </div>
            {#if updateMsg}<p class="hint">{updateMsg}</p>{/if}
          </fieldset>
          <fieldset>
            <legend>{t.options.about.appUpdates}</legend>
            {#if updates.status && !updates.status.enabled}
              <p class="hint">{t.options.about.disabled}</p>
            {:else}
              <div class="row">
                <button class="xp-btn" disabled={updates.busy} onclick={() => updates.check()}>{t.options.about.checkApp}</button>
              </div>
              {#if updates.message}<p class="hint">{updates.message}</p>{/if}
            {/if}
          </fieldset>
          <p class="hint">{t.options.about.credits}</p>
          <p class="hint">{t.options.about.notice}</p>
        </div>
      {/if}
    </div>
    <div class="buttons">
      <button class="xp-btn" onclick={accept}>{t.options.ok}</button>
      <button class="xp-btn" onclick={close}>{t.options.cancel}</button>
      <button class="xp-btn" onclick={apply}>{t.options.apply}</button>
    </div>
  {/if}
</DialogFrame>

<style>
  .tabs {
    display: flex;
    gap: 1px;
    padding-left: 4px;
  }
  .tab {
    position: relative;
    top: 1px;
    padding: 3px 8px;
    border: 1px solid var(--tab-border);
    border-bottom: 0;
    border-radius: 3px 3px 0 0;
    background: var(--ctl-face);
    font: inherit;
  }
  .tab.active {
    z-index: 1;
    padding-bottom: 5px;
    margin-top: -2px;
    background: var(--view-bg);
    box-shadow: inset 0 2px #f9b63c;
  }
  .page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 220px;
    padding: 10px;
    border: 1px solid var(--tab-border);
    background: var(--view-bg);
  }
  fieldset label {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 0;
  }
  .wide {
    width: 100%;
  }
  .hk {
    justify-content: space-between;
  }
  .hk input {
    width: 150px;
  }
  .hint {
    margin: 6px 0 0;
    color: var(--text-muted);
  }
  .about p {
    margin: 2px 0;
  }
  /* The user can add up to 64 visualizations: the list scrolls, the buttons stay. */
  .vizsearch {
    margin-bottom: 6px;
  }
  .vizbtns {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .vizlist {
    height: 180px;
    overflow-y: auto;
    border: 1px solid var(--input-border);
    background: var(--view-bg);
  }
  .vizlist button {
    display: block;
    width: 100%;
    padding: 2px 6px;
    border: 0;
    background: none;
    font: inherit;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: default;
  }
  .vizlist .sel {
    background: var(--selection);
    color: #fff;
  }
  .folders {
    margin: 0 0 4px;
    padding: 0;
    list-style: none;
    max-height: 120px;
    overflow: auto;
  }
  .folders li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
  }
  .folders span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 1.15em;
  }
  .row {
    flex-wrap: wrap;
    margin-top: 4px;
    display: flex;
    gap: 6px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 10px;
  }
</style>
