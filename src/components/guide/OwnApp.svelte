<script lang="ts">
  import { app } from '../../lib/store.svelte';
  import { guide } from '../../lib/guide.svelte';
  import { isAppId } from '../../lib/util';
  import StepHead from './StepHead.svelte';
  import Schematic from './Schematic.svelte';
  import Tile from '../Tile.svelte';

  const uid = $props.id();
  const cur = $derived(guide.step(2));
  const set = (n: number) => guide.setStep(2, n);

  // step 4: the same lookup as in "Use my own application ID"
  let v = $state('');
  let touched = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  const id = $derived(v.trim());
  const valid = $derived(isAppId(id));
  const entry = $derived(valid ? app.entry(id) : undefined);
  const bad = $derived(id.length > 0 && !valid && (touched || /\D/.test(id) || id.length > 20));

  function typed() {
    clearTimeout(timer);
    if (isAppId(id)) timer = setTimeout(() => void app.lookup(id, 'own'), 300);
  }
  $effect(() => {
    guide.preview = entry?.state === 'ok' ? { name: entry.name ?? id, icon_url: entry.icon_url ?? null } : null;
  });

  async function use() {
    if (entry?.state !== 'ok') return;
    const name = entry.name ?? id;
    await app.pickGame({ id, name, icon_url: entry.icon_url ?? null }, 'own');
    guide.close();
    app.toast(app.showing ? `Now showing as ${name}` : `${name} is set. Switch on "Showing in Discord" to share it.`);
  }
</script>

{#snippet nav(n: number)}
  <div class="acts">
    {#if n > 1}<button type="button" class="btn" onclick={() => set(n - 1)}>Back</button>{/if}
    {#if n < 4}<button type="button" class="btn primary" onclick={() => set(n + 1)}>Next</button>{/if}
  </div>
{/snippet}

<ol class="steps">
  <li>
    <StepHead n={1} title="Create the application" current={cur === 1} expanded={cur === 1} controls="{uid}-1" onclick={() => set(1)} />
    {#if cur === 1}
      <div class="sb" class:rise={guide.moved} id="{uid}-1">
        <p>Open the Developer Portal and sign in with your Discord account. Press "New Application", type a name, then press "Create".</p>
        <p>People see this name after "Playing".</p>
        <Schematic kind="create" />
        <div class="acts"><button type="button" class="btn" onclick={() => app.openExternal('https://discord.com/developers/applications')}>Open the Developer Portal</button></div>
        {@render nav(1)}
      </div>
    {/if}
  </li>
  <li>
    <StepHead n={2} title="Add an icon" current={cur === 2} expanded={cur === 2} controls="{uid}-2" onclick={() => set(2)} />
    {#if cur === 2}
      <div class="sb" class:rise={guide.moved} id="{uid}-2">
        <p>On the "General Information" page, upload an app icon. 1024 x 1024 works best.</p>
        <p>This is the small icon shown next to your name in voice channels.</p>
        <Schematic kind="icon" />
        {@render nav(2)}
      </div>
    {/if}
  </li>
  <li>
    <StepHead n={3} title="Upload pictures (optional)" current={cur === 3} expanded={cur === 3} controls="{uid}-3" onclick={() => set(3)} />
    {#if cur === 3}
      <div class="sb" class:rise={guide.moved} id="{uid}-3">
        <p>In the left sidebar open "Rich Presence", then "Art Assets". Add your images. 1024 x 1024 is recommended, and you can add up to 300.</p>
        <p>The name you give a picture is its key. Discord makes it lowercase. Type that key into "Large image" or "Small image" in Decoy. New pictures can take a few minutes to appear.</p>
        <p>No time to upload? Paste an https link to any picture instead. That also allows GIFs.</p>
        <Schematic kind="assets" />
        {@render nav(3)}
      </div>
    {/if}
  </li>
  <li>
    <StepHead n={4} title="Copy the Application ID" current={cur === 4} expanded={cur === 4} controls="{uid}-4" onclick={() => set(4)} />
    {#if cur === 4}
      <div class="sb" class:rise={guide.moved} id="{uid}-4">
        <p>On the "General Information" page, press "Copy" under "Application ID". Then paste it here.</p>
        <Schematic kind="id" />
        <form class="idf" onsubmit={(e) => { e.preventDefault(); void use(); }}>
          <div class="field">
            <label for="{uid}-id">Application ID</label>
            <input id="{uid}-id" type="text" inputmode="numeric" bind:value={v} oninput={typed} onblur={() => (touched = true)}
              placeholder="For example 1445160676522856458" autocomplete="off" spellcheck="false"
              aria-invalid={bad || entry?.state === 'error' || undefined} aria-describedby="{uid}-res" />
          </div>
          <div id="{uid}-res" class="res" aria-live="polite">
            {#if entry?.state === 'loading'}
              <p class="caption lk"><span class="spinner"></span>Looking up this application...</p>
            {:else if entry?.state === 'ok'}
              <div class="pick"><Tile url={entry.icon_url} seed={id} size={22} /><b>{entry.name}</b><small>found on Discord</small></div>
            {:else if entry?.state === 'error'}
              <p class="err"><span>Discord could not find this application. Check the ID and try again.</span></p>
            {:else if bad}
              <p class="err"><span>An Application ID is 17 to 20 digits, with no spaces or letters.</span></p>
            {:else}
              <p class="caption">Paste the ID and Decoy looks it up.</p>
            {/if}
          </div>
          <div class="acts">
            <button type="button" class="btn" onclick={() => set(3)}>Back</button>
            <button type="submit" class="btn primary" disabled={entry?.state !== 'ok'}>Use it in this profile</button>
          </div>
        </form>
      </div>
    {/if}
  </li>
</ol>

<style>
  .idf { display: flex; flex-direction: column; gap: 10px; }
  .res { min-height: 34px; display: flex; flex-direction: column; justify-content: center; }
  .lk { display: flex; gap: 8px; align-items: center; }
</style>
