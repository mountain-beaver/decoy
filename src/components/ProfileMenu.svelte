<script lang="ts">
  import { app } from '../lib/store.svelte';
  import Tile from './Tile.svelte';

  type View = 'list' | 'new' | 'rename' | 'delete';
  let open = $state(false);
  let view = $state<View>('list');
  let name = $state('');
  let root: HTMLDivElement | undefined = $state();
  let trigger: HTMLButtonElement | undefined = $state();
  let fileInput: HTMLInputElement | undefined = $state();
  const uid = $props.id();

  const list = $derived(app.profiles.map((p) => (app.draft && p.id === app.draft.id ? app.draft : p)));
  const count = $derived(list.length);
  const has = $derived(!!app.draft);
  const iconOf = (appId: string) => {
    const e = app.entry(appId);
    return e?.state === 'ok' ? e.icon_url : null;
  };

  function show() {
    open = true; view = 'list';
    for (const p of app.profiles) app.ensureApp(p.presence.app_id);
    queueMicrotask(() => root?.querySelector<HTMLElement>('[role=menuitemradio][aria-checked=true], [role=menuitem]')?.focus());
  }
  function hide(refocus = true) {
    open = false; view = 'list';
    if (refocus) trigger?.focus();
  }

  function onwin(e: Event) {
    if (open && root && !root.contains(e.target as Node)) hide(false);
  }

  function items(): HTMLElement[] {
    return [...(root?.querySelectorAll<HTMLElement>('[role=menuitem],[role=menuitemradio]') ?? [])];
  }
  function key(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); if (view === 'list') hide(); else { view = 'list'; queueMicrotask(() => items()[0]?.focus()); } return; }
    if (view !== 'list') return;
    const els = items();
    const i = els.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown') { e.preventDefault(); els[(i + 1) % els.length]?.focus(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); els[(i - 1 + els.length) % els.length]?.focus(); }
    else if (e.key === 'Home') { e.preventDefault(); els[0]?.focus(); }
    else if (e.key === 'End') { e.preventDefault(); els[els.length - 1]?.focus(); }
    else if (e.key === 'Tab') hide(false);
  }

  function go(v: View) {
    view = v;
    name = v === 'rename' ? (app.draft?.name ?? '') : v === 'new' ? 'New profile' : '';
    queueMicrotask(() => {
      const el = root?.querySelector<HTMLInputElement>('input.nm');
      if (el) { el.focus(); el.select(); } else root?.querySelector<HTMLElement>('[data-first]')?.focus();
    });
  }

  async function submit(e: Event) {
    e.preventDefault();
    const n = name.trim();
    if (!n) return;
    if (view === 'new') { hide(); await app.createProfile(n); }
    else { app.renameActive(n); hide(); }
  }

  async function act(fn: () => Promise<void> | void) {
    hide();
    await fn();
  }

  function picked(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    (e.currentTarget as HTMLInputElement).value = '';
    if (f) void app.importFile(f);
  }
</script>

<svelte:window onpointerdown={onwin} />

