<script lang="ts">
  import { app } from '../../lib/store.svelte';
  import { userName } from '../../lib/util';

  const uid = $props.id();
  const e = $derived(app.engine);

  const live = $derived.by(() => {
    switch (e.status) {
      case 'active': case 'connected':
        return { tone: 'ok', short: 'Connected', line: `Right now: Connected to Discord${userName(e.user) ? ` as ${userName(e.user)}` : ''}.`, more: 'If your profile is still empty, check the next points.' };
      case 'connecting':
        return { tone: 'warn', short: 'Connecting', line: 'Right now: Connecting to Discord.', more: 'Give it a few seconds.' };
      case 'waiting_for_discord':
        return { tone: 'warn', short: 'Waiting', line: 'Right now: Waiting for Discord. Start Discord on this computer.', more: 'Decoy connects by itself once Discord is running.' };
      case 'failed':
        return { tone: 'bad', short: 'Refused', line: 'Right now: Discord refused the presence.', more: e.message };
      default:
        return { tone: '', short: 'Off', line: 'Right now: Not connected.', more: 'Press "Show in Discord" above the editor or in the top right corner.' };
    }
  });

  const rows: { q: string; a: string[] }[] = [
    { q: 'Is Decoy connected to Discord?', a: [] },
    { q: 'Is "Show in Discord" switched on?', a: [
      'Nothing is shared until you press "Show in Discord". The switch in the top right corner then reads "Showing in Discord".',
    ] },
    { q: 'Are you using the Discord desktop app?', a: [
      'Discord must be the desktop app, on the same computer as Decoy. The browser version cannot do this.',
    ] },
    { q: 'Is activity sharing turned on?', a: [
      'In Discord open User Settings, then "Activity Privacy". Turn on sharing your activity.',
      'The wording differs between accounts: "Share your detected activities with others" or "Share my activity".',
    ] },
    { q: 'Is your status Invisible?', a: ['Invisible hides your activity. Switch to Online, Idle or Do Not Disturb.'] },
    { q: 'Do you use Vesktop?', a: [
      'Turn on its Rich Presence (arRPC) option in the Vesktop settings.',
      'Decoy then connects without showing your name. That is normal.',
    ] },
    { q: 'Did you just upload pictures?', a: [
      'New pictures can take a few minutes to appear. Until then Discord shows the application icon.',
      'Paste an https link to see a picture right away.',
    ] },
    { q: 'Are you changing things quickly?', a: [
      'Discord accepts about five updates per 20 seconds. If you edit fast, changes arrive in batches. Wait a moment.',
    ] },
    { q: 'Is your display name ignored?', a: [
      'Discord may ignore a custom "Display name" and show the application name instead. To change that name for good, rename the application in the Developer Portal.',
    ] },
    { q: 'Can you click your own buttons?', a: [
      'No. You never see your own buttons as clickable. Ask a friend to look at your profile.',
    ] },
  ];

  let open = $state<Record<number, boolean>>({ 0: true });
  let moved = $state(false);
</script>

<div class="rows">
  {#each rows as r, i (i)}
    <div class="row">
      <h3>
        <button type="button" aria-expanded={!!open[i]} aria-controls="{uid}-{i}" onclick={() => { moved = true; open[i] = !open[i]; }}>
          <span class="q">{r.q}</span>
          {#if i === 0}<span class="live {live.tone}"><i></i>{live.short}</span>{/if}
          <svg viewBox="0 0 10 10" aria-hidden="true"><path d="m3 1.5 3.5 3.5L3 8.5" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>
        </button>
      </h3>
      {#if open[i]}
        <div class="a" class:rise={moved} id="{uid}-{i}">
          {#if i === 0}
            <p class="now" aria-live="polite">{live.line}</p>
            <p>{live.more}</p>
          {:else}
            {#each r.a as t (t)}<p>{t}</p>{/each}
          {/if}
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .rows { display: flex; flex-direction: column; }
  .row { border-top: 1px solid var(--line); }
  .row:first-child { border-top: 0; }
  h3 { font: inherit; }
  button { display: flex; align-items: center; gap: 10px; width: 100%; text-align: left; background: none; border: 0; padding: 11px 0; border-radius: 6px; }
  .q { font-weight: 600; flex: 1; min-width: 0; }
  button svg { width: 10px; height: 10px; color: var(--muted); flex: none; transition: transform .15s; }
  button[aria-expanded="true"] svg { transform: rotate(90deg); }
  button:hover svg { color: var(--ink); }
  .live { display: inline-flex; align-items: center; gap: 6px; color: var(--muted); font-size: 12px; white-space: nowrap; }
  .live i { width: 8px; height: 8px; border-radius: 50%; background: var(--muted); }
  .live.ok i { background: var(--ok); }
  .live.warn i { background: var(--warn); }
  .live.bad i { background: var(--bad); }
  .a { padding: 0 20px 14px 0; display: flex; flex-direction: column; gap: 5px; }
  .a p { color: var(--muted); max-width: 62ch; text-wrap: pretty; }
  .a .now { color: var(--ink); font-weight: 500; }
</style>
