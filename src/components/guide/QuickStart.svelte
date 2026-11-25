<script lang="ts">
  import { guide } from '../../lib/guide.svelte';
  import StepHead from './StepHead.svelte';
  import { app } from '../../lib/store.svelte';

  const cur = $derived(guide.step(1));
  const set = (n: number) => guide.setStep(1, n);
</script>

<ol class="steps">
  <li>
    <StepHead n={1} title="Pick a game" current={cur === 1} onclick={() => set(1)} />
    <div class="sb"><p>Type the name of a game in the search box and choose it from the list. It is Discord's own list, so the name and icon look like the real thing.</p></div>
  </li>
  <li>
    <StepHead n={2} title="Say what you are doing" current={cur === 2} onclick={() => set(2)} />
    <div class="sb"><p>Fill in "Details" and "State". Details is the first line, for example "Racing, lap 3". State is the second, for example "In a lobby".</p></div>
  </li>
  <li>
    <StepHead n={3} title="Press Show in Discord" current={cur === 3} onclick={() => set(3)} />
    <div class="sb">
      <p>Nothing appears on your profile until you do this. Press the "Show in Discord" button in the band above the editor, or the switch in the top right corner. It then reads "Showing in Discord".</p>
      <p>Keep the Discord desktop app open on this computer. Decoy can stay in the tray.</p>
      <div class="acts">
        {#if app.draft && !app.showing}
          <button type="button" class="btn primary" onclick={() => { void app.setShowing(true); guide.close(); }}>Show in Discord</button>
          <button type="button" class="btn" onclick={() => guide.close()}>Back to editor</button>
        {:else}
          <button type="button" class="btn primary" onclick={() => guide.close()}>Back to editor</button>
        {/if}
      </div>
    </div>
  </li>
</ol>

<p class="alt">Want your own name and icon? <button type="button" class="link" onclick={() => guide.go(2)}>Make your own application</button></p>

<style>
  .alt { margin-top: 22px; color: var(--muted); display: flex; gap: 6px; flex-wrap: wrap; align-items: baseline; }
  .alt .link { align-self: auto; }
</style>
