<script lang="ts">
  import { app, ACCENTS, type ThemePref } from '../lib/store.svelte';
  import { fmtCount, fmtDate } from '../lib/util';
  import Toggle from './Toggle.svelte';
  import Seg from './Seg.svelte';

  let dlg: HTMLDialogElement | undefined = $state();
  const themes: { value: ThemePref; label: string }[] = [
    { value: 'system', label: 'System' }, { value: 'light', label: 'Light' }, { value: 'dark', label: 'Dark' },
  ];

  $effect(() => {
    if (!dlg) return;
    if (app.settingsOpen && !dlg.open) dlg.showModal();
    if (!app.settingsOpen && dlg.open) dlg.close();
  });

  function backdrop(e: MouseEvent) {
    if (e.target === dlg) app.settingsOpen = false;
  }
  const g = $derived(app.games);
</script>

<dialog bind:this={dlg} aria-labelledby="set-h" onclose={() => (app.settingsOpen = false)} onclick={backdrop}>
  <div class="card">
    <div class="head">
      <h2 id="set-h">Settings</h2>
      <button type="button" class="icon-btn" aria-label="Close settings" onclick={() => (app.settingsOpen = false)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      </button>
    </div>

    <div class="body">
      <section aria-label="Startup">
        <Toggle label="Start with my computer" checked={app.settings.autostart} onchange={(v) => app.updateSettings({ autostart: v })} />
        <Toggle label="Start hidden in the tray" hint="The window stays closed until you open it from the tray icon." checked={app.settings.start_in_tray} onchange={(v) => app.updateSettings({ start_in_tray: v })} />
        <Toggle label="Connect when Decoy starts" hint="Shows your active profile as soon as Discord is running." checked={app.settings.connect_on_launch} onchange={(v) => app.updateSettings({ connect_on_launch: v })} />
      </section>

      <section class="grp" aria-labelledby="set-look">
        <h3 id="set-look">Look</h3>
        <div class="field">
          <span class="lab" id="set-acc">Accent</span>
          <div class="accents" role="radiogroup" aria-labelledby="set-acc">
            {#each ACCENTS as a (a.id)}
              <label class="chip">
                <input type="radio" class="sr" name="accent" value={a.id} checked={app.accent === a.id} onchange={() => app.setAccent(a.id)} />
                <span><i style:background={a.color}></i>{a.name}</span>
              </label>
            {/each}
          </div>
        </div>
        <div class="field">
          <span class="lab">Theme</span>
          <Seg label="Theme" options={themes} value={app.theme} onchange={(v) => app.setTheme(v)} />
        </div>
      </section>

      <section class="grp" aria-labelledby="set-games">
        <h3 id="set-games">Discord's game list</h3>
        <div class="pick games">
          <span class="gl">
            {#if g.loading}Downloading...
            {:else if g.count > 0}<b>{fmtCount(g.count)} games</b>
            {:else}<b>Not downloaded</b>{/if}
          </span>
          {#if g.count > 0 && !g.loading}<small>updated {fmtDate(g.fetched_at)}</small>{/if}
          <button type="button" class="btn" disabled={g.loading} onclick={() => app.refreshGames()}>
            {#if g.loading}<span class="spinner"></span>{/if}Refresh game list
          </button>
        </div>
        {#if g.error}<p class="err">Could not refresh the list. {g.error} Check your connection and try again.</p>{/if}
      </section>
    </div>

    <div class="foot">
      <p class="caption">Decoy {app.version || '0.1.0'}<br />Not affiliated with Discord or any game publisher.</p>
      <button type="button" class="btn primary" onclick={() => (app.settingsOpen = false)}>Done</button>
    </div>
  </div>
</dialog>

<style>
  dialog {
    border: 1px solid var(--line); border-radius: 14px; padding: 0; background: var(--surface); color: var(--ink);
    box-shadow: var(--shadow); width: min(440px, calc(100vw - 32px)); max-height: calc(100vh - 32px); overflow: hidden;
  }
  dialog[open] { display: flex; }
  dialog::backdrop { background: color-mix(in oklab, var(--ink) 38%, transparent); }
  .card { display: flex; flex-direction: column; min-height: 0; width: 100%; }
  .head { display: flex; align-items: center; justify-content: space-between; padding: 14px 14px 6px 20px; }
  h2 { font: 700 19px/1.1 var(--display); letter-spacing: -.01em; }
  .body { padding: 0 20px 6px; overflow-y: auto; min-height: 0; display: flex; flex-direction: column; gap: 4px; }
  .body > section:first-child { border-bottom: 1px solid var(--line); padding-bottom: 6px; }
  .grp { display: flex; flex-direction: column; gap: 10px; padding: 12px 0 8px; }
  .grp h3 { font: 700 14px/1.2 var(--display); }
  .lab { font-weight: 500; color: var(--muted); }
  .accents { display: flex; flex-wrap: wrap; gap: 8px; }
  .chip { cursor: pointer; position: relative; }
  .chip span {
    font-weight: 500; background: var(--surface); border: 1px solid var(--line); border-radius: 999px; padding: 6px 12px 6px 8px;
    display: flex; gap: 8px; align-items: center;
  }
  .chip span:hover { border-color: var(--muted); }
  .chip i { width: 16px; height: 16px; border-radius: 50%; border: 1px solid rgba(26, 29, 35, .25); }
  .chip input:checked + span { border-color: var(--ink); box-shadow: 0 0 0 1px var(--ink); }
  .chip input:focus-visible + span { outline: 2px solid var(--ink); outline-offset: 2px; }
  .games { gap: 10px; padding: 8px 8px 8px 12px; }
  .games .gl { white-space: nowrap; }
  .games small { margin-left: 0; }
  .games .btn { margin-left: auto; }
  .foot { display: flex; align-items: flex-end; justify-content: space-between; gap: 16px; padding: 12px 20px 16px; border-top: 1px solid var(--line); margin-top: 8px; }
  .foot .caption { line-height: 1.5; }
</style>
