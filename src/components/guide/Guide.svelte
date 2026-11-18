<script lang="ts">
  import { app } from '../../lib/store.svelte';
  import { guide, type Chapter } from '../../lib/guide.svelte';
  import Seg from '../Seg.svelte';
  import Spotlight from './Spotlight.svelte';
  import QuickStart from './QuickStart.svelte';
  import OwnApp from './OwnApp.svelte';
  import Reference from './Reference.svelte';
  import Trouble from './Trouble.svelte';
  import { spotFor } from './spots';

  const chapters = [
    { value: '1', label: 'Quick start' },
    { value: '2', label: 'Own app' },
    { value: '3', label: 'Pictures' },
    { value: '4', label: 'Not showing' },
  ];
  const heads: Record<Chapter, { h: string; lead: string }> = {
    1: { h: 'Quick start', lead: 'Three steps. It takes about a minute.' },
    2: { h: 'Your own name and icon', lead: 'Discord shows the name and icon of an application you create. It is free and takes about five minutes.' },
    3: { h: 'Pictures and buttons', lead: 'A short reference. Point at an item to see where it shows up.' },
    4: { h: "If it doesn't show up", lead: 'Go through these from the top. Most problems are in the first few.' },
  };

  // ?hot=timer is for screenshots only: it highlights a reference entry without a pointer
  let hot = $state<string | null>(new URLSearchParams(location.search).get('hot'));
  let first = true;
  let main: HTMLElement | undefined = $state();
  const ch = $derived(guide.chapter);
  const view = $derived(spotFor(ch, ch === 1 || ch === 2 ? guide.step(ch) : 0, ch === 3 ? hot : null));

  $effect(() => {
    guide.chapter;
    if (first) { first = false; return; }
    hot = null;
    main?.scrollTo({ top: 0 });
  });

  function keydown(e: KeyboardEvent) {
    if (app.settingsOpen || e.defaultPrevented) return;
    if (e.key === 'Escape') { e.preventDefault(); guide.close(); return; }
    if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && !e.altKey && !e.ctrlKey && !e.metaKey && !e.shiftKey) {
      if ((e.target as HTMLElement).closest?.('input, textarea, select, [contenteditable], [role="radiogroup"]')) return;
      e.preventDefault();
      guide.go((((ch - 1 + (e.key === 'ArrowRight' ? 1 : -1)) + 4) % 4 + 1) as Chapter);
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="guide">
  <main class="main" bind:this={main} aria-labelledby="gd-h">
    <div class="inner">
      <Seg label="Guide chapters" options={chapters} value={String(ch)} onchange={(v) => guide.go(Number(v) as Chapter)} />
      <header class="head">
        <h2 id="gd-h">{heads[ch].h}</h2>
        <p>{heads[ch].lead}</p>
      </header>
      {#if ch === 1}<QuickStart />
      {:else if ch === 2}<OwnApp />
      {:else if ch === 3}<Reference bind:hot />
      {:else}<Trouble />{/if}
    </div>
  </main>
  <Spotlight spot={view.spot} caption={view.caption} off={view.off} />
</div>

<style>
  .guide { flex: 1; display: grid; grid-template-columns: minmax(0, 1fr) 348px; min-height: 0; }
  .main { overflow-y: auto; padding: 18px 20px 48px; min-width: 0; }
  .inner { max-width: 560px; margin: 0 auto; display: flex; flex-direction: column; gap: 18px; }
  .head { display: flex; flex-direction: column; gap: 6px; margin-top: 6px; }
  h2 { font: 700 26px/1.1 var(--display); letter-spacing: -.02em; text-wrap: balance; }
  .head p { color: var(--muted); font-size: 14px; line-height: 1.5; max-width: 56ch; text-wrap: pretty; }
  @media (max-width: 900px) { .guide { grid-template-columns: minmax(0, 1fr) 300px; } .main { padding-inline: 16px; } }

  /* shared by the steps of chapters one and two */
  .guide :global(.steps) { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .guide :global(.steps > li) { position: relative; padding-bottom: 8px; }
  .guide :global(.steps > li:not(:last-child)::before) {
    content: ""; position: absolute; left: 12px; top: 36px; bottom: 0; border-left: 1px solid var(--line);
  }
  .guide :global(.sb) { margin-left: 38px; padding: 2px 0 8px; display: flex; flex-direction: column; gap: 10px; }
  .guide :global(.sb p) { text-wrap: pretty; max-width: 62ch; }
  .guide :global(.sb > *) { flex-shrink: 0; }
  .guide :global(.acts) { display: flex; gap: 8px; flex-wrap: wrap; }
  .guide :global(.rise) { animation: rise .16s ease-out; }
  @keyframes rise { from { opacity: 0; transform: translateY(4px); } }
</style>
