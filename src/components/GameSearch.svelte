<script lang="ts">
  import { backend } from '../lib/backend';
  import { app } from '../lib/store.svelte';
  import type { GameHit } from '../lib/types';
  import { fmtCount } from '../lib/util';
  import Tile from './Tile.svelte';

  let { onpick, onown, hero = false, autofocus = false }: {
    onpick: (g: GameHit) => void; onown?: () => void; hero?: boolean; autofocus?: boolean;
  } = $props();

  const uid = $props.id();
  let q = $state('');
  let hits = $state<GameHit[]>([]);
  let busy = $state(false);
  let open = $state(false);
  let active = $state(0);
  let searched = $state('');
  let input: HTMLInputElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const gamesReady = $derived(app.games.count > 0 && !app.games.loading);
  const term = $derived(q.trim());
  const showList = $derived(hero ? term.length > 0 && gamesReady : open);

  $effect(() => { if (autofocus) input?.focus(); });

  function run() {
    clearTimeout(timer);
    const s = q.trim();
    if (!s || !gamesReady) { hits = []; busy = false; searched = ''; return; }
    busy = true;
    timer = setTimeout(async () => {
      const my = ++seq;
      try {
        const r = await backend.searchGames(s, 8);
        if (my !== seq) return;
        hits = r; searched = s; active = 0;
      } catch { if (my === seq) { hits = []; searched = s; } }
      if (my === seq) busy = false;
    }, 120);
  }

  function pick(g: GameHit) {
    open = false; q = ''; hits = [];
    onpick(g);
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') { e.preventDefault(); open = true; if (hits.length) active = (active + 1) % hits.length; }
    else if (e.key === 'ArrowUp') { e.preventDefault(); if (hits.length) active = (active - 1 + hits.length) % hits.length; }
    else if (e.key === 'Enter') { if (showList && hits[active]) { e.preventDefault(); pick(hits[active]); } }
    else if (e.key === 'Escape') { if (open && !hero) { open = false; e.stopPropagation(); } }
  }

  function parts(name: string): [string, string, string] {
    const i = name.toLowerCase().indexOf(term.toLowerCase());
    if (i < 0 || !term) return [name, '', ''];
    return [name.slice(0, i), name.slice(i, i + term.length), name.slice(i + term.length)];
  }

  function focusout(e: FocusEvent) {
    const next = e.relatedTarget as Node | null;
    if (!next || !(e.currentTarget as HTMLElement).contains(next)) open = false;
  }
</script>

<div class="gs" class:hero onfocusout={focusout}>
  <div class="box">
    <svg viewBox="0 0 20 20" aria-hidden="true"><circle cx="8.5" cy="8.5" r="5.5" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="m13 13 4 4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/></svg>
    <input bind:this={input} type="text" role="combobox" aria-label="Search Discord's game list" aria-expanded={showList}
      aria-controls="{uid}-list" aria-autocomplete="list" aria-activedescendant={showList && hits[active] ? `${uid}-o${active}` : undefined}
      placeholder={hero ? 'Search for a game, for example Minecraft' : "Search Discord's game list"} autocomplete="off" spellcheck="false"
      bind:value={q} oninput={() => { open = true; run(); }} onfocus={() => (open = true)} onkeydown={key} />
    {#if busy}<span class="spinner" aria-hidden="true"></span>{/if}
  </div>

  {#if app.games.loading}
    <div class="note" class:float={!hero} hidden={!hero && !open}>
      <div class="bar" role="progressbar" aria-label="Downloading the game list"></div>
      <p><b>Downloading Discord's game list.</b> This takes a moment and only happens once. Search works as soon as it finishes.</p>
    </div>
  {:else if app.games.count === 0}
    <div class="note" class:float={!hero} hidden={!hero && !open}>
      <p><b>The game list is not available yet.</b> {app.games.error ?? 'Decoy could not load it.'} Check your connection, then retry.</p>
      <button type="button" class="btn" onclick={() => app.refreshGames()}>Retry download</button>
    </div>
  {:else if showList}
    <ul id="{uid}-list" role="listbox" aria-label="Games" class:float={!hero}>
      {#each hits as g, i (g.id)}
        {@const [a, m, z] = parts(g.name)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li id="{uid}-o{i}" role="option" aria-selected={i === active} class:on={i === active}
          onmousedown={(e) => e.preventDefault()} onclick={() => pick(g)} onmousemove={() => (active = i)}>
          <Tile url={g.icon_url} seed={g.id} size={28} />
          <span class="nm">{a}<strong>{m}</strong>{z}</span>
        </li>
      {/each}
      {#if !hits.length && !busy && searched === term}
        <li class="none" role="presentation">
          <p>No game called "{term}" in the list.</p>
          <p class="caption">Check the spelling, or {#if onown}<button type="button" class="link" onclick={onown}>use your own application ID</button>{:else}use your own application ID{/if}.</p>
        </li>
      {/if}
    </ul>
  {:else if hero}
    <p class="caption hint">{fmtCount(app.games.count)} games from Discord's own list. Anything Discord can detect works.</p>
  {/if}
</div>

<style>
  .gs { position: relative; display: flex; flex-direction: column; gap: 8px; min-width: 0; }
  .box { position: relative; display: flex; align-items: center; }
  .box svg { position: absolute; left: 10px; width: 16px; height: 16px; color: var(--muted); pointer-events: none; }
  .box input { padding-left: 33px; padding-right: 32px; }
  .box .spinner { position: absolute; right: 11px; }
  .hero .box input { font-size: 16px; padding: 13px 40px 13px 42px; border-radius: 12px; background: var(--sunken); border: 1px solid var(--line); }
  .hero .box input:focus { border-color: var(--ink); }
  .hero .box svg { left: 14px; width: 19px; height: 19px; }
  .hero .box .spinner { right: 15px; }
  ul { list-style: none; margin: 0; padding: 4px; display: flex; flex-direction: column; gap: 1px; }
  ul.float, .note.float {
    position: absolute; z-index: 20; left: 0; right: 0; top: calc(100% + 4px); background: var(--surface);
    border: 1px solid var(--line); border-radius: 10px; box-shadow: var(--pop-shadow);
  }
  .hero ul { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  li { display: flex; align-items: center; gap: 10px; padding: 6px 8px; border-radius: 7px; cursor: pointer; min-width: 0; }
  li.on { background: var(--sunken); }
  li .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  li .nm strong { font-weight: 700; }
  li.none { cursor: default; flex-direction: column; align-items: flex-start; gap: 3px; padding: 10px; }
  .note { padding: 12px; display: flex; flex-direction: column; gap: 10px; align-items: flex-start; background: var(--sunken); border-radius: 10px; }
  .note[hidden] { display: none; }
  .note.float { background: var(--surface); }
  .note .bar { width: 100%; }
  .note p { color: var(--muted); }
  .note p b { color: var(--ink); font-weight: 600; }
  .hint { padding-left: 4px; }
</style>
