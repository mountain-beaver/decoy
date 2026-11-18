// Data shapes mirror the serde types in crates/decoy-core.

export type ActivityKind = 'playing' | 'listening' | 'watching' | 'competing';
export type StatusDisplay = 'name' | 'state' | 'details';

export interface Party {
  current: number;
  max: number;
}

export type TimestampMode =
  | { mode: 'none' }
  | { mode: 'since_connect' }
  | { mode: 'since_update' }
  | { mode: 'local_time' }
  | { mode: 'custom'; start_ms?: number | null; end_ms?: number | null };

export interface ImageSpec {
  source: string;
  text?: string | null;
  url?: string | null;
}

export interface Button {
  label: string;
  url: string;
}

export interface Presence {
  app_id: string;
  kind: ActivityKind;
  name: string | null;
  details: string | null;
  details_url: string | null;
  state: string | null;
  state_url: string | null;
  status_display: StatusDisplay;
  party: Party | null;
  timestamps: TimestampMode;
  large_image: ImageSpec | null;
  small_image: ImageSpec | null;
  buttons: Button[];
}

export type FieldName =
  | 'app_id' | 'name' | 'details' | 'details_url' | 'state' | 'state_url' | 'party' | 'timestamps'
  | 'large_image' | 'large_image_text' | 'large_image_url'
  | 'small_image' | 'small_image_text' | 'small_image_url' | 'buttons';
export type FieldId = FieldName | { button_label: number } | { button_url: number };

export interface Issue {
  field: FieldId;
  message: string;
}

export interface Profile {
  id: string;
  name: string;
  presence: Presence;
}

export interface Settings {
  active_profile: string | null;
  connect_on_launch: boolean;
  start_in_tray: boolean;
  autostart: boolean;
}

export interface DiscordUser {
  id: string;
  username: string;
  global_name?: string | null;
  avatar?: string | null;
}

export type EngineState =
  | { status: 'idle' }
  | { status: 'connecting' }
  | { status: 'waiting_for_discord' }
  | { status: 'active'; user: DiscordUser }
  | { status: 'connected'; user: DiscordUser }
  | { status: 'failed'; message: string };

export interface GamesInfo {
  count: number;
  fetched_at: number | null;
  loading: boolean;
  error: string | null;
}

export interface AppState {
  engine: EngineState;
  showing: boolean;
  profiles: Profile[];
  settings: Settings;
  games: GamesInfo;
  version: string;
}

export interface GameHit {
  id: string;
  name: string;
  icon_url: string | null;
}

export interface AppInfo {
  id: string;
  name: string;
  icon_url: string | null;
}

export interface AssetInfo {
  id: string;
  name: string;
  url: string;
}

export function emptyPresence(): Presence {
  return {
    app_id: '',
    kind: 'playing',
    name: null,
    details: null,
    details_url: null,
    state: null,
    state_url: null,
    status_display: 'name',
    party: null,
    timestamps: { mode: 'none' },
    large_image: null,
    small_image: null,
    buttons: [],
  };
}
