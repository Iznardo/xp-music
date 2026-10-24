<script lang="ts">
  // XP message box and text box (FR-004-15). Controlled from `ui`.
  import { ui } from "../../stores/ui.svelte";
  import { t } from "../../i18n.svelte";
  import DialogFrame from "./DialogFrame.svelte";
  import { focusOnMount } from "../../focus";

  let value = $state("");
  $effect(() => {
    if (ui.prompt) value = ui.prompt.value;
  });

  function answer(b: string) {
    const m = ui.message;
    ui.message = null;
    m?.resolve(b);
  }
  function submit(v: string | null) {
    const p = ui.prompt;
    ui.prompt = null;
    p?.resolve(v);
  }
</script>

{#if ui.message}
  {@const m = ui.message}
  <DialogFrame title={m.title} onclose={() => answer(m.buttons[m.buttons.length - 1] ?? "")}>
    <div class="msg">
      <span class="icon {m.kind}">{m.kind === "error" ? "✕" : m.kind === "warning" ? "!" : m.kind === "question" ? "?" : "i"}</span>
      <p>{m.text}</p>
    </div>
    <div class="buttons">
      {#each m.buttons as b, i}
        {#if i === 0}<button class="xp-btn" onclick={() => answer(b)} use:focusOnMount>{b}</button>{:else}<button class="xp-btn" onclick={() => answer(b)}>{b}</button>{/if}
      {/each}
    </div>
  </DialogFrame>
{/if}

{#if ui.prompt}
  {@const p = ui.prompt}
  <DialogFrame title={p.title} onclose={() => submit(null)}>
    <form
      onsubmit={(e) => {
        e.preventDefault();
        submit(value);
      }}
    >
      <label class="field">{p.label}<input class="xp-input" bind:value use:focusOnMount /></label>
      <div class="buttons">
        <button class="xp-btn" type="submit">{t.dialogs.ok}</button>
        <button class="xp-btn" type="button" onclick={() => submit(null)}>{t.dialogs.cancel}</button>
      </div>
    </form>
  </DialogFrame>
{/if}

<style>
  .msg {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    min-width: 260px;
    max-width: 420px;
    padding: 8px 4px 14px;
  }
  .msg p {
    margin: 6px 0 0;
    white-space: pre-line;
  }
  .icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    color: #fff;
    font: bold 18px Georgia, serif;
    box-shadow: inset 0 -3px 4px rgb(0 0 0 / 0.3), 0 1px 2px rgb(0 0 0 / 0.4);
    background: radial-gradient(circle at 40% 30%, #8ec3ff, #1f5fd1);
  }
  .icon.error {
    background: radial-gradient(circle at 40% 30%, #ff9a8a, #c41f0b);
  }
  .icon.warning {
    border-radius: 3px;
    background: radial-gradient(circle at 40% 30%, #fff1a0, #e5b100);
    color: #000;
  }
  .buttons {
    display: flex;
    justify-content: center;
    gap: 6px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 300px;
    margin-bottom: 12px;
  }
</style>
