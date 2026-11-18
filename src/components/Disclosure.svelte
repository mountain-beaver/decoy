<script lang="ts">
  import type { Snippet } from 'svelte';
  let { label = 'More', open = $bindable(false), children }: { label?: string; open?: boolean; children: Snippet } = $props();
  const uid = $props.id();
</script>

<button type="button" class="more" aria-expanded={open} aria-controls={uid} onclick={() => (open = !open)}>
  <svg viewBox="0 0 10 10" aria-hidden="true"><path d="m3 1.5 3.5 3.5L3 8.5" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
  {label}
</button>
{#if open}
  <div id={uid} class="stack more-body">{@render children()}</div>
{/if}

<style>
  .more-body { padding: 2px 0 2px 12px; border-left: 2px solid var(--line); }
</style>
