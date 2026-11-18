// The single link to Rust: Tauri commands, or the mock in a browser.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppInfo, AppState, AssetInfo, EngineState, GameHit, Issue, Presence, Profile, Settings,
} from './types';
import { createMock } from './mock';

export interface Backend {
  getState(): Promise<AppState>;
  saveProfile(profile: Profile): Promise<Issue[]>;
  createProfile(name: string): Promise<Profile>;
  duplicateProfile(id: string): Promise<Profile>;
  deleteProfile(id: string): Promise<void>;
  setActiveProfile(id: string): Promise<void>;
  exportProfileToFile(id: string): Promise<string | null>;
  importProfile(json: string): Promise<Profile>;
  setShowing(on: boolean): Promise<void>;
  validate(presence: Presence): Promise<Issue[]>;
  searchGames(query: string, limit: number): Promise<GameHit[]>;
  refreshGames(): Promise<void>;
  lookupApp(appId: string): Promise<AppInfo>;
  listAssets(appId: string): Promise<AssetInfo[]>;
  setSettings(settings: Settings): Promise<void>;
  openExternal(url: string): Promise<void>;
  onEngineState(cb: (s: EngineState) => void): Promise<() => void>;
  onAppState(cb: (s: AppState) => void): Promise<() => void>;
}

function createTauri(): Backend {
  const call = <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);
  return {
    getState: () => call('get_state'),
    saveProfile: (profile) => call('save_profile', { profile }),
    createProfile: (name) => call('create_profile', { name }),
    duplicateProfile: (id) => call('duplicate_profile', { id }),
    deleteProfile: (id) => call('delete_profile', { id }),
    setActiveProfile: (id) => call('set_active_profile', { id }),
    exportProfileToFile: (id) => call('export_profile_to_file', { id }),
    importProfile: (json) => call('import_profile', { json }),
    setShowing: (on) => call('set_showing', { on }),
    validate: (presence) => call('validate', { presence }),
    searchGames: (query, limit) => call('search_games', { query, limit }),
    refreshGames: () => call('refresh_games'),
    lookupApp: (appId) => call('lookup_app', { appId }),
    listAssets: (appId) => call('list_assets', { appId }),
    setSettings: (settings) => call('set_settings', { settings }),
    openExternal: (url) => call('open_external', { url }),
    onEngineState: (cb) => listen<EngineState>('engine-state', (e) => cb(e.payload)),
    onAppState: (cb) => listen<AppState>('app-state', (e) => cb(e.payload)),
  };
}

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export const backend: Backend = inTauri ? createTauri() : createMock();
