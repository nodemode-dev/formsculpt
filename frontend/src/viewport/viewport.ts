import type { MeshBuffers, VertexUpdate } from '../engine/service';
import { OrbitCamera, type ViewName } from './camera';
import { GlRenderer, hexColor } from './gl-renderer';
import type { Vec3 } from './math';

export interface ViewportStats {
  renderer: string;
  frameMs: number;
}

export interface Ray {
  origin: Vec3;
  dir: Vec3;
}

export interface StrokeHandler {
  enabled(): boolean;
  begin(ray: Ray, pressure: number, worldPerPixel: number, invert: boolean, tilt: number): void;
  move(ray: Ray, pressure: number, tilt: number): void;
  end(): void;
  missed(): boolean;
}

export class Viewport {
  readonly camera = new OrbitCamera();
  readonly renderer: GlRenderer;
  private canvas: HTMLCanvasElement;
  private dirty = true;
  private frame = 0;
  private lastFrameMs = 0;
  private drag: {
    button: number;
    x: number;
    y: number;
    mode: 'camera' | 'stroke' | 'pending';
    /** The modifiers held at the press, and how far the pointer has gone since (the Nomad and ZBrush schemes). */
    alt?: boolean;
    shift?: boolean;
    ctrl?: boolean;
    moved?: number;
    /** The ZBrush scheme's Alt+click on the model: a press that picks the object unless it moves. */
    pick?: boolean;
    /** Shift's 90 degree snapping in the ZBrush scheme: the turn the drag would have without it. */
    free?: { yaw: number; pitch: number };
  } | null = null;

