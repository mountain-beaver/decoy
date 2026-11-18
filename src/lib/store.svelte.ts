import { untrack } from 'svelte';
import { backend } from './backend';
import type {
  AppState, AssetInfo, EngineState, GameHit, GamesInfo, Issue, Presence, Profile, Settings,
} from './types';
import { errMsg, fieldKey, isAppId } from './util';

export type AppEntry = {
  state: 'loading' | 'ok' | 'error';
  name?: string;
  icon_url?: string | null;
  origin?: 'list' | 'own';
  message?: string;
};
export type Toast = { id: number; text: string; kind: 'error' | 'ok' };

export const ACCENTS = [
  { id: 'highlighter', name: 'Highlighter', color: '#FFD84D' },
  { id: 'coral', name: 'Coral', color: '#FF8FA3' },
  { id: 'aqua', name: 'Aqua', color: '#6FE0D2' },
  { id: 'silver', name: 'Silver', color: '#C9CED6' },
] as const;
export type AccentId = (typeof ACCENTS)[number]['id'];
export type ThemePref = 'system' | 'light' | 'dark';

function lsGet(key: string): string | null {
  try { return localStorage.getItem(key); } catch { return null; }
}
function lsSet(key: string, value: string) {
  try { localStorage.setItem(key, value); } catch { /* storage is unavailable, not critical */ }
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));

class AppStore {
  ready = $state(false);
  loadError = $state<string | null>(null);
  engine = $state<EngineState>({ status: 'idle' });
  showing = $state(false);
  profiles = $state<Profile[]>([]);
  settings = $state<Settings>({ active_profile: null, connect_on_launch: true, start_in_tray: false, autostart: false });
  games = $state<GamesInfo>({ count: 0, fetched_at: null, loading: false, error: null });
  version = $state('');

  draft = $state<Profile | null>(null);
  issues = $state<Issue[]>([]);
  saved = $state(false);

  apps = $state<Record<string, AppEntry>>({});
  assets = $state<Record<string, AssetInfo[]>>({});

  toasts = $state<Toast[]>([]);
  settingsOpen = $state(false);

  now = $state(Date.now());
  showSince = $state(Date.now());
  changedAt = $state(Date.now());

  accent = $state<AccentId>('highlighter');
  theme = $state<ThemePref>('system');

  #baseline = '';
  #saveTimer: ReturnType<typeof setTimeout> | undefined;
  #valTimer: ReturnType<typeof setTimeout> | undefined;
  #vseq = 0;
  #toastId = 0;
  #savedTimer: ReturnType<typeof setTimeout> | undefined;
  #started = false;

  /** Presence of the active profile; call only when a draft exists. */
  get p(): Presence {
    return this.draft!.presence;
  }

  get active(): Profile | null {
    return this.draft;
  }

  get busyIssues() {
    return this.issues.length;
  }

  // ---------- startup ----------
  init() {
    if (this.#started) return;
    this.#started = true;
    const url = new URLSearchParams(location.search);
    const acc = url.get('accent') ?? lsGet('decoy.accent');
    if (ACCENTS.some((a) => a.id === acc)) this.accent = acc as AccentId;
    const th = url.get('theme') ?? lsGet('decoy.theme');
    if (th === 'light' || th === 'dark') this.theme = th;
    this.#applyLook();

    $effect.root(() => {
      $effect(() => {
        const d = this.draft;
        if (!d) return;
        const json = JSON.stringify(d);
        untrack(() => this.#onDraft(json));
      });
      $effect(() => {
        this.accent;
        this.theme;
        untrack(() => this.#applyLook());
      });
    });

    // one shared stopwatch, paused while the window is hidden
    let ticker: ReturnType<typeof setInterval> | undefined;
    const run = () => {
      this.now = Date.now();
      if (ticker === undefined && !document.hidden) ticker = setInterval(() => (this.now = Date.now()), 1000);
    };
    const stop = () => { if (ticker !== undefined) { clearInterval(ticker); ticker = undefined; } };
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) { stop(); void this.flush(); } else run();
    });
    run();

    void backend.onEngineState((s) => { this.engine = s; });
    void backend.onAppState((s) => { void this.#applyState(s); });
    void this.load();
  }

  async load() {
    this.loadError = null;
    try {
      await this.#applyState(await backend.getState());
      this.ready = true;
    } catch (e) {
      this.loadError = errMsg(e);
    }
  }

  #applyLook() {
    const r = document.documentElement;
    const a = ACCENTS.find((x) => x.id === this.accent)!;
    r.style.setProperty('--accent', a.color);
    if (this.theme === 'system') r.removeAttribute('data-theme');
    else r.setAttribute('data-theme', this.theme);
  }

  setAccent(id: AccentId) {
    this.accent = id;
    lsSet('decoy.accent', id);
  }
  setTheme(t: ThemePref) {
    this.theme = t;
    lsSet('decoy.theme', t);
  }

  async #applyState(s: AppState) {
    const wasShowing = this.showing;
    this.engine = s.engine;
    this.showing = s.showing;
    if (s.showing && !wasShowing) this.showSince = Date.now();
    this.settings = s.settings;
    this.games = s.games;
    this.version = s.version;
    // outside changes do not overwrite what the user is typing right now
    const keepDraft = this.draft && JSON.stringify(this.draft) !== this.#baseline;
    const activeId = s.settings.active_profile && s.profiles.some((p) => p.id === s.settings.active_profile)
      ? s.settings.active_profile
      : (s.profiles[0]?.id ?? null);
    this.profiles = s.profiles;
    if (keepDraft && this.draft!.id === activeId) {
      // keep the draft as it is
    } else if (!this.draft || this.draft.id !== activeId || JSON.stringify(s.profiles.find((p) => p.id === activeId)) !== this.#baseline) {
      this.#loadDraft(s.profiles.find((p) => p.id === activeId) ?? null);
    }
    if (activeId && s.settings.active_profile !== activeId) {
      this.settings = { ...this.settings, active_profile: activeId };
      void backend.setActiveProfile(activeId).catch(() => {});
    }
  }

