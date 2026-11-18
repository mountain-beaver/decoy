<script lang="ts">
  let { label, hint = '', checked, onchange }: { label: string; hint?: string; checked: boolean; onchange: (v: boolean) => void } = $props();
  const uid = $props.id();
</script>

<div class="row">
  <div class="txt">
    <span id="{uid}-l" class="l">{label}</span>
    {#if hint}<span id="{uid}-d" class="caption">{hint}</span>{/if}
  </div>
  <button type="button" class="tog" role="switch" aria-checked={checked} aria-labelledby="{uid}-l" aria-describedby={hint ? `${uid}-d` : undefined}
    onclick={() => onchange(!checked)}><i></i></button>
</div>

<style>
  .row { display: flex; align-items: center; gap: 16px; justify-content: space-between; padding: 9px 0; }
  .txt { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .l { font-weight: 500; }
  .tog { width: 38px; height: 22px; border-radius: 999px; border: 1px solid var(--line); background: var(--sunken); padding: 0; position: relative; flex: none; transition: background .15s; }
  .tog i { position: absolute; top: 3px; left: 3px; width: 14px; height: 14px; border-radius: 50%; background: var(--muted); transition: transform .15s, background .15s; }
  .tog[aria-checked="true"] { background: var(--accent); border-color: transparent; }
  .tog[aria-checked="true"] i { transform: translateX(16px); background: var(--on-accent); }
</style>
