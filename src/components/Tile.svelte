<script lang="ts">
  import { hueOf } from '../lib/util';

  let { url = null, seed = '', size, round = false, class: cls = '' }: {
    url?: string | null; seed?: string; size?: number; round?: boolean; class?: string;
  } = $props();

  let failed = $state(false);
  $effect(() => { url; failed = false; });
  const hue = $derived(hueOf(seed || 'decoy'));
</script>

{#if url && !failed}
  <img class="tile {cls}" src={url} alt="" referrerpolicy="no-referrer" draggable="false"
    style:width={size ? `${size}px` : undefined} style:height={size ? `${size}px` : undefined}
    style:border-radius={round ? '50%' : undefined}
    onerror={() => (failed = true)} />
{:else}
  <i class="tile {cls}" style:--h={hue}
    style:width={size ? `${size}px` : undefined} style:height={size ? `${size}px` : undefined}
    style:border-radius={round ? '50%' : undefined}></i>
{/if}
