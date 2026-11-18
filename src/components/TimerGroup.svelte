<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { fromLocalInput, toLocalInput } from '../lib/util';
  import type { TimestampMode } from '../lib/types';
  import Seg from './Seg.svelte';
  import Errs from './Errs.svelte';

  const p = $derived(app.p);
  const uid = $props.id();
  type Mode = TimestampMode['mode'];
  const modes: { value: Mode; label: string }[] = [
    { value: 'none', label: 'Off' },
    { value: 'since_connect', label: 'Since I turned it on' },
    { value: 'since_update', label: 'Since last change' },
    { value: 'local_time', label: 'Time of day' },
    { value: 'custom', label: 'Custom' },
  ];
  const custom = $derived(p.timestamps.mode === 'custom' ? p.timestamps : null);

  function setMode(m: Mode) {
    p.timestamps = m === 'custom' ? { mode: 'custom', start_ms: null, end_ms: null } : ({ mode: m } as TimestampMode);
  }
  function setEdge(which: 'start_ms' | 'end_ms', v: string) {
    if (p.timestamps.mode === 'custom') p.timestamps[which] = fromLocalInput(v);
  }
  const bad = $derived(app.issuesFor('timestamps').length > 0);
  const hint = $derived.by(() => {
    switch (p.timestamps.mode) {
      case 'none': return 'No timer is shown.';
      case 'since_connect': return 'Counts up from the moment you switched on "Showing in Discord".';
      case 'since_update': return 'Starts again every time you change something.';
      case 'local_time': return 'Counts up from midnight, so it reads like the time of day.';
      default: return 'Start alone counts up. End, with or without a start, counts down.';
    }
  });
</script>

<section class="group" aria-labelledby="{uid}-h">
  <h3 id="{uid}-h">Timer</h3>
  <Seg label="Timer" options={modes} value={p.timestamps.mode} onchange={setMode} />
  {#if custom}
    <div class="two">
      <div class="field">
        <label for="{uid}-s"><span>Start</span>{#if custom.start_ms != null}<button type="button" class="link clr" onclick={() => setEdge('start_ms', '')}>Clear</button>{/if}</label>
        <input id="{uid}-s" type="datetime-local" value={toLocalInput(custom.start_ms)} aria-invalid={bad || undefined}
          oninput={(e) => setEdge('start_ms', e.currentTarget.value)} />
      </div>
      <div class="field">
        <label for="{uid}-e"><span>End</span>{#if custom.end_ms != null}<button type="button" class="link clr" onclick={() => setEdge('end_ms', '')}>Clear</button>{/if}</label>
        <input id="{uid}-e" type="datetime-local" value={toLocalInput(custom.end_ms)} aria-invalid={bad || undefined}
          oninput={(e) => setEdge('end_ms', e.currentTarget.value)} />
      </div>
    </div>
    <Errs keys={['timestamps']} />
  {/if}
  <p class="caption">{hint}</p>
</section>

<style>
  .clr { font-size: 12px; align-self: auto; }
</style>
