<script lang="ts">
  let { n, title, current = false, expanded, controls, onclick }: {
    n: number; title: string; current?: boolean; expanded?: boolean; controls?: string; onclick: () => void;
  } = $props();
</script>

<h3 class="sh">
  <button type="button" class:cur={current} aria-expanded={expanded} aria-controls={controls}
    aria-current={expanded === undefined && current ? 'step' : undefined} {onclick}>
    <span class="n" aria-hidden="true">{n}</span>
    <span class="t">{title}</span>
  </button>
</h3>

<style>
  .sh { font: inherit; }
  button {
    display: flex; align-items: center; gap: 14px; width: 100%; text-align: left; background: none; border: 0;
    padding: 6px 0; border-radius: 8px;
  }
  .n {
    width: 24px; height: 24px; border-radius: 50%; border: 1px solid var(--line); color: var(--muted); flex: none;
    display: inline-flex; align-items: center; justify-content: center; font-weight: 600; font-size: 12px; font-variant-numeric: tabular-nums;
    background: var(--surface);
  }
  .t { font: 700 15px/1.25 var(--display); color: var(--muted); }
  button:hover .t { color: var(--ink); }
  .cur .n { background: var(--accent); border-color: transparent; color: var(--on-accent); }
  .cur .t { color: var(--ink); }
</style>