<div class="pm" bind:this={root} onkeydown={key} role="presentation">
  <button bind:this={trigger} type="button" class="pick trig" aria-haspopup="menu" aria-expanded={open} aria-controls="{uid}-m"
    onclick={() => (open ? hide(false) : show())}>
    {#if app.draft}
      <Tile url={iconOf(app.draft.presence.app_id)} seed={app.draft.presence.app_id || app.draft.id} size={18} />
      <b>{app.draft.name}</b>
    {:else}
      <b class="none">No profile</b>
    {/if}
    {#if count > 0}<small>{count} {count === 1 ? 'profile' : 'profiles'}</small>{/if}
    <svg class="chev" viewBox="0 0 10 10" aria-hidden="true"><path d="m2 3.5 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>
  </button>

  {#if open}
    <div id="{uid}-m" class="menu" role="menu" aria-label="Profiles">
      {#if view === 'list'}
        <p class="sec">Profiles</p>
        <div class="plist">
          {#each list as p (p.id)}
            <button type="button" role="menuitemradio" aria-checked={p.id === app.draft?.id} class="it" onclick={() => act(() => app.switchProfile(p.id))}>
              <Tile url={iconOf(p.presence.app_id)} seed={p.presence.app_id || p.id} size={22} />
              <span class="t">{p.name}</span>
              {#if p.id === app.draft?.id}<svg class="ck" viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>{/if}
            </button>
          {:else}
            <p class="empty">No profiles yet.</p>
          {/each}
        </div>
        <hr />
        <button type="button" role="menuitem" class="it" onclick={() => go('new')}><span class="t">New profile</span></button>
        <button type="button" role="menuitem" class="it" disabled={!has} onclick={() => act(() => app.duplicateActive())}><span class="t">Duplicate</span></button>
        <button type="button" role="menuitem" class="it" disabled={!has} onclick={() => go('rename')}><span class="t">Rename</span></button>
        <button type="button" role="menuitem" class="it" disabled={!has} onclick={() => act(() => app.exportActive())}><span class="t">Export to file</span></button>
        <button type="button" role="menuitem" class="it" onclick={() => { hide(false); fileInput?.click(); }}><span class="t">Import from file</span></button>
        <hr />
        <button type="button" role="menuitem" class="it bad" disabled={!has} onclick={() => go('delete')}><span class="t">Delete profile</span></button>
      {:else if view === 'new' || view === 'rename'}
        <form class="step" onsubmit={submit}>
          <label for="{uid}-n" class="sec" style="padding:0">{view === 'new' ? 'Name the new profile' : 'Rename profile'}</label>
          <input id="{uid}-n" class="nm" type="text" bind:value={name} maxlength="60" autocomplete="off" spellcheck="false" />
          <div class="acts">
            <button type="button" class="btn" onclick={() => { view = 'list'; }}>Cancel</button>
            <button type="submit" class="btn primary" disabled={!name.trim()}>{view === 'new' ? 'Create profile' : 'Rename'}</button>
          </div>
        </form>
      {:else}
        <div class="step">
          <p class="q">Delete "{app.draft?.name}"?</p>
          <p class="caption">This can't be undone. Export it first if you may want it back.</p>
          <div class="acts">
            <button type="button" class="btn" data-first onclick={() => { view = 'list'; }}>Keep it</button>
            <button type="button" class="btn danger" onclick={() => act(() => app.deleteActive())}>Delete profile</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
  <input bind:this={fileInput} type="file" accept=".json,application/json" class="sr" tabindex="-1" aria-label="Profile file" onchange={picked} />
</div>

<style>
  .pm { position: relative; }
  .trig { border: 0; min-width: 0; max-width: 300px; font-weight: normal; }
  .trig:hover { background: color-mix(in oklab, var(--sunken) 80%, var(--line)); }
  .trig b { overflow: hidden; text-overflow: ellipsis; }
  .trig .none { color: var(--muted); font-weight: 500; }
  .chev { width: 10px; height: 10px; color: var(--muted); flex: none; }
  .menu {
    position: absolute; z-index: 40; top: calc(100% + 6px); left: 0; width: 264px; background: var(--surface);
    border: 1px solid var(--line); border-radius: 12px; box-shadow: var(--pop-shadow); padding: 6px;
    display: flex; flex-direction: column; gap: 1px;
  }
  .sec { font-size: 12px; color: var(--muted); padding: 4px 8px 4px; font-weight: 500; }
  .plist { max-height: 188px; overflow-y: auto; display: flex; flex-direction: column; gap: 1px; }
  hr { border: 0; border-top: 1px solid var(--line); margin: 5px 2px; }
  .it {
    all: unset; box-sizing: border-box; display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 7px; cursor: pointer; min-width: 0;
  }
  .it:hover, .it:focus-visible { background: var(--sunken); }
  .it:focus-visible { outline: 2px solid var(--ink); outline-offset: -2px; }
  .it[aria-checked="true"] .t { font-weight: 600; }
  .it:disabled { opacity: .4; cursor: default; pointer-events: none; }
  .it.bad { color: var(--bad); }
  .t { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .ck { width: 14px; height: 14px; flex: none; }
  .empty { color: var(--muted); padding: 6px 8px; }
  .step { display: flex; flex-direction: column; gap: 8px; padding: 6px; }
  .q { font: 700 14px/1.25 var(--display); }
  .acts { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }
  .step input { border: 1px solid var(--line); }
</style>
