<script lang="ts">
  // Options in the style of an XP property sheet (FR-004-14).
  import { commands, type AudioDevice, type Language, type Settings, type Theme, type ToolVersions, type VizPreset } from "../bindings";
  import { settings } from "../stores/settings.svelte";
  import { auth } from "../stores/auth.svelte";
  import { ui } from "../stores/ui.svelte";
  import { updates } from "../stores/updates.svelte";
  import { t } from "../i18n.svelte";
  import logo from "../../assets/logo.webp";
  import DialogFrame from "../components/xp/DialogFrame.svelte";
  import { VIZ_ORDER } from "../viz/presets";

  type Tab = "player" | "audio" | "keys" | "account" | "viz" | "about";
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
  let tab = $state<Tab>("player");
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
  // Each language is named in itself, so it can be found whatever the current one is.
  const languages: [Language, string][] = [["en", "English"], ["es", "Español"]];
  const vizs: VizPreset[] = VIZ_ORDER;

  async function apply() {
    if (!draft) return;
    const deviceChanged = draft.outputDevice !== settings.current?.outputDevice;
    await settings.update(draft);
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

<DialogFrame title={t.options.title} onclose={close} width={430}>
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
          {#each vizs as v}
            <label><input type="radio" name="viz" checked={draft.visualization === v} onchange={() => draft && (draft.visualization = v)} /> {t.options.vizNames[v]}</label>
          {/each}
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
    border: 1px solid #919b9c;
    border-bottom: 0;
    border-radius: 3px 3px 0 0;
    background: linear-gradient(to bottom, #fff, #ecebe5 80%, #d8d2c6);
    font: inherit;
  }
  .tab.active {
    z-index: 1;
    padding-bottom: 5px;
    margin-top: -2px;
    background: #fcfcfe;
    box-shadow: inset 0 2px #f9b63c;
  }
  .page {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 220px;
    padding: 10px;
    border: 1px solid #919b9c;
    background: #fcfcfe;
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
    color: #555;
  }
  .about p {
    margin: 2px 0;
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
