<script lang="ts">
  import { app } from '../lib/store.svelte';
  import GameSearch from './GameSearch.svelte';
  import OwnIdStart from './OwnIdStart.svelte';
  import { guide } from '../lib/guide.svelte';

  let own = $state(false);
</script>

<div class="welcome">
  <div class="mark" aria-hidden="true">
    <i class="tile" style="--h:330"></i><i class="tile" style="--h:185"></i><i class="tile" style="--h:28"></i>
  </div>
  <h1>What are you playing?</h1>
  <p class="lead">Pick a game, add your own text if you like, then press "Show in Discord". Decoy puts it on your Discord profile. You can change everything afterwards.</p>
  {#if own}
    <OwnIdStart onback={() => (own = false)} />
    <button type="button" class="link" onclick={() => guide.show(2)}>How do I get my own?</button>
  {:else}
    <GameSearch hero autofocus onpick={(g) => void app.pickGame(g)} onown={() => (own = true)} />
    <button type="button" class="link" onclick={() => (own = true)}>Use my own application ID</button>
    <button type="button" class="link" onclick={() => guide.show(1)}>New here? Read the short guide</button>
  {/if}
</div>

<style>
  .welcome { max-width: 460px; margin: 0 auto; padding: 36px 0 20px; display: flex; flex-direction: column; gap: 14px; }
  .mark { display: flex; margin-bottom: 4px; }
  .mark .tile { width: 44px; height: 44px; border-radius: 12px; border: 3px solid var(--surface); margin-left: -10px; }
  .mark .tile:first-child { margin-left: 0; transform: rotate(-6deg); }
  .mark .tile:last-child { transform: rotate(7deg); }
  h1 { font: 700 28px/1.08 var(--display); letter-spacing: -.02em; text-wrap: balance; }
  .lead { color: var(--muted); font-size: 14px; line-height: 1.5; text-wrap: pretty; margin-bottom: 6px; }
</style>
