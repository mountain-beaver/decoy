<script lang="ts">
  import { app } from '../lib/store.svelte';
  const e = $derived(app.engine);
</script>

{#if e.status === 'waiting_for_discord' && app.showing}
  <div class="band warn" role="status">
    <span class="spinner" aria-hidden="true"></span>
    <p><b>Waiting for Discord.</b> Start it and Decoy connects by itself.</p>
  </div>
{:else if e.status === 'failed'}
  <div class="band bad" role="alert">
    <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6.5" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M8 4.5v4M8 10.6v.9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
    <p><b>Discord refused the presence.</b> {e.message}</p>
    <button type="button" class="btn" onclick={() => app.setShowing(true)}>Try again</button>
  </div>
{/if}

<style>
  .band { display: flex; align-items: center; gap: 10px; padding: 9px 18px; border-bottom: 1px solid var(--line); flex: none; }
  .band p { flex: 1; min-width: 0; text-wrap: pretty; }
  .band b { font-weight: 600; }
  .band.warn { background: color-mix(in oklab, var(--warn) 13%, var(--surface)); }
  .band.bad { background: color-mix(in oklab, var(--bad) 11%, var(--surface)); }
  .band.bad > svg { width: 16px; height: 16px; color: var(--bad); flex: none; }
  .band .btn { padding: 4px 11px; }
</style>
