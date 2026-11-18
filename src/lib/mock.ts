// Mock backend for running in a plain browser (without Tauri): screenshots and development.
// URL parameters: ?theme=dark|light  ?engine=active|waiting|failed|connected  ?empty=1  ?games=loading|error
import type {
  AppInfo, AppState, AssetInfo, EngineState, GameHit, GamesInfo, Issue, Presence, Profile, Settings, FieldId,
} from './types';
import { emptyPresence } from './types';
import type { Backend } from './backend';
import { chars, clean, isUrl } from './util';

// ?fast=1 removes the delays so a screenshot catches the finished screen
const FAST = typeof location !== 'undefined' && new URLSearchParams(location.search).get('fast') === '1';
const wait = (ms: number) => (FAST ? Promise.resolve() : new Promise((r) => setTimeout(r, ms)));
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));

// Invented titles: the mock must not look like it ships anyone's catalogue.
const NAMES = [
  'Skyline Racer', 'Neon Harbor', 'Paper Kingdoms', 'Deep Field', 'Lantern Keep', 'Moss and Iron',
  'Orbit Garden', 'Rookfall', 'Tidewalker', 'Copper Sky', 'Night Bus Tycoon', 'Glasshouse', 'Hollow Pines',
  'Starling Cup', 'Quiet Signal', 'Brass Atlas', 'Salt Road', 'Thimble', 'Second Summer', 'Kiln',
  'Low Tide Detective', 'Foxglove', 'Ninety Mile Beach', 'Switchback', 'Paperweight', 'Cold Harbour Radio',
  'Meridian', 'Snowline Express', 'Pocket Orchestra', 'Greenhouse Wars', 'Dustjacket', 'Signal Fire',
  'Marrow Deep', 'Clockmaker\'s Apprentice', 'Sky Ferry', 'Tin Soldiers', 'Harbor Lights', 'Inkwell', 'Longboat', 'Sketchpad',
];
const FIRST_ID = '1300000000000000042';
const OWN_ID = '1100000000000000001';
const idFor = (i: number) => (i === 0 ? FIRST_ID : (1300000000000000000n + BigInt(i) * 90210012345n).toString());
const GAMES: GameHit[] = NAMES.map((name, i) => ({ id: idFor(i), name, icon_url: null }));

function svgAsset(a: string, b: string, glyph: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="64" height="64" fill="url(#g)"/><text x="32" y="42" font-size="26" text-anchor="middle" fill="rgba(255,255,255,.85)" font-family="sans-serif" font-weight="700">${glyph}</text></svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}
const OWN_ASSETS: AssetInfo[] = [
  { id: '9001', name: 'poster', url: svgAsset('#F6A04D', '#7A2E5B', 'M') },
  { id: '9002', name: 'popcorn', url: svgAsset('#FFE08A', '#E0553D', 'P') },
  { id: '9003', name: 'ticket', url: svgAsset('#7FE3D0', '#1D3B7A', 'T') },
];

/** The same rules as Presence::validate in the core. */
export function validatePresence(p: Presence): Issue[] {
  const out: Issue[] = [];
  const add = (field: FieldId, message: string) => out.push({ field, message });
  const urlCheck = (field: FieldId, label: string, v: string) => {
    if (!isUrl(v)) add(field, `${label} must start with https:// or http://`);
    else if (chars(v) > 512) add(field, `${label} must be 512 characters or fewer`);
  };
  if (!/^\d{17,20}$/.test(p.app_id.trim())) add('app_id', 'Application ID must be 17 to 20 digits');
  for (const [f, label, v] of [['name', 'Name', p.name], ['details', 'Details', p.details], ['state', 'State', p.state]] as const) {
    const t = clean(v);
    if (t && (chars(t) < 2 || chars(t) > 128)) add(f, `${label} must be 2 to 128 characters`);
  }
  for (const [f, label, v] of [['details_url', 'Details link', p.details_url], ['state_url', 'State link', p.state_url]] as const) {
    const t = clean(v);
    if (t) urlCheck(f, label, t);
  }
  if (p.party) {
    if (p.party.current < 1) add('party', 'Party size must be at least 1');
    else if (p.party.max < p.party.current) add('party', 'Party maximum must not be smaller than the current size');
  }
  if (p.timestamps.mode === 'custom' && p.timestamps.start_ms != null && p.timestamps.end_ms != null
    && p.timestamps.end_ms <= p.timestamps.start_ms) add('timestamps', 'End time must be after the start time');
  for (const [img, s, t, u, name] of [
    [p.large_image, 'large_image', 'large_image_text', 'large_image_url', 'Large image'],
    [p.small_image, 'small_image', 'small_image_text', 'small_image_url', 'Small image'],
  ] as const) {
    if (!img) continue;
    const src = img.source.trim();
    if (!src) continue;
    if (chars(src) > 256) add(s, `${name} source must be 256 characters or fewer`);
    else if (src.includes('://') && !isUrl(src)) add(s, `${name} URL must start with https:// or http://`);
    const tx = clean(img.text);
    if (tx && (chars(tx) < 2 || chars(tx) > 128)) add(t, `${name} text must be 2 to 128 characters`);
    const ul = clean(img.url);
    if (ul) urlCheck(u, `${name} link`, ul);
  }
  if (p.buttons.length > 2) add('buttons', 'Use at most 2 buttons');
  p.buttons.forEach((b, i) => {
    const l = b.label.trim();
    if (!l || chars(l) > 32) add({ button_label: i }, 'Button label must be 1 to 32 characters');
    const u = b.url.trim();
    if (!u) add({ button_url: i }, 'Button link is required');
    else urlCheck({ button_url: i }, 'Button link', u);
  });
  return out;
}

