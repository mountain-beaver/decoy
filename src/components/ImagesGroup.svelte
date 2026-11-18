<script lang="ts">
  import ImageField from './ImageField.svelte';
  const uid = $props.id();
</script>

<section class="group" aria-labelledby="{uid}-h">
  <h3 id="{uid}-h">Images</h3>
  <div class="imgs">
    <ImageField which="large" />
    <ImageField which="small" />
  </div>
</section>

<style>
  .imgs { display: grid; grid-template-columns: 1fr 1fr; gap: 14px 12px; align-items: start; }
  @container (max-width: 540px) { .imgs { grid-template-columns: 1fr; } }
</style>
