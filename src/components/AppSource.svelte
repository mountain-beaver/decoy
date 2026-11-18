<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { isAppId } from '../lib/util';
  import { guide } from '../lib/guide.svelte';
  import type { ActivityKind } from '../lib/types';
  import Seg from './Seg.svelte';
  import Tile from './Tile.svelte';
  import GameSearch from './GameSearch.svelte';
  import TextField from './TextField.svelte';
  import Errs from './Errs.svelte';

  const p = $derived(app.p);
  const id = $derived(p.app_id.trim());
  const entry = $derived(app.entry(id));
  const uid = $props.id();

  // null: the application source decides, true/false: the user's choice
  let ownPref = $state<boolean | null>(null);
  let changing = $state(false);
  const own = $derived(ownPref ?? (entry?.origin === 'own' || (!!id && entry?.state === 'error')));
  const showPicked = $derived(!own && !changing && isAppId(id) && entry?.state === 'ok');

  const kinds: { value: ActivityKind; label: string }[] = [
    { value: 'playing', label: 'Playing' },
    { value: 'listening', label: 'Listening to' },
    { value: 'watching', label: 'Watching' },
    { value: 'competing', label: 'Competing in' },
  ];

  let timer: ReturnType<typeof setTimeout> | undefined;
  function typed(v: string) {
    p.app_id = v.trim();
    clearTimeout(timer);
    if (isAppId(v)) timer = setTimeout(() => app.ensureApp(v), 350);
  }

  const missingId = $derived(!id);
  const idIssues = $derived(app.issuesFor('app_id'));
</script>

<section class="group" aria-labelledby="{uid}-h">
  <h3 id="{uid}-h">What you're doing</h3>
  <Seg label="Activity type" options={kinds} value={p.kind} onchange={(v) => (p.kind = v)} />

  {#if showPicked && entry}
    <div class="pick">
      <Tile url={entry.icon_url} seed={id} size={22} />
      <b>{entry.name}</b>
      <small>{entry.origin === 'own' ? 'your own application' : entry.origin === 'list' ? "from Discord's game list" : `ID ${id}`}</small>
      <button type="button" class="link" style="margin-left:4px" onclick={() => (changing = true)}>Change</button>
    </div>
    <div class="row">
      <button type="button" class="link" onclick={() => { ownPref = true; }}>Use my own application ID</button>
      <button type="button" class="link" onclick={() => guide.show(2)}>How do I get my own?</button>
    </div>
  {:else if own}
    <div class="field">
      <label for="{uid}-id">Application ID</label>
      <input id="{uid}-id" type="text" inputmode="numeric" value={p.app_id} placeholder="For example 1445160676522856458"
        autocomplete="off" spellcheck="false" aria-invalid={(!missingId && idIssues.length > 0) || entry?.state === 'error' || undefined}
        aria-describedby="{uid}-idr" oninput={(e) => typed(e.currentTarget.value)} />
      <div id="{uid}-idr" class="stack" style="gap:6px">
        {#if entry?.state === 'loading'}
          <p class="caption" style="display:flex;gap:8px;align-items:center"><span class="spinner"></span>Looking up this application...</p>
        {:else if entry?.state === 'ok'}
          <div class="pick">
            <Tile url={entry.icon_url} seed={id} size={22} />
            <b>{entry.name}</b>
            <small>found on Discord</small>
          </div>
        {:else if entry?.state === 'error' && isAppId(id)}
          <p class="err"><span>Discord could not find this application. {entry.message && entry.message !== 'Not found' ? `${entry.message} ` : ''}Copy the ID from General Information in your application's page in the <button type="button" class="link" onclick={() => app.openExternal('https://discord.com/developers/applications')}>Discord Developer Portal</button>.</span></p>
        {:else if !missingId}
          <Errs keys={['app_id']} />
        {:else}
          <p class="caption">Create an application in the Developer Portal, then paste its Application ID here. <button type="button" class="link" style="display:inline" onclick={() => guide.show(2)}>How do I get my own?</button></p>
        {/if}
      </div>
    </div>
    <button type="button" class="link" onclick={() => { ownPref = false; changing = true; }}>Search Discord's game list instead</button>
  {:else}
    {#if isAppId(id) && entry?.state === 'loading'}
      <div class="pick"><Tile seed={id} size={22} /><b>Looking up...</b></div>
    {:else}
      <GameSearch onown={() => (ownPref = true)} onpick={(g) => { changing = false; void app.pickGame(g); }} />
    {/if}
    <div class="row">
      {#if changing && isAppId(id) && entry?.state === 'ok'}
        <button type="button" class="link" onclick={() => (changing = false)}>Keep {entry.name}</button>
      {/if}
      <button type="button" class="link" onclick={() => (ownPref = true)}>Use my own application ID</button>
      <button type="button" class="link" onclick={() => guide.show(2)}>How do I get my own?</button>
    </div>
    {#if missingId}
      <p class="caption">Pick a game to show. Discord shows its icon next to your name in voice channels.</p>
    {/if}
  {/if}

  <TextField label="Display name" value={p.name} k="name" max={128}
    placeholder={entry?.state === 'ok' ? entry.name : 'Same as the game'}
    hint="Replaces the game's name on your profile. Discord may ignore it."
    oninput={(v) => (p.name = v || null)} />
</section>

<style>
  .row { display: flex; gap: 14px; flex-wrap: wrap; }
</style>
