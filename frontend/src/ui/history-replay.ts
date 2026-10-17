// History depth in the window: the limits and switches kept in this browser, step sizes as text, and the replay that
// walks the undo history from the start to the current step, each step seen from the camera it was made from.

import type { CameraPose } from '../viewport/camera';

export const HISTORY_STORE = 'formsculptor.history';

export interface HistoryPrefs {
  /** Undo steps the engine keeps (set_history_limits). */
  maxSteps: number;
  /** Memory the steps may keep together, in MB; 0 is no limit. */
  maxMemoryMb: number;
  /** Record the view with each step, and go to it when a step is chosen or replayed. */
  cameraPerStep: boolean;
  /** Replay speed in steps a second. */
  replayRate: number;
}

export const DEFAULT_HISTORY: HistoryPrefs = { maxSteps: 200, maxMemoryMb: 0, cameraPerStep: true, replayRate: 2 };

export const REPLAY_RATES: [string, string][] = [['1', 'Slow'], ['2', 'Normal'], ['4', 'Fast'], ['8', 'Very fast']];

const whole = (v: unknown, lo: number, hi: number, fallback: number): number =>
  typeof v === 'number' && Number.isInteger(v) && v >= lo && v <= hi ? v : fallback;

/** Prefs from stored JSON; anything missing or out of range takes its default. */
export function parseHistoryPrefs(raw: unknown): HistoryPrefs {
  const o = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  return {
    maxSteps: whole(o.maxSteps, 1, 10000, DEFAULT_HISTORY.maxSteps),
    maxMemoryMb: whole(o.maxMemoryMb, 0, 65536, DEFAULT_HISTORY.maxMemoryMb),
    cameraPerStep: typeof o.cameraPerStep === 'boolean' ? o.cameraPerStep : DEFAULT_HISTORY.cameraPerStep,
    replayRate: REPLAY_RATES.some(([v]) => Number(v) === o.replayRate) ? (o.replayRate as number) : DEFAULT_HISTORY.replayRate,
  };
}

export function loadHistoryPrefs(): HistoryPrefs {
  try {
    return parseHistoryPrefs(JSON.parse(localStorage.getItem(HISTORY_STORE) ?? 'null'));
  } catch {
    return { ...DEFAULT_HISTORY };
  }
}

export function saveHistoryPrefs(p: HistoryPrefs): void {
  try {
    localStorage.setItem(HISTORY_STORE, JSON.stringify(p));
  } catch {
    // Storage blocked: the prefs last until the page closes.
  }
}

/** A step's memory as short text: bytes, KB, MB or GB. */
export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(n < 10 * 1024 ** 2 ? 1 : 0)} MB`;
  return `${(n / 1024 ** 3).toFixed(1)} GB`;
}

/** The pose `t` of the way from `a` to `b`: the target straight, the distance by ratio, yaw the short way round. */
export function blendPose(a: CameraPose, b: CameraPose, t: number): CameraPose {
  const lerp = (x: number, y: number): number => x + (y - x) * t;
  let dyaw = (b.yaw - a.yaw) % 360;
  if (dyaw > 180) dyaw -= 360;
  if (dyaw < -180) dyaw += 360;
  return {
    target: [lerp(a.target[0], b.target[0]), lerp(a.target[1], b.target[1]), lerp(a.target[2], b.target[2])],
    distance: a.distance * Math.pow(b.distance / a.distance, t),
    yaw: a.yaw + dyaw * t,
    pitch: lerp(a.pitch, b.pitch),
    ortho: t < 0.5 ? a.ortho : b.ortho,
  };
}

export function samePose(a: CameraPose, b: CameraPose): boolean {
  const near = (x: number, y: number, e: number): boolean => Math.abs(x - y) <= e;
  return a.ortho === b.ortho && near(a.distance, b.distance, a.distance * 1e-3) && near(a.yaw, b.yaw, 0.05) && near(a.pitch, b.pitch, 0.05) && a.target.every((v, i) => near(v, b.target[i], a.distance * 1e-3));
}

export interface ReplayHooks {
  /** Goes to a step (`done` steps done) and resolves with the camera it was made from, if recorded. */
  goTo(done: number): Promise<CameraPose | null>;
  pose(): CameraPose;
  setPose(p: CameraPose): void;
  /** Steps a second. */
  rate(): number;
  /** Whether to follow each step's camera. */
  followCamera(): boolean;
  /** Called as the replay moves and when it stops (playing false). */
  onState(playing: boolean, at: number, of: number): void;
}

/** Plays the history from the start up to the step that was current, then leaves it there; stopping early goes
 * back to that step too, so a replay never leaves the history part undone (a new stroke would drop the rest). */
export class HistoryReplay {
  private run = 0;
  playing = false;

  constructor(private readonly hooks: ReplayHooks) {}

  async play(end: number): Promise<void> {
    if (this.playing || end < 1) return;
    const run = ++this.run;
    this.playing = true;
    const start = this.hooks.pose();
    let reached = -1;
    try {
      for (let k = 0; k <= end && run === this.run; k++) {
        const began = performance.now();
        const interval = 1000 / Math.max(0.1, this.hooks.rate());
        const camera = await this.hooks.goTo(k);
        reached = k;
        this.hooks.onState(true, k, end);
        if (camera && this.hooks.followCamera()) await this.glide(camera, Math.min(400, interval * 0.6), run);
        const rest = interval - (performance.now() - began);
        if (rest > 0 && k < end) await new Promise((r) => setTimeout(r, rest));
      }
    } finally {
      if (reached !== end) {
        await this.hooks.goTo(end);
        this.hooks.setPose(start);
      }
      this.playing = false;
      this.hooks.onState(false, end, end);
    }
  }

  /** Stops at the next step; `playing` stays true until the history is back where it was, so a new replay cannot
   * start over the old one's return. */
  stop(): void {
    if (this.playing) this.run++;
  }

  /** Eases the view to a step's camera outside a replay (a step chosen in the list). */
  ease(to: CameraPose): void {
    void this.glide(to, 350, this.run);
  }

  /** Eases the view to `to` over `ms`, giving up if the replay is stopped. */
  private glide(to: CameraPose, ms: number, run: number): Promise<void> {
    const from = this.hooks.pose();
    if (samePose(from, to) || ms <= 0) {
      this.hooks.setPose(to);
      return Promise.resolve();
    }
    return new Promise((resolve) => {
      const t0 = performance.now();
      const frame = (now: number): void => {
        if (run !== this.run) return resolve();
        const t = Math.min(1, (now - t0) / ms);
        const e = t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
        this.hooks.setPose(blendPose(from, to, e));
        if (t < 1) requestAnimationFrame(frame);
        else resolve();
      };
      requestAnimationFrame(frame);
    });
  }
}
