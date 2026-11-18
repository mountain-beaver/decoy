import type { FieldId } from './types';

/** Length the way Discord checks it: UTF-16 units, so an emoji counts as two. */
export const chars = (s: string | null | undefined) => (s ? s.length : 0);

export const clean = (s: string | null | undefined) => {
  const t = s?.trim();
  return t ? t : null;
};

export const isUrl = (s: string) => /^https?:\/\//i.test(s);

const p2 = (n: number) => String(n).padStart(2, '0');

export function fmtElapsed(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}:${p2(m)}:${p2(s % 60)}` : `${p2(m)}:${p2(s % 60)}`;
}

export function fmtLeft(ms: number): string {
  return `${fmtElapsed(ms)} left`;
}

/** Milliseconds -> a value for input[type=datetime-local] in the local zone. */
export function toLocalInput(ms: number | null | undefined): string {
  if (ms == null) return '';
  const d = new Date(ms);
  return `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}T${p2(d.getHours())}:${p2(d.getMinutes())}`;
}

export function fromLocalInput(v: string): number | null {
  if (!v) return null;
  const t = new Date(v).getTime();
  return Number.isNaN(t) ? null : t;
}

export function hueOf(seed: string): number {
  let h = 7;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return (h * 47) % 360;
}

export function fmtDate(secs: number | null): string {
  if (!secs) return 'never';
  return new Date(secs * 1000).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

export const fmtCount = (n: number) => n.toLocaleString('en-US');

export function errMsg(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e && typeof e === 'object' && 'message' in e) return String((e as { message: unknown }).message);
  return 'Something went wrong. Try again.';
}

export function fieldKey(f: FieldId): string {
  if (typeof f === 'string') return f;
  if ('button_label' in f) return `button_label:${f.button_label}`;
  return `button_url:${f.button_url}`;
}

export const isAppId = (s: string) => /^\d{17,20}$/.test(s.trim());

/** A Vesktop/arRPC bridge instead of the real account: its name must not be shown. */
export const isBridgeUser = (u: { id: string; username: string }) =>
  u.username === 'arrpc' || u.id === '1045800378228281345';

export const userName = (u: { id: string; username: string; global_name?: string | null }) =>
  isBridgeUser(u) ? '' : u.global_name || u.username;