export function createMock(): Backend {
  const q = new URLSearchParams(typeof location === 'undefined' ? '' : location.search);
  const force = q.get('engine');
  const empty = q.get('empty') === '1';
  const gamesMode = q.get('games');
  const user = { id: '1', username: 'sky', global_name: 'Sky' };
  const now = Date.now();

  const profiles: Profile[] = empty ? [] : [
    {
      id: 'p-race', name: 'Skyline Racer',
      presence: {
        ...emptyPresence(), app_id: FIRST_ID, details: 'Racing, lap 3', state: 'In a lobby',
        party: { current: 2, max: 4 }, timestamps: { mode: 'since_connect' },
        large_image: { source: 'cover', text: 'Harbor circuit at dusk', url: null },
        small_image: { source: 'star', text: 'Level 12', url: null },
        buttons: [
          { label: 'Watch trailer', url: 'https://example.com/trailer' },
          { label: 'Join my crew', url: 'https://example.com/crew' },
        ],
      },
    },
    {
      id: 'p-code', name: 'Coding',
      presence: {
        ...emptyPresence(), app_id: idFor(1), details: 'Editing main.rs', state: 'In project decoy',
        timestamps: { mode: 'since_update' }, status_display: 'details',
      },
    },
    {
      id: 'p-movie', name: 'Movie night',
      presence: {
        ...emptyPresence(), app_id: OWN_ID, kind: 'watching', name: 'Movie night', details: 'The Lighthouse Keepers',
        state: 'With the group', timestamps: { mode: 'custom', start_ms: now - 20 * 60e3, end_ms: now + 110 * 60e3 },
        large_image: { source: 'poster', text: 'Tonight', url: null }, small_image: { source: 'popcorn', text: null, url: null },
      },
    },
  ];

  let settings: Settings = {
    active_profile: empty ? null : 'p-race', connect_on_launch: true, start_in_tray: false, autostart: false,
    skip_update_check: false,
  };
  let games: GamesInfo = gamesMode === 'loading'
    ? { count: 0, fetched_at: null, loading: true, error: null }
    : gamesMode === 'error'
      ? { count: 0, fetched_at: null, loading: false, error: 'Could not reach discord.com.' }
      : { count: 24812, fetched_at: Math.floor(Date.now() / 1000) - 5 * 86400, loading: false, error: null };

  let showing = !empty && force !== 'connected';
  let engine: EngineState = empty ? { status: 'connected', user } : engineFor(showing);

  function engineFor(on: boolean): EngineState {
    if (!on) return force === 'waiting' || force === 'failed' ? { status: 'idle' } : { status: 'connected', user };
    if (force === 'waiting') return { status: 'waiting_for_discord' };
    if (force === 'failed') return { status: 'failed', message: 'Discord does not know this application ID. Check it in the Developer Portal.' };
    return { status: 'active', user };
  }

  const engineCbs = new Set<(s: EngineState) => void>();
  const appCbs = new Set<(s: AppState) => void>();
  const snapshot = (): AppState => clone({ engine, showing, profiles, settings, games, version: '0.1.0' });
  const emitEngine = () => engineCbs.forEach((f) => f(clone(engine)));
  const emitApp = () => appCbs.forEach((f) => f(snapshot()));
  const find = (id: string) => {
    const p = profiles.find((x) => x.id === id);
    if (!p) throw 'That profile no longer exists.';
    return p;
  };
  const nid = () => 'p-' + Math.random().toString(36).slice(2, 9);

  return {
    async getState() { await wait(60); return snapshot(); },
    async saveProfile(profile) {
      await wait(80);
      const i = profiles.findIndex((p) => p.id === profile.id);
      if (i < 0) throw 'That profile no longer exists.';
      profiles[i] = clone(profile);
      return validatePresence(profile.presence);
    },
    async createProfile(name) {
      await wait(60);
      const p: Profile = { id: nid(), name: name.trim() || 'New profile', presence: emptyPresence() };
      profiles.push(p);
      return clone(p);
    },
    async duplicateProfile(id) {
      await wait(60);
      const src = find(id);
      const p: Profile = { id: nid(), name: `${src.name} copy`, presence: clone(src.presence) };
      profiles.push(p);
      return clone(p);
    },
    async deleteProfile(id) {
      await wait(40);
      const i = profiles.findIndex((p) => p.id === id);
      if (i >= 0) profiles.splice(i, 1);
      if (settings.active_profile === id) settings.active_profile = null;
    },
    async setActiveProfile(id) { find(id); settings.active_profile = id; },
    async exportProfileToFile(id) {
      await wait(150);
      const p = find(id);
      return `~/Documents/${p.name.replace(/[^\w-]+/g, '-')}.decoy.json`;
    },
    async importProfile(json) {
      await wait(80);
      let raw: unknown;
      try { raw = JSON.parse(json); } catch { throw 'This file is not a valid Decoy profile: it is not JSON.'; }
      const r = raw as Partial<Profile>;
      if (!r || typeof r !== 'object' || (!r.name && !r.presence?.app_id)) throw 'This file is not a Decoy profile.';
      const p: Profile = { id: nid(), name: r.name || 'Imported profile', presence: { ...emptyPresence(), ...(r.presence ?? {}) } };
      profiles.push(p);
      return clone(p);
    },
    async setShowing(on) {
      showing = on;
      if (on) { engine = { status: 'connecting' }; emitEngine(); await wait(450); }
      engine = engineFor(on);
      emitEngine();
    },
    async validate(presence) { await wait(40); return validatePresence(presence); },
    async searchGames(query, limit) {
      await wait(110);
      const s = query.trim().toLowerCase();
      if (!s || games.count === 0) return [];
      const starts = GAMES.filter((g) => g.name.toLowerCase().startsWith(s));
      const inside = GAMES.filter((g) => !g.name.toLowerCase().startsWith(s) && g.name.toLowerCase().includes(s));
      return [...starts, ...inside].slice(0, limit);
    },
    async refreshGames() {
      games = { ...games, loading: true, error: null };
      emitApp();
      await wait(1600);
      games = { count: 24812, fetched_at: Math.floor(Date.now() / 1000), loading: false, error: null };
      emitApp();
    },
    async lookupApp(appId) {
      await wait(320);
      const id = appId.trim();
      if (!/^\d+$/.test(id)) throw 'Application ID must be digits only.';
      if (id === OWN_ID) return { id, name: 'Movie Night Club', icon_url: null } satisfies AppInfo;
      const g = GAMES.find((x) => x.id === id);
      if (g) return { id, name: g.name, icon_url: null };
      throw 'Discord has no application with this ID.';
    },
    async listAssets(appId) {
      await wait(260);
      return appId.trim() === OWN_ID ? clone(OWN_ASSETS) : [];
    },
    async setSettings(s) { settings = clone(s); },
    async openExternal(url) { window.open(url, '_blank', 'noopener'); },
    // ?update=0.2.0 shows the new version notice
    async checkUpdate() {
      const v = q.get('update');
      return v && !settings.skip_update_check ? { version: v, url: `https://github.com/mountain-beaver/decoy/releases/tag/v${v}` } : null;
    },
    async onEngineState(cb) { engineCbs.add(cb); return () => engineCbs.delete(cb); },
    async onAppState(cb) { appCbs.add(cb); return () => appCbs.delete(cb); },
    async onCloseRequest() { return () => {}; },
    async closeWindow() {},
  };
}
