<script lang="ts">
  import { app } from './lib/store.svelte';
  import TopBar from './components/TopBar.svelte';
  import StatusBand from './components/StatusBand.svelte';
  import Form from './components/Form.svelte';
  import Welcome from './components/Welcome.svelte';
  import Preview from './components/Preview.svelte';
  import SettingsDialog from './components/SettingsDialog.svelte';
  import Toasts from './components/Toasts.svelte';
  import Guide from './components/guide/Guide.svelte';
  import { guide } from './lib/guide.svelte';
</script>

<div class="win">
  {#if app.loadError}
    <main class="fail">
      <h1>Decoy could not start</h1>
      <p>{app.loadError}</p>
      <button type="button" class="btn primary" onclick={() => app.load()}>Try again</button>
    </main>
  {:else if !app.ready}
    <main class="fail" aria-busy="true"><span class="spinner"></span></main>
  {:else}
    <TopBar />
    {#if guide.open}
      <Guide />
    {:else}
    <StatusBand />
    <div class="body">
      <div class="formwrap">
        <main class="form" id="form">
          {#if app.draft}
            {#key app.draft.id}<Form />{/key}
          {:else}
            <Welcome />
          {/if}
        </main>
        <div class="fade" aria-hidden="true"></div>
        <div class="saved" class:on={app.saved} role="status">{#if app.saved}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/></svg>Saved{/if}</div>
      </div>
      <Preview />
    </div>
    {/if}
    <SettingsDialog />
  {/if}
  <Toasts />
</div>

<style>
  .win { height: 100%; display: flex; flex-direction: column; min-height: 0; background: var(--surface); }
  .body { flex: 1; display: grid; grid-template-columns: minmax(0, 1fr) 348px; min-height: 0; }
  .formwrap { position: relative; min-height: 0; min-width: 0; display: flex; }
  .form { flex: 1; overflow-y: auto; padding: 18px 20px 44px; min-width: 0; }
  .fade { position: absolute; inset: auto 0 0; height: 30px; background: linear-gradient(transparent, var(--surface)); pointer-events: none; }
  .saved {
    position: absolute; right: 18px; bottom: 12px; display: flex; align-items: center; gap: 5px; font-size: 12px; font-weight: 500;
    color: var(--muted); background: var(--sunken); border: 1px solid var(--line); border-radius: 999px; padding: 3px 10px 3px 8px;
    opacity: 0; transform: translateY(4px); transition: opacity .2s, transform .2s; pointer-events: none;
  }
  .saved.on { opacity: 1; transform: none; }
  .saved svg { width: 12px; height: 12px; color: var(--ok); }
  .fail { margin: auto; max-width: 380px; display: flex; flex-direction: column; gap: 12px; align-items: flex-start; padding: 24px; }
  .fail h1 { font: 700 22px/1.1 var(--display); }
  .fail p { color: var(--muted); }
</style>
