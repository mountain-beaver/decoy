<script lang="ts">
  import { app } from '../lib/store.svelte';
  import type { StatusDisplay } from '../lib/types';
  import TextField from './TextField.svelte';
  import Disclosure from './Disclosure.svelte';
  import Seg from './Seg.svelte';
  import Errs from './Errs.svelte';

  const p = $derived(app.p);
  const uid = $props.id();
  let more = $state(!!(app.p.details_url || app.p.state_url || app.p.status_display !== 'name'));

  const shows: { value: StatusDisplay; label: string }[] = [
    { value: 'name', label: 'Game name' },
    { value: 'details', label: 'Details' },
    { value: 'state', label: 'State' },
  ];
  const partyBad = $derived(app.issuesFor('party').length > 0);
</script>

<section class="group" aria-labelledby="{uid}-h">
  <h3 id="{uid}-h">Text</h3>
  <div class="two">
    <TextField label="Details" value={p.details} k="details" max={128} oninput={(v) => (p.details = v || null)} />
    <TextField label="State" value={p.state} k="state" max={128} oninput={(v) => (p.state = v || null)} />
  </div>

  {#if p.party}
    <div class="field">
      <span class="lab" id="{uid}-party">Party size</span>
      <div class="party" role="group" aria-labelledby="{uid}-party">
        <input type="number" min="1" max="999" aria-label="Current party size" aria-invalid={partyBad || undefined}
          value={p.party.current} oninput={(e) => { if (p.party) p.party.current = Math.max(0, Math.floor(+e.currentTarget.value || 0)); }} />
        <span class="of">of</span>
        <input type="number" min="1" max="999" aria-label="Maximum party size" aria-invalid={partyBad || undefined}
          value={p.party.max} oninput={(e) => { if (p.party) p.party.max = Math.max(0, Math.floor(+e.currentTarget.value || 0)); }} />
        <button type="button" class="icon-btn" aria-label="Remove party size" title="Remove party size" onclick={() => (p.party = null)}>
          <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
        </button>
      </div>
      <Errs keys={['party']} />
    </div>
  {:else}
    <button type="button" class="ghost" onclick={() => (p.party = { current: 1, max: 4 })}>
      <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M6 1.5v9M1.5 6h9" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>
      Add party size
    </button>
  {/if}

  <Disclosure bind:open={more}>
    <div class="two">
      <TextField label="Details link" type="url" value={p.details_url} k="details_url" placeholder="https://" oninput={(v) => (p.details_url = v || null)} />
      <TextField label="State link" type="url" value={p.state_url} k="state_url" placeholder="https://" oninput={(v) => (p.state_url = v || null)} />
    </div>
    <div class="field">
      <span class="lab" id="{uid}-sd">Show in member list</span>
      <Seg label="Show in member list" options={shows} value={p.status_display} onchange={(v) => (p.status_display = v)} />
      <p class="caption">What appears under your name in the member list.</p>
    </div>
  </Disclosure>
</section>

<style>
  .party { display: flex; align-items: center; gap: 8px; }
  .party input { width: 72px; }
  .of { color: var(--muted); }
</style>
