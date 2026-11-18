<script lang="ts">
  import { app } from '../../lib/store.svelte';
  import { guide } from '../../lib/guide.svelte';
  import { userName } from '../../lib/util';
  import Tile from '../Tile.svelte';

  let { spot, caption, off = false }: { spot: string[]; caption: string; off?: boolean } = $props();

  // "all": everything is visible, no rings; otherwise unmentioned parts dim and mentioned ones get a ring
  const focus = $derived(!off && spot.length > 0 && !spot.includes('all'));
  const on = (k: string) => (focus && spot.includes(k) ? '' : undefined);

  const name = $derived(guide.preview?.name ?? 'Skyline Racer');
  const icon = $derived(guide.preview?.icon_url ?? null);
  const seed = $derived(guide.preview ? guide.preview.name : 'skyline');
  const me = $derived(
    app.engine.status === 'active' || app.engine.status === 'connected' ? (userName(app.engine.user) || 'You') : 'You',
  );
</script>

<aside class="side" aria-label="Where it shows up">
  <div class="stage" data-focus={focus ? '' : undefined} data-off={off ? '' : undefined}>
    <div class="part">
      <p class="lab">Profile</p>
      <div class="dc">
        <div class="dc-card">
          <div class="sp kind" data-on={on('kind')}><span>Playing</span></div>
          <div class="dc-row">
            <div class="art">
              <div class="sp big" data-on={on('big')}><Tile url={icon} {seed} size={64} /></div>
              <div class="sp badge" data-on={on('badge')}><div class="bd"><Tile seed="star" size={18} round /></div></div>
            </div>
            <div class="txt">
              <div class="sp" data-on={on('name')}><b>{name}</b></div>
              <div class="sp" data-on={on('details')}><span>Racing, lap 3</span></div>
              <div class="sp" data-on={on('state')}><span>In a lobby (2 of 4)</span></div>
              <div class="sp" data-on={on('timer')}>
                <span class="tm">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 8h12a4 4 0 0 1 4 4v3a3 3 0 0 1-5.4 1.8L15 15H9l-1.6 1.8A3 3 0 0 1 2 15v-3a4 4 0 0 1 4-4Zm1 2v1.5H5.5v1.5H7V14.5h1.5V13H10v-1.5H8.500V10Zm9.500.500a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z"/></svg>
                  12:04
                </span>
              </div>
            </div>
          </div>
          <div class="sp" data-on={on('buttons')}><div class="dc-btns"><span>Watch trailer</span><span>Join my crew</span></div></div>
        </div>
      </div>
    </div>

    <div class="part">
      <p class="lab">Member list</p>
      <div class="dc">
        <div class="dc-line">
          <span class="dc-av"></span>
          <div class="dc-who">
            <b>{me}</b>
            <div class="mline">
              <span class="sp" data-on={on('member')}><small>Playing {name}</small></span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="part">
      <p class="lab">Voice channel, icon next to your name</p>
      <div class="dc">
        <div class="dc-line">
          <span class="dc-av sm"></span>
          <div class="dc-who"><b>{me}</b></div>
          <span class="sp vi" data-on={on('voiceicon')}><Tile url={icon} {seed} size={20} /></span>
        </div>
      </div>
    </div>
  </div>

  <p class="cap" aria-live="polite"><i aria-hidden="true"></i><span>{caption}</span></p>
</aside>

<style>
  .side {
    background: var(--sunken); border-left: 1px solid var(--line); padding: 18px; display: flex; flex-direction: column; gap: 14px;
    overflow-y: auto; min-height: 0;
  }
  .stage { display: flex; flex-direction: column; gap: 14px; }
  .lab { font-size: 12px; color: var(--muted); margin-bottom: 6px; }
  .side .dc { padding: 10px; }

  /* the ring is drawn on its own layer: only opacity and transform change */
  .sp { position: relative; transition: opacity .15s; min-width: 0; }
  .sp::after {
    content: ""; position: absolute; inset: -3px -6px; border: 2px solid var(--accent); border-radius: 8px;
    opacity: 0; transform: scale(.94); transition: opacity .15s, transform .15s; pointer-events: none;
  }
  .sp[data-on]::after { opacity: 1; transform: none; }
  .stage[data-focus] .sp:not([data-on]) { opacity: .28; }
  .stage[data-off] .sp, .stage[data-off] .dc-av { opacity: .25; }

  .kind { font-size: 12px; font-weight: 600; color: var(--dc-muted); align-self: flex-start; }
  .art { position: relative; flex: none; }
  .art :global(.tile) { border-radius: 8px; }
  .big { width: 64px; height: 64px; }
  .big::after { inset: -4px; border-radius: 12px; }
  .badge { position: absolute; right: -6px; bottom: -6px; width: 24px; height: 24px; }
  .badge::after { inset: -3px; border-radius: 50%; }
  .bd { width: 24px; height: 24px; border-radius: 50%; border: 3px solid var(--dc-card); overflow: hidden; background: #F5C451; }
  .bd :global(.tile) { width: 100% !important; height: 100% !important; }
  .txt { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; align-items: flex-start; }
  .txt b { font-weight: 600; font-size: 14px; display: block; }
  .txt .sp { max-width: 100%; }
  .tm { color: var(--dc-ok); font-weight: 600; font-variant-numeric: tabular-nums; display: flex; align-items: center; gap: 5px; }
  .tm svg { width: 14px; height: 14px; fill: var(--dc-ok); flex: none; }
  .mline { display: flex; align-items: center; gap: 6px; }
  .mline .sp { display: flex; }
  .vi::after { inset: -4px; border-radius: 7px; }
  .vi { margin-left: auto; display: flex; }
  .vi :global(.tile) { border-radius: 4px; }

  .cap { display: flex; gap: 9px; align-items: flex-start; font-size: 13px; line-height: 1.4; text-wrap: pretty; }
  .cap i { width: 11px; height: 11px; border-radius: 3px; background: var(--accent); flex: none; margin-top: 3px; }
  @media (max-width: 900px) { .side { padding: 14px; } }
  @media (max-height: 640px) {
    .side { padding-block: 10px; gap: 8px; }
    .stage { gap: 8px; }
    .lab { margin-bottom: 3px; }
    .side .dc { padding: 8px; }
    .side :global(.dc-card) { gap: 7px; padding: 10px; }
    .side :global(.dc-line) { padding-block: 5px; }
    .side :global(.dc-btns span) { padding-block: 5px; }
  }
</style>