  /**
   * The pointer conventions of the layout in use. 'panel' is the window's own. 'nomad' follows Nomad Sculpt's manual:
   * Alt+drag moves the view and releasing Alt during the drag zooms; a two-finger tap undoes and a three-finger tap
   * redoes. 'zbrush' follows ZBrush's Starting Guide and shortcut sheet: Alt sculpts inverted (Zsub), Shift smooths,
   * Ctrl masks (Ctrl+Alt unmasks), Alt+click picks the SubTool under the pointer; on the background a drag rotates
   * (Shift snaps to 90 degrees), Alt+drag moves, releasing Alt during the drag zooms, Ctrl+click inverts the mask and
   * Ctrl+drag clears it.
   */
  scheme: 'panel' | 'nomad' | 'zbrush' = 'panel';
  /** Set by the app: hold a tool for one stroke (smooth or mask); null gives the previous tool back. */
  holdTool: ((kind: string | null) => void) | null = null;
  /** Set by the app: pick the object under a ray (the ZBrush scheme's Alt+click). */
  pickAt: ((ray: Ray) => void) | null = null;
  /** Set by the app: a ZBrush Ctrl+click (blur the mask on the model, sharpen it with Alt, invert it off the model). */
  ctrlClick: ((ray: Ray, alt: boolean) => void) | null = null;
  /** Set by the app: whether a tap is a mask shortcut now (Nomad's One tap shortcuts with the Mask tool in hand). */
  tapsMask: (() => boolean) | null = null;
  /** Set by the app: a mask shortcut tap (invert off the model, blur on a masked area, sharpen on an unmasked one). */
  maskTap: ((ray: Ray) => void) | null = null;
  /** Set by the app: run a registry command (undo, redo, the mask's invert and clear). */
  command: ((id: string) => void) | null = null;
  private holding = false;
  /** Fingers' taps: when the first finger landed, the most fingers down at once, and whether they moved. */
  private tap: { at: number; most: number; moved: boolean } | null = null;
  onFrame: ((stats: ViewportStats) => void) | null = null;
  /** Set by the app: the context came back after a loss, so images given to the renderer must be given again. */
  onRestore: (() => void) | null = null;
  strokes: StrokeHandler | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.renderer = new GlRenderer(canvas);
    const css = getComputedStyle(document.body);
    const bg = css.getPropertyValue('--canvas-bg');
    if (bg) this.renderer.setClearColor(bg);
    const b = this.renderer.backdrop;
    b.top = hexColor(css.getPropertyValue('--viewport-top')) ?? b.top;
    b.bottom = hexColor(css.getPropertyValue('--viewport-bottom')) ?? b.bottom;
    b.gridColor = hexColor(css.getPropertyValue('--viewport-grid')) ?? b.gridColor;
    new ResizeObserver(() => this.resize()).observe(canvas);
    const watchRatio = (): void => {
      matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`).addEventListener('change', () => {
        this.resize();
        watchRatio();
      }, { once: true });
    };
    watchRatio();
    canvas.addEventListener('webglcontextlost', (e) => e.preventDefault());
    canvas.addEventListener('webglcontextrestored', () => {
      this.renderer.restore();
      this.onRestore?.();
      this.invalidate();
    });
    canvas.addEventListener('pointerdown', (e) => this.down(e));
    canvas.addEventListener('pointermove', (e) => this.move(e));
    canvas.addEventListener('pointerup', (e) => this.up(e));
    canvas.addEventListener('pointercancel', (e) => this.up(e));
    canvas.addEventListener('wheel', (e) => this.wheel(e), { passive: false });
    canvas.addEventListener('contextmenu', (e) => e.preventDefault());
    this.resize();
  }

  private resize(): void {
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const w = Math.max(1, Math.round(this.canvas.clientWidth * dpr));
    const h = Math.max(1, Math.round(this.canvas.clientHeight * dpr));
    const sized = this.canvas.width !== w || this.canvas.height !== h;
    if (sized) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.camera.aspect = w / h;
    this.invalidate();
    if (sized) this.draw();
  }

  invalidate(): void {
    this.dirty = true;
    if (!this.frame) this.frame = requestAnimationFrame(() => this.draw());
  }

  private draw(): void {
    this.frame = 0;
    if (!this.dirty) return;
    this.dirty = false;
    const t0 = performance.now();
    this.renderer.render(this.camera, this.canvas.width, this.canvas.height);
    this.lastFrameMs = performance.now() - t0;
    this.onFrame?.({ renderer: this.renderer.kind, frameMs: this.lastFrameMs });
  }

  setMesh(mesh: MeshBuffers): void {
    this.renderer.upload(mesh);
    this.invalidate();
  }

  patchMesh(update: VertexUpdate): void {
    if (this.renderer.patch(update)) this.invalidate();
  }

  removeMesh(id: number): void {
    this.renderer.remove(id);
    this.invalidate();
  }

  setActive(id: number | null): void {
    this.renderer.active = id;
    this.invalidate();
  }

  frameBounds(min: Vec3, max: Vec3): void {
    this.camera.frame(min, max);
    this.invalidate();
  }

  snap(view: ViewName): void {
    this.camera.snap(view);
    this.invalidate();
  }

  /** The home view, framing the given bounds (everything shown) when there are any. */
  home(min?: Vec3, max?: Vec3): void {
    this.camera.home();
    if (min && max) this.camera.frame(min, max);
    this.invalidate();
  }

  /** Turns the view by a drag of dx, dy pixels (the nav cube's drag). */
  orbit(dx: number, dy: number): void {
    this.camera.orbit(dx, dy);
    this.invalidate();
  }

  toggleOrtho(): boolean {
    this.camera.orthographic = !this.camera.orthographic;
    this.invalidate();
    return this.camera.orthographic;
  }

  /** A ray through a point given in client pixels (for picking on double-click). */
  rayAtClient(x: number, y: number): Ray | null {
    const r = this.canvas.getBoundingClientRect();
    return this.camera.ray(x - r.left, y - r.top, r.width, r.height);
  }

  /** Orbit around `point` from here on, without moving the eye: the view does not jump. */
  pivotAt(point: Vec3): void {
    this.camera.pivotAt(point);
    this.invalidate();
  }

  setFov(degrees: number): void {
    this.camera.fov = (Math.max(10, Math.min(120, degrees)) * Math.PI) / 180;
    this.invalidate();
  }

  private rayAt(e: PointerEvent): Ray | null {
    const r = this.canvas.getBoundingClientRect();
    return this.camera.ray(e.clientX - r.left, e.clientY - r.top, r.width, r.height);
  }

  private pressure(e: PointerEvent): number {
    return e.pointerType === 'mouse' ? 1 : Math.max(0.05, e.pressure || 0.5);
  }

  /** The direction a pen leans, in radians from the screen's up (0 when upright, or for a mouse or finger). */
  private tilt(e: PointerEvent): number {
    if (e.pointerType !== 'pen' || (Math.abs(e.tiltX) < 3 && Math.abs(e.tiltY) < 3)) return 0;
    return Math.atan2(e.tiltX, -e.tiltY);
  }

  /** Fingers on the canvas, by pointer id, and the last two-finger span and midpoint while a gesture runs. */
  private touches = new Map<number, [number, number]>();
  private gesture: { span: number; mid: [number, number] } | null = null;
  /** The last three-finger midpoint while three fingers drag. */
  private three: [number, number] | null = null;
  /** Nomad's Finger always smooths: a one-finger stroke uses Smooth whatever the tool. */
  fingerSmooths = false;
  /** Set by the app: a three-finger drag by dx, dy pixels (Nomad's three-finger shortcuts). */
  threeFinger: ((dx: number, dy: number) => void) | null = null;

  private threeMid(): [number, number] {
    const all = [...this.touches.values()];
    return [all.reduce((s, p) => s + p[0], 0) / all.length, all.reduce((s, p) => s + p[1], 0) / all.length];
  }

  private gestureFrame(): { span: number; mid: [number, number] } {
    const [a, b] = [...this.touches.values()];
    return { span: Math.hypot(a[0] - b[0], a[1] - b[1]), mid: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2] };
  }

  private down(e: PointerEvent): void {
    if (e.pointerType === 'touch') {
      this.touches.set(e.pointerId, [e.clientX, e.clientY]);
      if (this.touches.size === 1) this.tap = { at: performance.now(), most: 1, moved: false };
      else if (this.tap) this.tap.most = Math.max(this.tap.most, this.touches.size);
      if (this.touches.size === 2) {
        // A second finger ends any stroke the first one began and starts pinch to zoom and two-finger pan.
        if (this.drag?.mode === 'stroke') this.strokes?.end();
        this.drag = null;
        this.gesture = this.gestureFrame();
        this.canvas.setPointerCapture(e.pointerId);
        return;
      }
      if (this.touches.size === 3) {
        // A third finger turns the pinch into a three-finger drag (its shortcuts, when the app has any).
        this.gesture = null;
        this.three = this.threeMid();
        this.canvas.setPointerCapture(e.pointerId);
      }
      if (this.touches.size > 2) return;
    }
    try {
      this.canvas.setPointerCapture(e.pointerId);
    } catch {
      // the pointer is already gone (or was never live, as with a synthesised event); the press still counts
    }
    this.canvas.focus({ preventScroll: true });
    // A pen touching down with its barrel button held, or with its eraser end, draws with the tool bound to that
    // button's hold (a temporary tool) instead of moving the camera as a right button would.
    let held = false;
    if (e.pointerType === 'pen') {
      const which = e.button === 5 || (e.buttons & 32) !== 0 ? 'eraser' : e.button === 2 || (e.buttons & 2) !== 0 ? 'barrel' : null;
      if (which) {
        this.penHeld = which;
        this.penButtonAt = performance.now();
        held = this.penHold?.(which, true) ?? false;
      }
    }
    // The ZBrush scheme reads Alt, Shift and Ctrl as sculpting modifiers (a mouse or pen's main button).
    const zb = this.scheme === 'zbrush' && e.pointerType !== 'touch' && e.button === 0 && !this.flying;
    const ctrl = e.ctrlKey || e.metaKey;
    // Alt and Shift turn and pan the view, except on a tool handle that takes them as modifiers (the gizmo's).
    const modified = !zb && (e.altKey || e.shiftKey);
    // In fly navigation every drag looks around; nothing is sculpted.
    const sculpt = !this.flying && (e.button === 0 || held) && (!modified || (this.grabsModified?.(e.offsetX, e.offsetY) ?? false)) && this.strokes?.enabled();
    // ZBrush: Ctrl masks and Shift smooths for this stroke; Alt inverts it, and an Alt press waits to see whether it is a
    // click that picks a SubTool. A press on a tool handle (the gizmo's) keeps its modifiers for the handle.
    const onHandle = this.grabsModified?.(e.offsetX, e.offsetY) ?? false;
    // Nomad's Finger always smooths: a finger's stroke smooths, whatever the tool (a pen or mouse keeps the tool).
    const fingerSmooth = this.fingerSmooths && e.pointerType === 'touch' && sculpt && !onHandle;
    const temporary = fingerSmooth ? 'smooth' : zb && sculpt && !onHandle ? (ctrl ? 'mask' : e.shiftKey ? 'smooth' : null) : null;
    if (temporary) {
      this.holdTool?.(temporary);
      this.holding = true;
    }
    const pick = zb && sculpt && !onHandle && e.altKey && !ctrl && !e.shiftKey;
    // ZBrush's Ctrl+click is not a mask dab: on the model it blurs the mask (sharpens with Alt), on the background it
    // inverts it; so a Ctrl press, like an Alt press, waits to see whether it moves.
    const ctrlClick = zb && sculpt && !onHandle && ctrl;
    // A finger's stroke waits until it has moved a little, so a second finger arriving for a pinch leaves no dab.
    // A press with a mask shortcut waiting also waits, so a tap (not a drag) can be the shortcut.
    const pending = sculpt && (e.pointerType === 'touch' || pick || ctrlClick || (this.tapsMask?.() ?? false));
    const invert = zb ? e.altKey : ctrl;
    this.drag = { button: e.button, x: e.clientX, y: e.clientY, mode: pending ? 'pending' : sculpt ? 'stroke' : 'camera', alt: e.altKey, shift: e.shiftKey, ctrl, moved: 0, pick };
    this.pendingStart = pending ? { ray: this.rayAt(e), pressure: this.pressure(e), ctrl: invert } : null;
    if (sculpt && !pending) this.beginStroke(this.rayAt(e), this.pressure(e), invert, this.tilt(e));
  }

  private pendingStart: { ray: Ray | null; pressure: number; ctrl: boolean } | null = null;

  private beginStroke(ray: Ray | null, pressure: number, ctrl: boolean, tilt: number): void {
    if (ray) this.strokes?.begin(ray, pressure, this.camera.worldPerPixel(this.canvas.clientHeight), ctrl, tilt);
  }

  private move(e: PointerEvent): void {
    // The barrel button pressed and let go while the pen hovers (no contact) is reported as a press and a release.
    if (e.pointerType === 'pen' && !this.drag) {
      const barrel = (e.buttons & 2) !== 0;
      if (barrel !== this.hoverBarrel) {
        this.hoverBarrel = barrel;
        this.penButtonAt = performance.now();
        this.penHover?.(barrel);
      }
    }
    if (e.pointerType === 'touch' && this.touches.has(e.pointerId)) {
      this.touches.set(e.pointerId, [e.clientX, e.clientY]);
      if (this.gesture && this.touches.size === 2) {
        const next = this.gestureFrame();
        if (this.tap && (Math.abs(next.span - this.gesture.span) > 2 || Math.hypot(next.mid[0] - this.gesture.mid[0], next.mid[1] - this.gesture.mid[1]) > 2)) this.tap.moved = true;
        this.camera.pan(next.mid[0] - this.gesture.mid[0], next.mid[1] - this.gesture.mid[1], this.canvas.clientHeight);
        if (next.span > 1 && this.gesture.span > 1) this.camera.zoom(this.gesture.span / next.span);
        this.gesture = next;
        this.invalidate();
        return;
      }
      if (this.three && this.touches.size === 3) {
        const next = this.threeMid();
        const dx = next[0] - this.three[0];
        const dy = next[1] - this.three[1];
        if (this.tap && Math.hypot(dx, dy) > 2) this.tap.moved = true;
        if (this.tap?.moved) {
          this.three = next;
          this.threeFinger?.(dx, dy);
        }
        return;
      }
    }
    if (!this.drag) return;
    this.drag.moved = (this.drag.moved ?? 0) + Math.hypot(e.clientX - this.drag.x, e.clientY - this.drag.y);
    if (this.drag.mode === 'pending') {
      if (Math.hypot(e.clientX - this.drag.x, e.clientY - this.drag.y) < 6 || !this.pendingStart) return;
      this.drag.pick = false;
      this.beginStroke(this.pendingStart.ray, this.pendingStart.pressure, this.pendingStart.ctrl, 0);
      this.pendingStart = null;
      this.drag.mode = 'stroke';
    }
    if (this.drag.mode === 'stroke') {
      if (this.strokes?.missed()) {
        this.strokes.end();
        this.drag.mode = 'camera';
      } else {
        const ray = this.rayAt(e);
        if (ray) this.strokes?.move(ray, this.pressure(e), this.tilt(e));
        this.drag.x = e.clientX;
        this.drag.y = e.clientY;
        return;
      }
    }
    const dx = e.clientX - this.drag.x;
    const dy = e.clientY - this.drag.y;
    this.drag.x = e.clientX;
    this.drag.y = e.clientY;
    if (this.scheme !== 'panel' && this.drag.button === 0 && !this.flying) {
      // Nomad and ZBrush: a drag begun with Alt moves the view, and keeps zooming once Alt is let go.
      if (this.scheme === 'zbrush' && this.drag.ctrl) return; // Ctrl on the background: the mask, on release.
      if (this.drag.alt) {
        if (e.altKey) this.camera.pan(dx, dy, this.canvas.clientHeight);
        else this.camera.zoom(Math.exp(dy * 0.01));
      } else if (this.scheme === 'zbrush' && this.drag.shift && !this.camera.trackball) {
        // Shift snaps the turn to 90 degree steps, from the turn the drag would make without it.
        const free = this.drag.free ?? { yaw: this.camera.yaw, pitch: this.camera.pitch };
        this.camera.yaw = free.yaw;
        this.camera.pitch = free.pitch;
        this.camera.orbit(dx, dy);
        this.drag.free = { yaw: this.camera.yaw, pitch: this.camera.pitch };
        const q = Math.PI / 2;
        this.camera.yaw = Math.round(this.camera.yaw / q) * q;
        this.camera.pitch = Math.max(-q + 1e-3, Math.min(q - 1e-3, Math.round(this.camera.pitch / q) * q));
      } else if (this.drag.shift && this.scheme === 'nomad') this.camera.pan(dx, dy, this.canvas.clientHeight);
      else this.camera.orbit(dx, dy);
      this.invalidate();
      return;
    }
    const panning = this.drag.button === 1 || this.drag.button === 2 || e.shiftKey;
    if (this.flying && !panning) this.camera.look(dx, dy);
    else if (panning) this.camera.pan(dx, dy, this.canvas.clientHeight);
    else if (e.ctrlKey || e.metaKey) this.camera.zoom(Math.exp(dy * 0.01));
    else this.camera.orbit(dx, dy);
    this.invalidate();
  }

  private up(e: PointerEvent): void {
    if (this.canvas.hasPointerCapture(e.pointerId)) this.canvas.releasePointerCapture(e.pointerId);
    if (e.pointerType === 'touch') {
      this.touches.delete(e.pointerId);
      if (this.touches.size < 3) this.three = null;
      // A tool held for the first finger (Finger always smooths) is let go once every finger is up, also when a
      // pinch or a tap took over.
      const release = (): void => {
        if (this.touches.size === 0 && this.holding) {
          this.holding = false;
          this.holdTool?.(null);
        }
      };
      // Nomad: a quick tap of two fingers undoes, of three redoes (when every finger has lifted).
      if (this.touches.size === 0 && this.tap) {
        const t = this.tap;
        this.tap = null;
        if (this.scheme === 'nomad' && e.type !== 'pointercancel' && !t.moved && t.most >= 2 && performance.now() - t.at < 300) {
          this.command?.(t.most >= 3 ? 'history.redo' : 'history.undo');
          this.gesture = null;
          this.drag = null;
          release();
          return;
        }
      }
      if (this.gesture) {
        // The gesture ends when a finger lifts; the remaining finger does nothing until it lifts too.
        if (this.touches.size < 2) this.gesture = null;
        this.drag = null;
        release();
        return;
      }
    }
    const d = e.type === 'pointercancel' && this.drag?.mode !== 'stroke' ? null : this.drag;
    if (d?.mode === 'pending' && this.pendingStart) {
      if (d.pick) {
        // ZBrush's Alt+click: pick the SubTool under the pointer.
        if (this.pendingStart.ray) this.pickAt?.(this.pendingStart.ray);
      } else if (this.scheme === 'zbrush' && d.ctrl && e.pointerType !== 'touch') {
        if (this.pendingStart.ray) this.ctrlClick?.(this.pendingStart.ray, d.alt ?? false);
      } else if (this.pendingStart.ray && (this.tapsMask?.() ?? false)) {
        // Nomad's One tap shortcuts: the tap works on the mask instead of leaving a dab.
        this.maskTap?.(this.pendingStart.ray);
      } else {
        // A tap: one dab where the finger touched.
        this.beginStroke(this.pendingStart.ray, this.pendingStart.pressure, this.pendingStart.ctrl, 0);
        this.strokes?.end();
      }
    } else if (d?.mode === 'stroke') {
      const missed = this.strokes?.missed() ?? false;
      this.strokes?.end();
      if (missed && this.scheme === 'zbrush' && d.ctrl) this.command?.('mask.invert');
    } else if (d?.mode === 'camera' && this.scheme === 'zbrush' && d.ctrl && d.button === 0) {
      // ZBrush: Ctrl+click on the background inverts the mask, Ctrl+drag there clears it.
      this.command?.((d.moved ?? 0) < 4 ? 'mask.invert' : 'mask.clear');
    }
    if (this.holding) {
      this.holding = false;
      this.holdTool?.(null);
    }
    this.pendingStart = null;
    this.drag = null;
    if (this.penHeld) {
      const which = this.penHeld;
      this.penHeld = null;
      this.hoverBarrel = false;
      this.penButtonAt = performance.now();
      this.penHold?.(which, false);
    }
  }

  /** Set by the app: the pen's barrel button or eraser end touched down (true) or lifted (false); returns whether a
   * temporary tool is in use, so the touch sculpts. */
  penHold: ((which: 'barrel' | 'eraser', down: boolean) => boolean) | null = null;
  /** Set by the app: the barrel button pressed (true) or let go (false) while the pen hovers. */
  penHover: ((down: boolean) => void) | null = null;
  /** Fly navigation (first person): drags look around from the eye instead of orbiting, and never sculpt. The keys
   * that walk are the app's. */
  flying = false;
  /** Set by the app: whether a press with Alt or Shift at this canvas point lands on a tool handle that takes the
   * modifier itself (the gizmo's), rather than turning or panning the view. */
  grabsModified: ((x: number, y: number) => boolean) | null = null;
  /** When a pen button last changed, so a context menu it raises can be ignored. */
  penButtonAt = -1e9;
  private penHeld: 'barrel' | 'eraser' | null = null;
  private hoverBarrel = false;

  private wheel(e: WheelEvent): void {
    e.preventDefault();
    this.camera.zoom(Math.exp(e.deltaY * 0.0015));
    this.invalidate();
  }
}
