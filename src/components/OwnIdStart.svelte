<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { isAppId } from '../lib/util';
  import Tile from './Tile.svelte';

  let { onback }: { onback: () => void } = $props();
  let v = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  const id = $derived(v.trim());
  const entry = $derived(app.entry(id));

  function typed() {
    clearTimeout(timer);
    if (isAppId(id)) timer = setTimeout(() => void app.lookup(id, 'own'), 300);
  }
  async function go() {
    if (entry?.state !== 'ok') return;
    await app.pickGame({ id, name: entry.name ?? id, icon_url: entry.icon_url ?? null }, 'own');
  }
</script>

<form class="own" onsubmit={(e) => { e.preventDefault(); void go(); }}>
  <div class="field">
    <label for="own-id">Application ID</label>
    <input id="own-id" type="text" inputmode="numeric" bind:value={v} oninput={typed} placeholder="For example 1445160676522856458"
      autocomplete="off" spellcheck="false" aria-invalid={entry?.state === 'error' || undefined} />
  </div>
  {#if entry?.state === 'loading'}
    <p class="caption" style="display:flex;gap:8px;align-items:center"><span class="spinner"></span>Looking up this application...</p>
  {:else if entry?.state === 'ok'}
    <div class="pick"><Tile url={entry.icon_url} seed={id} size={22} /><b>{entry.name}</b><small>found on Discord</small></div>
  {:else if entry?.state === 'error' && isAppId(id)}
    <p class="err"><span>Discord could not find this application. {entry.message ?? ''} Copy the ID from General Information in your application's page in the <button type="button" class="link" onclick={() => app.openExternal('https://discord.com/developers/applications')}>Discord Developer Portal</button>.</span></p>
  {:else}
    <p class="caption">Create an application in the Developer Portal, then paste its Application ID here.</p>
  {/if}
  <div class="acts">
    <button type="submit" class="btn primary" disabled={entry?.state !== 'ok'}>Use this application</button>
    <button type="button" class="link" onclick={onback}>Search Discord's game list instead</button>
  </div>
</form>

<style>
  .own { display: flex; flex-direction: column; gap: 10px; }
  .acts { display: flex; gap: 14px; align-items: center; flex-wrap: wrap; margin-top: 4px; }
</style>
