<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { guide } from '../lib/guide.svelte';

  const e = $derived(app.engine);
  const KEY = 'decoy.liveTipDone';
  const read = () => { try { return localStorage.getItem(KEY) === '1'; } catch { return false; } };
  let tipDone = $state(read());
  function closeTip() {
    tipDone = true;
    try { localStorage.setItem(KEY, '1'); } catch { /* storage is unavailable, the tip just comes back */ }
  }
</script>

{#if app.draft && !app.showing}
  <div class="band off" role="status">
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M6.5 5.4v5.2L10.6 8Z" fill="currentColor"/></svg>
    <p><b>Not on your Discord profile yet.</b> Decoy shares nothing until you start it.</p>
    <button type="button" class="btn primary" onclick={() => app.setShowing(true)}>Show in Discord</button>
  </div>
{:else if e.status === 'waiting_for_discord' && app.showing}
  <div class="band warn" role="status">
    <span class="spinner" aria-hidden="true"></span>
    <p><b>Waiting for Discord.</b> Start the Discord desktop app on this computer and Decoy connects by itself.</p>
  </div>
{:else if e.status === 'failed'}
  <div class="band bad" role="alert">
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M8 4.5v4M8 10.6v.9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
    <p><b>Discord refused the presence.</b> {e.message}</p>
    <button type="button" class="btn" onclick={() => app.setShowing(true)}>Try again</button>
  </div>
{:else if e.status === 'active' && app.showing && !tipDone}
  <div class="band ok" role="status">
    <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"/></svg>
    <p><b>Live on your Discord profile.</b> Friends can see it now. Not there? <button type="button" class="link inline" onclick={() => guide.show(4)}>Check these points</button></p>
    <button type="button" class="icon-btn x" aria-label="Close" title="Close" onclick={closeTip}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4.5 4.5 7 7m0-7-7 7" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>
    </button>
  </div>
{/if}

<style>
  .band { display: flex; align-items: center; gap: 10px; padding: 9px 18px; border-bottom: 1px solid var(--line); flex: none; }
  .band p { flex: 1; min-width: 0; text-wrap: pretty; }
  .band b { font-weight: 600; }
  .band > svg { width: 16px; height: 16px; flex: none; }
  .band.off { background: color-mix(in oklab, var(--accent) 22%, var(--surface)); }
  .band.warn { background: color-mix(in oklab, var(--warn) 13%, var(--surface)); }
  .band.bad { background: color-mix(in oklab, var(--bad) 11%, var(--surface)); }
  .band.bad > svg { color: var(--bad); }
  .band.ok { background: color-mix(in oklab, var(--ok) 10%, var(--surface)); }
  .band.ok > svg { color: var(--ok); }
  .band .btn { padding: 5px 13px; }
  .band .link.inline { display: inline; align-self: auto; color: inherit; }
  .band .x { width: 26px; height: 26px; }
  .band .x svg { width: 12px; height: 12px; }
</style>
