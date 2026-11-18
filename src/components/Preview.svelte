<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { emptyPresence, type ImageSpec } from '../lib/types';
  import { clean, fmtElapsed, fmtLeft, isUrl, userName } from '../lib/util';
  import Tile from './Tile.svelte';

  const sample = emptyPresence();
  const live = $derived(!!app.draft);
  const p = $derived(app.draft?.presence ?? sample);
  const appId = $derived(p.app_id.trim());
  const entry = $derived(app.entry(appId));
  const assets = $derived(app.assets[appId] ?? []);
  const dim = $derived(!live || !app.showing);

  const gameName = $derived(clean(p.name) ?? (entry?.state === 'ok' ? entry.name : null) ?? (live ? 'Pick a game' : 'Your game'));
  const kindLine = $derived(
    ({ playing: 'Playing', listening: 'Listening to', watching: 'Watching', competing: 'Competing in' } as const)[p.kind]
      + (p.kind === 'playing' ? '' : ` ${gameName}`),
  );
  const prefix = $derived(({ playing: 'Playing', listening: 'Listening to', watching: 'Watching', competing: 'Competing in' } as const)[p.kind]);

  const details = $derived(clean(p.details) ?? (live ? null : 'What you are doing'));
  const stateText = $derived.by(() => {
    const s = clean(p.state) ?? (live ? '' : 'Where you are');
    const party = p.party && p.party.current >= 1 ? `(${p.party.current} of ${p.party.max})` : '';
    return [s, party].filter(Boolean).join(' ');
  });

  const source = (img: ImageSpec | null) => img?.source.trim() ?? '';
  const resolve = (img: ImageSpec | null) => {
    const s = source(img);
    if (!s) return null;
    return isUrl(s) ? s : (assets.find((a) => a.name === s)?.url ?? null);
  };
  const large = $derived(resolve(p.large_image) ?? entry?.icon_url ?? null);
  const small = $derived(resolve(p.small_image));
  const hasSmall = $derived(!!source(p.small_image));
  const bigSeed = $derived(source(p.large_image) || appId || 'decoy');
  const iconUrl = $derived(entry?.state === 'ok' ? entry.icon_url : null);

  // timer: one shared stopwatch in the store
  const timer = $derived.by(() => {
    const t = p.timestamps;
    const now = app.now;
    switch (t.mode) {
      case 'none': return null;
      case 'since_connect': return { text: fmtElapsed(now - app.showSince), clock: false };
      case 'since_update': return { text: fmtElapsed(now - app.changedAt), clock: false };
      case 'local_time': {
        const d = new Date(now); d.setHours(0, 0, 0, 0);
        return { text: fmtElapsed(now - d.getTime()), clock: true };
      }
      case 'custom':
        if (t.end_ms != null) return { text: fmtLeft(t.end_ms - now), clock: true };
        if (t.start_ms != null) return { text: fmtElapsed(now - t.start_ms), clock: false };
        return null;
    }
  });

  const buttons = $derived(p.buttons.filter((b) => b.label.trim()));
  const memberLine = $derived.by(() => {
    const d = clean(p.details); const s = clean(p.state);
    if (p.status_display === 'details' && d) return d;
    if (p.status_display === 'state' && s) return s;
    return `${prefix} ${gameName}`;
  });
  const me = $derived(
    app.engine.status === 'active' || app.engine.status === 'connected'
      ? (userName(app.engine.user) || 'You') : 'You',
  );
</script>

<aside class="side" aria-label="Preview">
  <div>
    <p class="lab">Profile</p>
    <div class="dc" class:off={dim}>
      <div class="dc-card">
        <div class="dc-kind">{kindLine}</div>
        <div class="dc-row">
          <div class="dc-art">
            <Tile url={large} seed={bigSeed} />
            {#if hasSmall}<u><Tile url={small} seed={source(p.small_image)} round /></u>{/if}
          </div>
          <div class="dc-txt">
            <b>{gameName}</b>
            {#if details}<span>{details}</span>{/if}
            {#if stateText}<span>{stateText}</span>{/if}
            {#if timer}
              <span class="dc-time">
                {#if timer.clock}
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm1 5v4.6l3.2 1.9-.9 1.5L11 12.5V7h2Z"/></svg>
                {:else}
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 8h12a4 4 0 0 1 4 4v3a3 3 0 0 1-5.4 1.8L15 15H9l-1.6 1.8A3 3 0 0 1 2 15v-3a4 4 0 0 1 4-4Zm1 2v1.5H5.5v1.5H7V14.5h1.5V13H10v-1.5H8.500V10Zm9.500.500a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z"/></svg>
                {/if}
                <span>{timer.text}</span>
              </span>
            {/if}
          </div>
        </div>
        {#if buttons.length}
          <div class="dc-btns">{#each buttons as b, i (i)}<span>{b.label}</span>{/each}</div>
        {/if}
      </div>
    </div>
  </div>
  <div>
    <p class="lab">Member list</p>
    <div class="dc" class:off={dim}>
      <div class="dc-line"><span class="dc-av"></span><div class="dc-who"><b>{me}</b><small>{memberLine}</small></div></div>
    </div>
  </div>
  <div>
    <p class="lab">Voice channel, icon next to your name</p>
    <div class="dc" class:off={dim}>
      <div class="dc-line"><span class="dc-av sm"></span><div class="dc-who"><b>{me}</b></div><Tile class="voice" url={iconUrl} seed={appId || 'decoy'} /></div>
    </div>
  </div>
  {#if !live}
    <p class="caption">This is how your presence will look. Pick a game to start.</p>
  {:else if !app.showing}
    <p class="caption">Hidden. Switch on "Showing in Discord" to share this.</p>
  {/if}
</aside>

<style>
  .side {
    background: var(--sunken); border-left: 1px solid var(--line); padding: 18px; display: flex; flex-direction: column; gap: 14px;
    overflow-y: auto; min-height: 0;
  }
  .lab { font-size: 12px; color: var(--muted); margin-bottom: 6px; }
  .side .dc { padding: 10px; }
</style>
