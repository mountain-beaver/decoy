<script lang="ts" generics="T extends string">
  let { options, value, onchange, label }: {
    options: { value: T; label: string }[]; value: T; onchange: (v: T) => void; label: string;
  } = $props();
  const name = $props.id();
</script>

<div class="seg" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <label>
      <input type="radio" class="sr" {name} value={o.value} checked={value === o.value} onchange={() => onchange(o.value)} />
      <span>{o.label}</span>
    </label>
  {/each}
</div>
