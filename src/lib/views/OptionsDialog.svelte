<script lang="ts">
  // Opciones estilo hoja de propiedades de XP (FR-004-14).
  import { commands, type AudioDevice, type Settings, type Theme, type VizPreset } from "../bindings";
  import { settings } from "../stores/settings.svelte";
  import { auth } from "../stores/auth.svelte";
  import { ui } from "../stores/ui.svelte";
  import { t } from "../strings";
  import DialogFrame from "../components/xp/DialogFrame.svelte";

  type Tab = "player" | "audio" | "keys" | "account" | "viz";
  type HotkeyName = keyof Settings["hotkeys"];
  const hotkeyNames: HotkeyName[] = ["playPause", "next", "previous", "show"];

  /// Convierte la pulsación en un acelerador ("Ctrl+Alt+P"). Supr/Retroceso lo quitan.
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
    // Sin modificador solo se aceptan teclas F (si no, cualquier letra quedaría secuestrada).
    if (!mods.length && !/^F\d+$/.test(key)) return;
    draft.hotkeys[name] = [...mods, key].join("+");
  }
  let tab = $state<Tab>("player");
  let draft = $state<Settings | null>(settings.current ? { ...settings.current } : null);
  let devices = $state<AudioDevice[]>([]);
  void commands.playerListDevices().then((d) => (devices = d));

  const themes: Theme[] = ["Blue", "Olive", "Silver"];
  const vizs: VizPreset[] = ["Bars", "Waves", "Ambience", "Battery", "Cover"];

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
          <legend>{t.options.title}</legend>
          <label><input type="checkbox" bind:checked={draft.closeToTray} /> {t.options.closeToTray}</label>
          <label><input type="checkbox" bind:checked={draft.startMinimized} /> {t.options.startMinimized}</label>
          <label><input type="checkbox" bind:checked={draft.launchAtLogin} /> {t.options.launchAtLogin}</label>
          <label><input type="checkbox" bind:checked={draft.notifications} /> {t.options.notifications}</label>
          <label><input type="checkbox" bind:checked={draft.xpSounds} /> {t.options.xpSounds}</label>
          <label><input type="checkbox" bind:checked={draft.pauseOnLock} /> {t.options.pauseOnLock}</label>
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
  .row {
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
