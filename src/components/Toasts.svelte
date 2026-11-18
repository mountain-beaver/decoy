<script lang="ts">
  import { app } from '../lib/store.svelte';
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast" class:bad={t.kind === 'error'} role={t.kind === 'error' ? 'alert' : 'status'}>
      <i></i><span>{t.text}</span>
      <button type="button" aria-label="Dismiss" onclick={() => app.dismiss(t.id)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts { position: fixed; z-index: 100; left: 50%; bottom: 16px; transform: translateX(-50%); display: flex; flex-direction: column; gap: 8px; align-items: center; width: min(520px, calc(100vw - 32px)); pointer-events: none; }
  .toast {
    pointer-events: auto; display: flex; align-items: flex-start; gap: 10px; background: var(--ink); color: var(--surface);
    border-radius: 10px; padding: 9px 8px 9px 14px; box-shadow: var(--pop-shadow); animation: up .18s ease-out;
    max-width: 100%;
  }
  .toast i { width: 8px; height: 8px; border-radius: 50%; background: var(--ok); margin-top: 6px; flex: none; }
  .toast.bad i { background: var(--bad); }
  .toast span { flex: 1; min-width: 0; overflow-wrap: anywhere; padding-top: 1px; }
  .toast button { all: unset; cursor: pointer; width: 22px; height: 22px; display: flex; align-items: center; justify-content: center; border-radius: 6px; opacity: .7; flex: none; }
  .toast button:hover { opacity: 1; background: color-mix(in oklab, var(--surface) 18%, transparent); }
  .toast button:focus-visible { outline: 2px solid var(--surface); }
  .toast svg { width: 12px; height: 12px; }
  @keyframes up { from { opacity: 0; transform: translateY(6px); } }
</style>
