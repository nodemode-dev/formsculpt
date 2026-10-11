import type { MeshBuffers } from '../engine/service';
import { OrbitCamera, type ViewName } from './camera';
import { GlRenderer } from './gl-renderer';
import type { Vec3 } from './math';

export interface ViewportStats {
  renderer: string;
  frameMs: number;
}

export class Viewport {
  readonly camera = new OrbitCamera();
  readonly renderer: GlRenderer;
  private canvas: HTMLCanvasElement;
  private dirty = true;
  private frame = 0;
  private lastFrameMs = 0;
  private drag: { button: number; x: number; y: number } | null = null;
  onFrame: ((stats: ViewportStats) => void) | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.renderer = new GlRenderer(canvas);
    const bg = getComputedStyle(document.documentElement).getPropertyValue('--canvas-bg');
    if (bg) this.renderer.setClearColor(bg);
    new ResizeObserver(() => this.resize()).observe(canvas);
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
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.camera.aspect = w / h;
    this.invalidate();
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

  toggleOrtho(): boolean {
    this.camera.orthographic = !this.camera.orthographic;
    this.invalidate();
    return this.camera.orthographic;
  }

  private down(e: PointerEvent): void {
    this.canvas.setPointerCapture(e.pointerId);
    this.drag = { button: e.button, x: e.clientX, y: e.clientY };
  }

  private move(e: PointerEvent): void {
    if (!this.drag) return;
    const dx = e.clientX - this.drag.x;
    const dy = e.clientY - this.drag.y;
    this.drag.x = e.clientX;
    this.drag.y = e.clientY;
    const panning = this.drag.button === 1 || this.drag.button === 2 || e.shiftKey;
    if (panning) this.camera.pan(dx, dy, this.canvas.clientHeight);
    else if (e.ctrlKey || e.metaKey) this.camera.zoom(Math.exp(dy * 0.01));
    else this.camera.orbit(dx, dy);
    this.invalidate();
  }

  private up(e: PointerEvent): void {
    if (this.canvas.hasPointerCapture(e.pointerId)) this.canvas.releasePointerCapture(e.pointerId);
    this.drag = null;
  }

  private wheel(e: WheelEvent): void {
    e.preventDefault();
    this.camera.zoom(Math.exp(e.deltaY * 0.0015));
    this.invalidate();
  }
}
