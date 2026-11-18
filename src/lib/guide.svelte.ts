// Guide state: whether it is open, chapter and step (remembered), the application found for the showcase.
export type Chapter = 1 | 2 | 3 | 4;
const KEY = 'decoy.guide';
const MAX: Record<number, number> = { 1: 3, 2: 4 };

function lsGet(): { chapter?: number; steps?: Record<string, number> } {
  try { return JSON.parse(localStorage.getItem(KEY) ?? '{}') ?? {}; } catch { return {}; }
}
const clampInt = (v: unknown, lo: number, hi: number, def: number) => {
  const n = Math.trunc(Number(v));
  return Number.isFinite(n) && n >= lo && n <= hi ? n : def;
};

class GuideStore {
  open = $state(false);
  /** The reader has already switched something: only then do steps appear with an animation. */
  moved = $state(false);
  chapter = $state<Chapter>(1);
  steps = $state<Record<number, number>>({ 1: 1, 2: 1 });
  /** The application found by ID at step 4: the showcase displays its name and icon. */
  preview = $state<{ name: string; icon_url: string | null } | null>(null);

  constructor() {
    if (typeof location === 'undefined') return;
    const saved = lsGet();
    this.chapter = clampInt(saved.chapter, 1, 4, 1) as Chapter;
    for (const c of [1, 2]) this.steps[c] = clampInt(saved.steps?.[c], 1, MAX[c], 1);

    // ?open=guide&chapter=2&step=3 in the browser, VITE_OPEN="guide:2:3" in a build
    const q = new URLSearchParams(location.search);
    const env = (import.meta.env as Record<string, string | undefined>).VITE_OPEN;
    let spec: [unknown, unknown] | null = null;
    if (q.get('open') === 'guide') spec = [q.get('chapter'), q.get('step')];
    else if (env?.startsWith('guide')) { const [, c, s] = env.split(':'); spec = [c, s]; }
    if (spec) {
      this.open = true;
      this.chapter = clampInt(spec[0], 1, 4, this.chapter) as Chapter;
      if (MAX[this.chapter]) this.steps[this.chapter] = clampInt(spec[1], 1, MAX[this.chapter], 1);
    }
  }

  #save() {
    try { localStorage.setItem(KEY, JSON.stringify({ chapter: this.chapter, steps: this.steps })); } catch { /* not critical */ }
  }

  show(chapter?: Chapter) {
    if (chapter) this.chapter = chapter;
    this.open = true;
    this.#save();
  }
  close() { this.open = false; }
  go(chapter: Chapter) { this.moved = true; this.chapter = chapter; this.#save(); }
  step(chapter: number) { return this.steps[chapter] ?? 1; }
  setStep(chapter: number, n: number) {
    this.moved = true;
    this.steps[chapter] = Math.min(Math.max(n, 1), MAX[chapter] ?? 1);
    this.#save();
  }
}

export const guide = new GuideStore();
