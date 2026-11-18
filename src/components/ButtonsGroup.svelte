<script lang="ts">
  import { app } from '../lib/store.svelte';
  import TextField from './TextField.svelte';
  import Errs from './Errs.svelte';

  const p = $derived(app.p);
  const uid = $props.id();
</script>

<section class="group" aria-labelledby="{uid}-h">
  <h3 id="{uid}-h">Buttons</h3>
  {#each p.buttons as b, i (i)}
    <div class="row">
      <TextField label="Label" value={b.label} k="button_label:{i}" max={32} oninput={(v) => (b.label = v)} />
      <TextField label="Link" type="url" value={b.url} k="button_url:{i}" placeholder="https://" oninput={(v) => (b.url = v)} />
      <button type="button" class="icon-btn rm" aria-label="Remove button {i + 1}" title="Remove button" onclick={() => p.buttons.splice(i, 1)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      </button>
    </div>
  {/each}
  <Errs keys={['buttons']} />
  {#if p.buttons.length < 2}
    <button type="button" class="ghost" onclick={() => p.buttons.push({ label: '', url: '' })}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M6 1.5v9M1.5 6h9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      Add button
    </button>
  {/if}
  <p class="caption">Discord doesn't let you click your own buttons. Other people can.</p>
</section>

<style>
  .row { display: grid; grid-template-columns: 1fr 1fr 32px; gap: 10px; align-items: start; }
  .rm { margin-top: 22px; }
  @container (max-width: 350px) {
    .row { grid-template-columns: 1fr 32px; }
    .row > :nth-child(2) { grid-column: 1; }
    .rm { grid-row: 1; grid-column: 2; }
  }
</style>
