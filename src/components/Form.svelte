<script lang="ts">
  import AppSource from './AppSource.svelte';
  import TextGroup from './TextGroup.svelte';
  import TimerGroup from './TimerGroup.svelte';
  import ImagesGroup from './ImagesGroup.svelte';
  import ButtonsGroup from './ButtonsGroup.svelte';
</script>

<div class="form-inner">
  <AppSource />
  <TextGroup />
  <TimerGroup />
  <ImagesGroup />
  <ButtonsGroup />
</div>

<style>
  .form-inner { max-width: 640px; margin: 0 auto; display: flex; flex-direction: column; gap: 24px; }
</style>
