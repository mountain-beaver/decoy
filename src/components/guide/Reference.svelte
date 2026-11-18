<script lang="ts">
  let { hot = $bindable<string | null>(null) }: { hot?: string | null } = $props();

  const items: { key: string; term: string; text: string[] }[] = [
    { key: 'large', term: 'Large image', text: ['The big square on the left of your card. Without one, Discord shows the application icon.'] },
    { key: 'small', term: 'Small image', text: ['A round badge on the corner of the large image. Good for a rank, a mood or a status.'] },
    { key: 'key', term: 'Asset key or link', text: [
      'A key is the name of a picture you uploaded to your own application. It only works with that application.',
      'A link must start with https://. It works with any picture on the web and also allows GIFs.',
    ] },
    { key: 'hover', term: 'Hover text', text: ['Shown when someone points at a picture. It needs 2 to 128 characters.'] },
    { key: 'buttons', term: 'Buttons', text: [
      'You can add up to two. Each has a label of up to 32 characters and a link that starts with https://.',
      'You cannot click your own buttons. Other people can.',
    ] },
    { key: 'party', term: 'Party size', text: ['Adds a count such as "(2 of 4)" to the "State" line. Use it for a lobby or a group.'] },
    { key: 'timer', term: 'Timer', text: [] },
    { key: 'member', term: 'Show in member list', text: ['Chooses the line under your name in the member list: "Game name", "Details" or "State".'] },
  ];
  const timers = [
    ['Off', 'No timer.'],
    ['Since I turned it on', 'Counts up from the moment you switch on "Showing in Discord".'],
    ['Since last change', 'Starts over whenever you edit the profile.'],
    ['Time of day', 'Counts up from midnight, so it reads like a clock.'],
    ['Custom', 'Pick a start, an end or both. With an end it counts down.'],
  ];

  function leave(e: FocusEvent | PointerEvent) {
    const list = e.currentTarget as HTMLElement;
    if (!list.contains(document.activeElement)) hot = null;
  }
</script>

<div class="ref" role="list" onpointerleave={leave}>
  {#each items as it (it.key)}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <article class="it" class:on={hot === it.key} role="listitem" tabindex="0"
      onpointerenter={() => (hot = it.key)} onfocus={() => (hot = it.key)} onblur={() => { if (hot === it.key) hot = null; }}>
      <h3>{it.term}</h3>
      {#each it.text as t (t)}<p>{t}</p>{/each}
      {#if it.key === 'timer'}
        <dl>
          {#each timers as [k, d] (k)}<div><dt>"{k}"</dt><dd>{d}</dd></div>{/each}
        </dl>
      {/if}
    </article>
  {/each}
</div>

<style>
  .ref { display: flex; flex-direction: column; }
  .it { position: relative; padding: 12px 0; border-top: 1px solid var(--line); border-radius: 0; outline-offset: -2px; }
  .it:first-child { border-top: 0; }
  .it::before {
    content: ""; position: absolute; left: -12px; top: 14px; bottom: 14px; width: 4px; border-radius: 2px; background: var(--accent);
    opacity: 0; transform: scaleY(.6); transition: opacity .15s, transform .15s;
  }
  .it.on::before { opacity: 1; transform: none; }
  h3 { font: 600 13.5px/1.3 var(--ui); margin-bottom: 3px; }
  p { color: var(--muted); max-width: 62ch; text-wrap: pretty; }
  p + p { margin-top: 4px; }
  dl { margin: 6px 0 0; display: flex; flex-direction: column; gap: 4px; }
  dl div { display: grid; grid-template-columns: 9.5em minmax(0, 1fr); gap: 10px; }
  dt { font-weight: 500; }
  dd { margin: 0; color: var(--muted); }
  @media (max-width: 900px) { dl div { grid-template-columns: 1fr; gap: 0; } }
</style>
