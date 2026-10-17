// Fly navigation: a first-person way round the scene. While it is on, W A S D (or the arrow keys) walk forward,
// left, back and right, Q and E go down and up, Shift is faster and Alt slower, a drag in the viewport looks round
// from where the eye is, and Escape (or the command again) leaves it. The keys are taken before any shortcut sees
// them, so walking never runs a tool; every other key works as usual.

const KEYS: Record<string, [number, number, number]> = {
  KeyW: [1, 0, 0],
  ArrowUp: [1, 0, 0],
  KeyS: [-1, 0, 0],
  ArrowDown: [-1, 0, 0],
  KeyA: [0, -1, 0],
  ArrowLeft: [0, -1, 0],
  KeyD: [0, 1, 0],
  ArrowRight: [0, 1, 0],
  KeyQ: [0, 0, -1],
  KeyE: [0, 0, 1],
};

export interface FlyHooks {
  /** Moves the eye: forward, right and up in world units. */
  walk(forward: number, right: number, up: number): void;
  /** World units per second at normal speed (the app ties it to the view's distance, so it suits any scale). */
  speed(): number;
  /** Called when fly navigation starts or stops. */
  onToggle(on: boolean): void;
}

export class FlyNav {
  active = false;
  private readonly held = new Set<string>();
  private fast = false;
  private slow = false;
  private frame = 0;
  private last = 0;

  constructor(private readonly hooks: FlyHooks) {
    // Capture phase: the walking keys are taken before the shortcut handler or a focused control sees them.
    window.addEventListener('keydown', (e) => this.key(e, true), { capture: true });
    window.addEventListener('keyup', (e) => this.key(e, false), { capture: true });
    window.addEventListener('blur', () => this.held.clear());
  }

  set(on: boolean): void {
    if (on === this.active) return;
    this.active = on;
    this.held.clear();
    if (!on && this.frame) cancelAnimationFrame(this.frame);
    this.frame = 0;
    this.hooks.onToggle(on);
  }

  toggle(): void {
    this.set(!this.active);
  }

  private key(e: KeyboardEvent, down: boolean): void {
    if (!this.active) return;
    // Typing in a field (a name, a number) is left alone.
    const target = e.target as HTMLElement | null;
    if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.tagName === 'SELECT' || target.isContentEditable)) return;
    this.fast = e.shiftKey;
    this.slow = e.altKey;
    if (down && e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      this.set(false);
      return;
    }
    if (!(e.code in KEYS) || e.ctrlKey || e.metaKey) return;
    e.preventDefault();
    e.stopPropagation();
    if (down) this.held.add(e.code);
    else this.held.delete(e.code);
    if (this.held.size > 0 && !this.frame) {
      this.last = performance.now();
      this.frame = requestAnimationFrame((t) => this.step(t));
    }
  }

  /** One frame of walking: the held keys' directions summed, scaled by the time since the last frame. */
  private step(now: number): void {
    this.frame = 0;
    if (!this.active || this.held.size === 0) return;
    const dt = Math.min(0.1, (now - this.last) / 1000);
    this.last = now;
    let [f, r, u] = [0, 0, 0];
    for (const code of this.held) {
      const [a, b, c] = KEYS[code];
      f += a;
      r += b;
      u += c;
    }
    const len = Math.hypot(f, r, u) || 1;
    const v = (this.hooks.speed() * (this.fast ? 3 : this.slow ? 0.25 : 1) * dt) / len;
    if (f || r || u) this.hooks.walk(f * v, r * v, u * v);
    this.frame = requestAnimationFrame((t) => this.step(t));
  }
}