  // ---------- draft, validation, autosave ----------
  #loadDraft(p: Profile | null) {
    clearTimeout(this.#saveTimer);
    clearTimeout(this.#valTimer);
    this.issues = [];
    if (!p) {
      this.draft = null;
      this.#baseline = '';
      return;
    }
    const c = clone(p);
    this.#baseline = JSON.stringify(c);
    this.draft = c;
    this.changedAt = Date.now();
    this.#validateNow();
    this.ensureApp(c.presence.app_id);
  }

  #onDraft(json: string) {
    if (json === this.#baseline) return;
    this.changedAt = Date.now();
    clearTimeout(this.#valTimer);
    this.#valTimer = setTimeout(() => this.#validateNow(), 220);
    clearTimeout(this.#saveTimer);
    this.#saveTimer = setTimeout(() => void this.#saveNow(), 500);
  }

  #validateNow() {
    if (!this.draft) return;
    const presence = $state.snapshot(this.draft.presence) as Presence;
    const seq = ++this.#vseq;
    backend.validate(presence).then((r) => { if (seq === this.#vseq) this.issues = r; }).catch(() => {});
  }

  async #saveNow() {
    if (!this.draft) return;
    const snap = $state.snapshot(this.draft) as Profile;
    const json = JSON.stringify(snap);
    if (json === this.#baseline) return;
    try {
      const issues = await backend.saveProfile(snap);
      this.#baseline = json;
      this.profiles = this.profiles.map((p) => (p.id === snap.id ? clone(snap) : p));
      if (this.draft && JSON.stringify(this.draft) === json) this.issues = issues;
      this.saved = true;
      clearTimeout(this.#savedTimer);
      this.#savedTimer = setTimeout(() => (this.saved = false), 1800);
    } catch (e) {
      this.toast(`Could not save your changes. ${errMsg(e)}`, 'error');
    }
  }

  async flush() {
    if (this.#saveTimer === undefined) return;
    clearTimeout(this.#saveTimer);
    this.#saveTimer = undefined;
    await this.#saveNow();
  }

  issuesFor(...keys: string[]): Issue[] {
    return this.issues.filter((i) => keys.includes(fieldKey(i.field)));
  }

  // ---------- profiles ----------
  async switchProfile(id: string) {
    if (this.draft?.id === id) return;
    await this.flush();
    try {
      await backend.setActiveProfile(id);
      this.settings = { ...this.settings, active_profile: id };
      this.#loadDraft(this.profiles.find((p) => p.id === id) ?? null);
    } catch (e) { this.toast(errMsg(e), 'error'); }
  }

  async createProfile(name: string): Promise<Profile | null> {
    await this.flush();
    try {
      const p = await backend.createProfile(name);
      this.profiles = [...this.profiles, p];
      await backend.setActiveProfile(p.id);
      this.settings = { ...this.settings, active_profile: p.id };
      this.#loadDraft(p);
      return p;
    } catch (e) { this.toast(errMsg(e), 'error'); return null; }
  }

  async duplicateActive() {
    if (!this.draft) return;
    await this.flush();
    try {
      const p = await backend.duplicateProfile(this.draft.id);
      this.profiles = [...this.profiles, p];
      await this.switchProfile(p.id);
      this.toast(`Duplicated as "${p.name}".`, 'ok');
    } catch (e) { this.toast(errMsg(e), 'error'); }
  }

  renameActive(name: string) {
    if (this.draft && name.trim()) this.draft.name = name.trim();
  }

  async deleteActive() {
    if (!this.draft) return;
    const { id, name } = this.draft;
    clearTimeout(this.#saveTimer);
    this.#saveTimer = undefined;
    try {
      await backend.deleteProfile(id);
      this.profiles = this.profiles.filter((p) => p.id !== id);
      const next = this.profiles[0] ?? null;
      if (next) {
        await backend.setActiveProfile(next.id);
        this.settings = { ...this.settings, active_profile: next.id };
      } else {
        this.settings = { ...this.settings, active_profile: null };
      }
      this.#loadDraft(next);
      this.toast(`Deleted "${name}".`, 'ok');
    } catch (e) { this.toast(errMsg(e), 'error'); }
  }

  async exportActive() {
    if (!this.draft) return;
    await this.flush();
    try {
      const path = await backend.exportProfileToFile(this.draft.id);
      if (path) this.toast(`Saved to ${path}`, 'ok');
    } catch (e) { this.toast(errMsg(e), 'error'); }
  }

  async importFile(file: File) {
    try {
      const json = await file.text();
      await this.flush();
      const p = await backend.importProfile(json);
      this.profiles = [...this.profiles, p];
      await this.switchProfile(p.id);
      this.toast(`Imported "${p.name}".`, 'ok');
    } catch (e) { this.toast(errMsg(e), 'error'); }
  }

  // ---------- main switch ----------
  async setShowing(on: boolean) {
    const prev = this.showing;
    this.showing = on;
    if (on) this.showSince = Date.now();
    try {
      await this.flush();
      await backend.setShowing(on);
    } catch (e) {
      this.showing = prev;
      this.toast(errMsg(e), 'error');
    }
  }

  // ---------- applications and games ----------
  entry(appId: string): AppEntry | undefined {
    return this.apps[appId.trim()];
  }

  /** Look up an application's name and icon; the result is cached. */
  async lookup(appId: string, origin?: 'list' | 'own'): Promise<AppEntry> {
    const id = appId.trim();
    if (!isAppId(id)) return { state: 'error', message: 'Application ID must be 17 to 20 digits.' };
    const cur = this.apps[id];
    if (cur && cur.state !== 'error') return cur;
    this.apps[id] = { state: 'loading' };
    try {
      const info = await backend.lookupApp(id);
      let o = origin;
      if (!o) {
        try {
          const hits = await backend.searchGames(info.name, 8);
          o = hits.some((h) => h.id === id) ? 'list' : this.games.count > 0 ? 'own' : undefined;
        } catch { /* could not learn the source, that is fine */ }
      }
      this.apps[id] = { state: 'ok', name: info.name, icon_url: info.icon_url, origin: o };
    } catch (e) {
      this.apps[id] = { state: 'error', message: errMsg(e) };
    }
    return this.apps[id];
  }

  ensureApp(appId: string) {
    if (isAppId(appId)) {
      void this.lookup(appId).then((e) => {
        if (e.origin === 'own') void this.loadAssets(appId.trim());
      });
    }
  }

  async loadAssets(appId: string) {
    if (this.assets[appId]) return;
    try {
      this.assets[appId] = await backend.listAssets(appId);
    } catch {
      this.assets[appId] = [];
    }
  }

  async pickGame(hit: GameHit, origin: 'list' | 'own' = 'list') {
    if (!this.draft) {
      const p = await this.createProfile(hit.name);
      if (!p) return;
    }
    const d = this.draft!;
    this.apps[hit.id] = { state: 'ok', name: hit.name, icon_url: hit.icon_url, origin };
    if (origin === 'own') void this.loadAssets(hit.id);
    d.presence.app_id = hit.id;
    if (!d.name.trim() || d.name === 'New profile') d.name = hit.name;
  }

  async refreshGames() {
    this.games = { ...this.games, loading: true, error: null };
    try {
      await backend.refreshGames();
    } catch (e) {
      this.games = { ...this.games, loading: false, error: errMsg(e) };
      return;
    }
    // a fallback in case the app-state event never arrives
    for (let i = 0; i < 40 && this.games.loading; i++) {
      await new Promise((r) => setTimeout(r, 1500));
      try { this.games = (await backend.getState()).games; } catch { break; }
    }
  }

  async updateSettings(patch: Partial<Settings>) {
    const prev = this.settings;
    this.settings = { ...prev, ...patch };
    try { await backend.setSettings($state.snapshot(this.settings) as Settings); } catch (e) {
      this.settings = prev;
      this.toast(errMsg(e), 'error');
    }
  }

  openExternal(url: string) {
    void backend.openExternal(url).catch((e) => this.toast(errMsg(e), 'error'));
  }

  toast(text: string, kind: 'error' | 'ok' = 'ok') {
    const id = ++this.#toastId;
    this.toasts = [...this.toasts.slice(-3), { id, text, kind }];
    setTimeout(() => this.dismiss(id), kind === 'error' ? 7000 : 3500);
  }
  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const app = new AppStore();
