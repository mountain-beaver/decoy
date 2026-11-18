<script lang="ts">
  import { app } from '../lib/store.svelte';
  import type { ImageSpec } from '../lib/types';
  import { clean, isUrl } from '../lib/util';
  import Tile from './Tile.svelte';
  import TextField from './TextField.svelte';
  import Disclosure from './Disclosure.svelte';
  import Errs from './Errs.svelte';

  let { which }: { which: 'large' | 'small' } = $props();
  const prop = $derived<'large_image' | 'small_image'>(which === 'large' ? 'large_image' : 'small_image');
  const title = $derived(which === 'large' ? 'Large image' : 'Small image');
  const uid = $props.id();

  const img = $derived(app.p[prop]);
  const appId = $derived(app.p.app_id.trim());
  const assets = $derived(app.assets[appId] ?? []);
  const entry = $derived(app.entry(appId));
  // svelte-ignore state_referenced_locally
  let more = $state(!!(app.p[which === 'large' ? 'large_image' : 'small_image']?.url));
  let picking = $state(false);

  const src = $derived(img?.source.trim() ?? '');
  const thumb = $derived(isUrl(src) ? src : (assets.find((a) => a.name === src)?.url ?? null));

  function set(field: keyof ImageSpec, v: string) {
    const cur: ImageSpec = app.p[prop] ?? { source: '', text: null, url: null };
    if (field === 'source') cur.source = v; else cur[field] = v || null;
    app.p[prop] = !cur.source.trim() && !clean(cur.text) && !clean(cur.url) ? null : cur;
  }

  function focusout(e: FocusEvent) {
    const next = e.relatedTarget as Node | null;
    if (!next || !(e.currentTarget as HTMLElement).contains(next)) picking = false;
  }
  const bad = $derived(app.issuesFor(prop).length > 0);
</script>

<div class="stack img">
  <div class="field">
    <label for="{uid}-src">{title}</label>
    <div class="src" onfocusout={focusout} role="presentation">
      <Tile url={thumb} seed={src || which} size={33} />
      <input id="{uid}-src" type="text" value={img?.source ?? ''} aria-invalid={bad || undefined}
        placeholder={assets.length ? 'Pick an asset or paste a link' : 'Asset name or https:// link'}
        autocomplete="off" spellcheck="false" oninput={(e) => set('source', e.currentTarget.value)} />
      {#if assets.length}
        <button type="button" class="btn" aria-haspopup="listbox" aria-expanded={picking} onclick={() => (picking = !picking)}>Choose</button>
        {#if picking}
          <ul class="pop" role="listbox" aria-label="{title} assets">
            {#each assets as a (a.id)}
              <li role="option" aria-selected={src === a.name}>
                <button type="button" onclick={() => { set('source', a.name); picking = false; }}>
                  <Tile url={a.url} seed={a.name} size={32} />
                  <span>{a.name}</span>
                  {#if src === a.name}<svg viewBox="0 0 16 16" aria-hidden="true"><path d="m3.5 8.5 3 3 6-7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
    <Errs keys={[prop]} />
    {#if !src}
      <p class="caption">
        {#if assets.length}Assets uploaded to {entry?.name ?? 'your application'}, or a link.
        {:else if entry?.origin === 'list'}Games from Discord's list have no assets of their own, so paste an image link.
        {:else}An asset name from your application, or an image link.{/if}
      </p>
    {/if}
  </div>
  <TextField label="Hover text" value={img?.text} k="{prop}_text" max={128} oninput={(v) => set('text', v)} />
  <Disclosure bind:open={more}>
    <TextField label="Link opened on click" type="url" value={img?.url} k="{prop}_url" placeholder="https://" oninput={(v) => set('url', v)} />
  </Disclosure>
</div>

<style>
  .src { position: relative; display: flex; gap: 8px; align-items: center; }
  .pop {
    position: absolute; z-index: 20; left: 0; right: 0; top: calc(100% + 4px); list-style: none; margin: 0; padding: 4px;
    background: var(--surface); border: 1px solid var(--line); border-radius: 10px; box-shadow: var(--pop-shadow);
    max-height: 220px; overflow: auto;
  }
  .pop button { all: unset; box-sizing: border-box; width: 100%; display: flex; align-items: center; gap: 10px; padding: 5px 8px; border-radius: 7px; cursor: pointer; }
  .pop button:hover { background: var(--sunken); }
  .pop button:focus-visible { outline: 2px solid var(--ink); outline-offset: -2px; }
  .pop span { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pop svg { width: 14px; height: 14px; color: var(--ink); }
  @container (max-width: 520px) { .img { gap: 10px; } }
</style>
