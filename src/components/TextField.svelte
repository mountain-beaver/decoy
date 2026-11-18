<script lang="ts">
  import { app } from '../lib/store.svelte';
  import { chars } from '../lib/util';
  import Errs from './Errs.svelte';

  let {
    label, value, oninput, k, max = 0, placeholder = '', type = 'text', hint = '', optional = false,
  }: {
    label: string; value: string | null | undefined; oninput: (v: string) => void; k: string;
    max?: number; placeholder?: string; type?: string; hint?: string; optional?: boolean;
  } = $props();

  const uid = $props.id();
  const n = $derived(chars(value));
  const bad = $derived(app.issuesFor(k).length > 0);
</script>

<div class="field">
  <label for={uid}>
    <span>{label}{#if optional}<span class="opt"> (optional)</span>{/if}</span>
    {#if max}<em class:over={n > max}>{n} / {max}</em>{/if}
  </label>
  <input id={uid} {type} value={value ?? ''} {placeholder} autocomplete="off" spellcheck="false"
    aria-invalid={bad || undefined} aria-describedby={`${uid}-e${hint ? ` ${uid}-h` : ''}`}
    oninput={(e) => oninput(e.currentTarget.value)} />
  {#if hint}<p class="caption" id="{uid}-h">{hint}</p>{/if}
  <Errs keys={[k]} id="{uid}-e" />
</div>

<style>
  .opt { color: color-mix(in oklab, var(--muted) 70%, transparent); font-weight: 400; }
</style>
