<script lang="ts">
  // Sketches of Developer Portal pages: HTML and CSS only, no screenshots and no third-party marks.
  let { kind }: { kind: 'create' | 'icon' | 'assets' | 'id' } = $props();
  const notes = {
    create: 'A sketch of the page. Press the highlighted buttons.',
    icon: 'A sketch of the page. Upload your picture in the icon box.',
    assets: 'A sketch of the page. Open the highlighted item in the sidebar.',
    id: 'A sketch of the page. Press the highlighted button.',
  } as const;
</script>

<figure class="sk">
  <div class="frame" aria-hidden="true">
    {#if kind === 'create'}
      <div class="pane solo">
        <div class="tb"><b>Applications</b><span class="pb hot">New Application</span></div>
        <div class="dlg">
          <div class="f"><small>Name</small><div class="inp">Skyline Racer</div></div>
          <div class="foot"><span class="pb">Cancel</span><span class="pb hot">Create</span></div>
        </div>
      </div>
    {:else}
      <div class="side">
        {#if kind === 'assets'}
          <span class="it">General Information</span>
          <span class="it grp">Rich Presence</span>
          <span class="it sub hot">Art Assets</span>
          <span class="it">Installation</span>
        {:else}
          <span class="it hot">General Information</span>
          <span class="it">Rich Presence</span>
          <span class="it">Installation</span>
        {/if}
      </div>
      <div class="pane">
        {#if kind === 'icon'}
          <div class="row">
            <div class="f"><small>Icon</small><div class="up"><i></i></div><small class="dim">1024 x 1024</small></div>
            <div class="f grow"><small>Name</small><div class="inp">Skyline Racer</div></div>
          </div>
        {:else if kind === 'assets'}
          <b class="h">Art Assets</b>
          <div class="grid">
            {#each [['cover', 330], ['star', 48], ['poster', 190], ['logo', 270]] as [k, h] (k)}
              <div class="cell"><i class="tile" style="--h:{h}"></i><small>{k}</small></div>
            {/each}
            <div class="cell"><i class="add"></i><small class="dim">Add Image</small></div>
          </div>
        {:else}
          <div class="f"><small>Name</small><div class="inp">Skyline Racer</div></div>
          <div class="f">
            <small>Application ID</small>
            <div class="idrow"><span class="inp mono">1445160676522856458</span><span class="pb hot">Copy</span></div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
  <figcaption class="caption">{notes[kind]}</figcaption>
</figure>

<style>
  .sk, .frame, .pane > * { flex-shrink: 0; }
  .sk { margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .frame { display: flex; gap: 8px; background: var(--sunken); border: 1px solid var(--line); border-radius: 10px; padding: 8px; min-height: 132px; }
  .pane { flex: 1; min-width: 0; background: var(--surface); border-radius: 7px; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
  .pane.solo { gap: 12px; }
  .side { width: 142px; flex: none; display: flex; flex-direction: column; gap: 2px; padding: 4px 0; }
  .it { padding: 5px 8px; border-radius: 6px; font-size: 12px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .it.grp { color: var(--ink); font-weight: 600; }
  .it.sub { margin-left: 10px; }
  .hot { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .tb { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .tb b, .h { font-size: 12px; font-weight: 600; }
  .pb { padding: 4px 10px; border-radius: 6px; border: 1px solid var(--line); font-size: 12px; color: var(--muted); white-space: nowrap; background: var(--surface); }
  .pb.hot { border-color: transparent; background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .dlg { border: 1px solid var(--line); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 10px; background: var(--sunken); }
  .foot { display: flex; justify-content: flex-end; gap: 6px; }
  .f { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .f small { font-size: 11px; color: var(--muted); }
  .dim { opacity: .8; }
  .inp { background: var(--sunken); border-radius: 6px; padding: 5px 9px; font-size: 12px; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pane .dlg .inp { background: var(--surface); }
  .mono { font-variant-numeric: tabular-nums; letter-spacing: .02em; flex: 1; }
  .idrow { display: flex; gap: 6px; align-items: center; }
  .row { display: flex; gap: 14px; align-items: flex-start; }
  .grow { flex: 1; }
  .up { width: 54px; height: 54px; border-radius: 12px; border: 1.5px dashed var(--muted); display: flex; align-items: center; justify-content: center; }
  .up i { width: 14px; height: 14px; background:
    linear-gradient(var(--muted), var(--muted)) center / 100% 1.5px no-repeat,
    linear-gradient(var(--muted), var(--muted)) center / 1.5px 100% no-repeat; }
  .grid { display: grid; grid-template-columns: repeat(5, minmax(0, 1fr)); gap: 8px; }
  .cell { display: flex; flex-direction: column; align-items: center; gap: 4px; min-width: 0; }
  .cell small { font-size: 11px; color: var(--muted); max-width: 100%; overflow: hidden; text-overflow: ellipsis; }
  .cell .tile, .cell .add { width: 100%; max-width: 46px; aspect-ratio: 1; border-radius: 8px; }
  .add { border: 1.5px dashed var(--muted); display: block; }
  @media (max-width: 900px) { .side { width: 118px; } .it { font-size: 11px; padding-inline: 6px; } }
</style>
