<script lang="ts">
  // Connect to YouTube Music: own Google window (primary) or import the session
  // from a Firefox/Zen browser / paste the cookie (FR-004-17, FR-003-01).
  import { commands, type BrowserProfile, type SessionAccount } from "../bindings";
  import { auth } from "../stores/auth.svelte";
  import { library } from "../stores/library.svelte";
  import { ui } from "../stores/ui.svelte";
  import { thumbUrl } from "../thumb";
  import { t } from "../i18n.svelte";
  import DialogFrame from "../components/xp/DialogFrame.svelte";

  let browsers = $state<BrowserProfile[]>([]);
  let accounts = $state<SessionAccount[]>([]);
  let pasted = $state("");
  let busy = $state(false);
  let error = $state<string | null>(null);
  void commands.authDetectBrowsers().then((b) => (browsers = b));

  async function run<T>(p: Promise<{ status: "ok"; data: T } | { status: "error"; error: { message: string } }>): Promise<T | null> {
    busy = true;
    error = null;
    const r = await p;
    busy = false;
    if (r.status === "error") {
      error = r.error.message;
      return null;
    }
    return r.data;
  }

  let waiting = $state(false);

  /// With a single account there is nothing to choose.
  async function offer(found: SessionAccount[] | null) {
    accounts = found ?? [];
    if (accounts.length === 1) await choose(accounts[0]!);
  }
  async function signIn() {
    waiting = true;
    const found = await run(commands.authSignIn());
    waiting = false;
    await offer(found);
  }
  async function scan(path: string) {
    await offer(await run(commands.authScanBrowser(path)));
  }
  async function scanPasted() {
    const found = await run(commands.authScanCookie(pasted));
    pasted = "";
    await offer(found);
  }
  async function choose(a: SessionAccount) {
    if (await run(commands.authSelectAccount(a))) {
      ui.loginOpen = false;
      void library.refresh();
    }
  }
  const where = (a: SessionAccount) =>
    typeof a.source === "object" && a.source.Browser ? a.source.Browser.containerName : a.source === "App" ? t.login.google : t.login.pasted;
</script>

<DialogFrame title={t.login.title} onclose={() => (ui.loginOpen = false)} width={460}>
  <div class="login">
    {#if auth.accountName && !auth.signedIn}<p class="warn">{t.login.expired(auth.accountName)}</p>{/if}
    <p>{t.login.intro}</p>

    <button class="xp-btn big" disabled={busy} onclick={signIn}>
      <span class="g">G</span>
      {t.login.signIn}
    </button>
    {#if waiting}<p class="help">{t.login.waiting}</p>{/if}

    {#if accounts.length > 1}
      <fieldset>
        <legend>{t.login.accounts}</legend>
        {#each accounts as a}
          <button class="account" onclick={() => choose(a)} disabled={busy}>
            {#if a.photo}<img src={thumbUrl(a.photo)} alt="" />{:else}<span class="avatar">{a.name[0]}</span>{/if}
            <span class="grow"><b>{a.name}</b> {a.handle ?? ""}<br /><small>{where(a)}</small></span>
          </button>
        {/each}
      </fieldset>
    {/if}

    <details>
      <summary>{t.login.other}</summary>
      <fieldset>
        <legend>{t.login.browsers}</legend>
        {#each browsers as b}
          <div class="row">
            <span class="grow"><b>{b.browser}</b> — {b.name}</span>
            <button class="xp-btn" disabled={busy} onclick={() => scan(b.path)}>{t.login.scan}</button>
          </div>
        {:else}
          <p>{t.login.noBrowsers}</p>
        {/each}
      </fieldset>
      <fieldset>
        <legend>{t.login.pasteTitle}</legend>
        <div class="row">
          <input class="xp-input grow" type="password" bind:value={pasted} placeholder="SAPISID=…; SID=…" />
          <button class="xp-btn" disabled={busy || !pasted} onclick={scanPasted}>{t.login.paste}</button>
        </div>
      </fieldset>
    </details>

    {#if busy}<p>{t.views.loading}</p>{/if}
    {#if error}<p class="err">{error}</p>{/if}

    <div class="buttons"><button class="xp-btn" onclick={() => (ui.loginOpen = false)}>{t.login.skip}</button></div>
  </div>
</DialogFrame>

<style>
  .login {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .login p {
    margin: 0;
    line-height: 1.4;
  }
  .big {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 6px 12px;
    font-weight: bold;
  }
  .g {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    color: #4285f4;
    font-family: Arial, sans-serif;
    font-weight: bold;
    box-shadow: 0 0 0 1px #c5c5c5;
  }
  details fieldset {
    margin-top: 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
  }
  .grow {
    flex: 1;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    margin: 2px 0;
    padding: 4px;
    border: 1px solid transparent;
    border-radius: 3px;
    background: none;
    text-align: left;
    font: inherit;
    cursor: pointer;
  }
  .account:hover {
    border-color: var(--hover-border);
    background: var(--hover-bg);
  }
  .account img,
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
  }
  .avatar {
    display: grid;
    place-items: center;
    background: radial-gradient(circle at 40% 30%, #8ec3ff, #1f5fd1);
    color: #fff;
    font-weight: bold;
  }
  .help {
    color: var(--text-muted);
    margin: 4px 0 !important;
  }
  .warn {
    color: var(--warn);
  }
  .err {
    color: var(--error);
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
  }
</style>
