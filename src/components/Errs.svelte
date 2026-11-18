<script lang="ts">
  import { app } from '../lib/store.svelte';
  let { keys, id }: { keys: string[]; id?: string } = $props();
  const list = $derived(app.issuesFor(...keys));
</script>

{#if list.length}
  <div {id} class="stack" style="gap:3px" aria-live="polite">
    {#each list as i (i.message)}<p class="err">{i.message}</p>{/each}
  </div>
{/if}
