<script lang="ts">
  import { app } from '../lib/store.svelte';
  import ProfileMenu from './ProfileMenu.svelte';
  import { userName } from '../lib/util';
  import { guide } from '../lib/guide.svelte';

  const e = $derived(app.engine);
  const who = $derived(e.status === 'active' || e.status === 'connected' ? userName(e.user) : '');
  const tone = $derived(
    e.status === 'active' || e.status === 'connected' ? 'ok'
      : e.status === 'failed' ? 'bad'
        : e.status === 'waiting_for_discord' || e.status === 'connecting' ? 'warn' : '',
  );
  const label = $derived(
    e.status === 'active' || e.status === 'connected' ? (who ? `Connected as ${who}` : 'Connected to Discord')
      : e.status === 'connecting' ? 'Connecting to Discord'
        : e.status === 'waiting_for_discord' ? 'Waiting for Discord'
          : e.status === 'failed' ? 'Could not connect' : 'Not connected',
  );
  // an empty application ID is explained by the form itself and is not counted
  const n = $derived(app.issues.filter((i) => !(i.field === 'app_id' && !app.draft?.presence.app_id.trim())).length);

  function firstIssue() {
    document.querySelector<HTMLElement>('[aria-invalid="true"]')?.scrollIntoView({ block: 'center', behavior: 'smooth' });
    document.querySelector<HTMLElement>('[aria-invalid="true"]')?.focus({ preventScroll: true });
  }
</script>

<header class="top" class:inguide={guide.open}>
  <div class="brand">
    <svg viewBox="64 64 896 896" aria-hidden="true"><rect x="64" y="64" width="896" height="896" rx="208" fill="#1A1D23"/><g transform="translate(160 158) scale(10.500)"><path d="M9 42 4 28l17 7Z" fill="#E7E9ED" stroke="#E7E9ED" stroke-width="2.4" stroke-linejoin="round"/><ellipse cx="29" cy="43" rx="21" ry="12.5" fill="#E7E9ED"/><rect x="33" y="22" width="19" height="20" rx="7" fill="#E7E9ED"/><circle cx="42.500" cy="22" r="10.500" fill="#E7E9ED"/><rect x="50" y="20" width="13" height="7.400" rx="3.700" fill="var(--accent)"/><circle cx="45" cy="19.500" r="2.200" fill="#1A1D23"/><path d="M18 43q10 8 21 0" fill="none" stroke="#1A1D23" stroke-width="3" stroke-linecap="round"/></g></svg>
    Decoy
  </div>
  <ProfileMenu />
  <div class="status {tone}" role="status" title={label}><i></i><span>{label}</span></div>
  {#if n > 0 && app.draft}
    <button type="button" class="fix" onclick={firstIssue}>{n === 1 ? '1 thing to fix' : `${n} things to fix`}</button>
  {/if}
  {#if app.update && !guide.open}
    <button type="button" class="upd" title="Open the download page" onclick={() => app.openExternal(app.update!.url)}>Update to {app.update.version}</button>
  {/if}
  {#if guide.open}
    <button type="button" class="btn back" onclick={() => guide.close()}>
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M6.5 1.500 3 5l3.500 3.500" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
      Back to editor
    </button>
  {:else}
    <button type="button" class="gbtn" onclick={() => guide.show()}>Guide</button>
  {/if}
  <button type="button" class="switch" class:ready={!app.showing && !!app.draft} role="switch" aria-checked={app.showing} disabled={!app.draft}
    title={app.showing ? 'Click to stop showing it' : 'Click to show it on your Discord profile'}
    onclick={() => app.setShowing(!app.showing)}>
    <i></i><span>{app.showing ? 'Showing in Discord' : 'Show in Discord'}</span>
  </button>
  <button type="button" class="icon-btn" aria-label="Settings" title="Settings" onclick={() => (app.settingsOpen = true)}>
    <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M8.2 2.5h3.6l.5 2.1c.5.2 1 .5 1.4.8l2-.7 1.8 3.1-1.6 1.4c.1.5.1 1.1 0 1.6l1.6 1.4-1.8 3.1-2-.7c-.4.3-.9.6-1.4.8l-.5 2.1H8.2l-.5-2.1c-.5-.2-1-.5-1.4-.8l-2 .7-1.8-3.1 1.6-1.4a5 5 0 0 1 0-1.6L2.5 7.8l1.8-3.1 2 .7c.4-.3.9-.6 1.4-.8l.5-2.1Z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><circle cx="10" cy="10" r="2.4" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>
  </button>
</header>

<style>
  .top { display: flex; align-items: center; gap: 16px; padding: 12px 18px; border-bottom: 1px solid var(--line); flex: none; position: relative; z-index: 30; }
  .top :global(.status) { margin-left: auto; }
  .gbtn { background: none; border: 0; padding: 5px 4px; font-weight: 500; color: var(--muted); white-space: nowrap; border-radius: 6px; }
  .gbtn:hover { color: var(--ink); }
  .upd { background: none; border: 1px solid var(--line); border-radius: 999px; padding: 4px 11px; font-weight: 600; color: var(--ink); white-space: nowrap; }
  .upd:hover { border-color: var(--accent); }
  .back { padding: 5px 12px 5px 9px; white-space: nowrap; }
  .back svg { width: 10px; height: 10px; }
  .switch:disabled { opacity: .5; cursor: default; }
  .switch.ready { border-color: var(--accent); box-shadow: 0 0 0 2px color-mix(in oklab, var(--accent) 45%, transparent); }
  .fix { background: none; border: 0; padding: 2px 0; color: var(--bad); font-weight: 600; white-space: nowrap; text-decoration: underline; text-decoration-style: dotted; text-underline-offset: 3px; }
  .fix:hover { text-decoration-style: solid; }
  @media (max-width: 860px) { .top.inguide :global(.status span) { display: none; } }
  @media (max-width: 860px) { .top { gap: 12px; padding-inline: 14px; } }
</style>
