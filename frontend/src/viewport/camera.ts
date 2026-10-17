import { add, cross, invert, lookAt, multiply, normalize, ortho, perspective, scale, sub, transformPoint, type Mat4, type Vec3 } from './math';

export type ViewName = 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom';

/** A recorded view: the engine keeps one with each undo step (set_history_camera). Angles in degrees. */
export interface CameraPose {
  target: Vec3;
  distance: number;
  yaw: number;
  pitch: number;
  ortho: boolean;
}

const VIEWS: Record<ViewName, [number, number]> = {
  front: [0, 0],
  back: [Math.PI, 0],
  right: [Math.PI / 2, 0],
  left: [-Math.PI / 2, 0],
  top: [0, Math.PI / 2 - 1e-4],
  bottom: [0, -Math.PI / 2 + 1e-4],
};

export class OrbitCamera {
  target: Vec3 = [0, 0, 0];
  distance = 4;
  yaw = Math.PI / 6;
  pitch = Math.PI / 9;
  fov = (40 * Math.PI) / 180;
  orthographic = false;
  aspect = 1;
  /** Trackball mode: the eye's direction from the target and a free up vector, so the view can roll and go over the
   * poles. Turntable mode (the default) uses yaw and pitch with the world's up. */
  trackball = false;
  private dir: Vec3 = [0, 0, 1];
  private up: Vec3 = [0, 1, 0];

  private turntableDir(): Vec3 {
    const cp = Math.cos(this.pitch);
    return [cp * Math.sin(this.yaw), Math.sin(this.pitch), cp * Math.cos(this.yaw)];
  }

  eye(): Vec3 {
    return add(this.target, scale(this.trackball ? this.dir : this.turntableDir(), this.distance));
  }

  /** The view's up and right directions in world space. */
  private axes(): { up: Vec3; right: Vec3 } {
    const forward = normalize(sub(this.target, this.eye()));
    const right = normalize(cross(forward, this.trackball ? this.up : [0, 1, 0]));
    return { up: cross(right, forward), right };
  }

  /** The screen's up direction in world space, so brush alphas stamp upright on screen. */
  screenUp(): Vec3 {
    return this.axes().up;
  }

  view(): Mat4 {
    return lookAt(this.eye(), this.target, this.trackball ? this.up : [0, 1, 0]);
  }

  setTrackball(on: boolean): void {
    if (on === this.trackball) return;
    if (on) {
      this.dir = this.turntableDir();
      this.up = this.axes().up;
    } else {
      this.pitch = Math.asin(Math.max(-1, Math.min(1, this.dir[1])));
      this.yaw = Math.atan2(this.dir[0], this.dir[2]);
      this.pitch = Math.max(-Math.PI / 2 + 1e-3, Math.min(Math.PI / 2 - 1e-3, this.pitch));
    }
    this.trackball = on;
  }

  projection(): Mat4 {
    const near = Math.max(this.distance * 0.001, 1e-4);
    const far = this.distance * 100;
    if (this.orthographic) return ortho(this.distance * Math.tan(this.fov / 2), this.aspect, -far, far);
    return perspective(this.fov, this.aspect, near, far);
  }

  viewProjection(): Mat4 {
    return multiply(this.projection(), this.view());
  }

  orbit(dx: number, dy: number): void {
    if (this.trackball) {
      // Turn the eye direction and the up vector about the view's own axes: horizontal drags about its up, vertical
      // drags about its right, so the view can roll and there is no pole to stop at.
      const { up, right } = this.axes();
      const turn = (v: Vec3): Vec3 => rotate(rotate(v, up, -dx * 0.008), right, dy * 0.008);
      this.dir = normalize(turn(this.dir));
      this.up = normalize(turn(this.up));
      return;
    }
    this.yaw -= dx * 0.008;
    this.pitch = Math.max(-Math.PI / 2 + 1e-3, Math.min(Math.PI / 2 - 1e-3, this.pitch + dy * 0.008));
  }

  pan(dx: number, dy: number, viewportHeight: number): void {
    const { up, right } = this.axes();
    const unit = (2 * this.distance * Math.tan(this.fov / 2)) / Math.max(viewportHeight, 1);
    this.target = add(this.target, add(scale(right, -dx * unit), scale(up, dy * unit)));
  }

  zoom(factor: number): void {
    this.distance = Math.max(1e-3, Math.min(1e5, this.distance * factor));
  }

  /** Moves the orbit target to `point` and re-derives yaw, pitch and distance so the eye stays where it is. */
  pivotAt(point: Vec3): void {
    const eye = this.eye();
    const v = sub(eye, point);
    const d = Math.hypot(v[0], v[1], v[2]);
    if (d < 1e-6) return;
    this.target = point;
    this.distance = d;
    if (this.trackball) {
      const up = this.axes().up;
      this.dir = scale(v, 1 / d);
      // Keep the up vector square to the new direction so the view does not twist.
      this.up = normalize(sub(up, scale(this.dir, up[0] * this.dir[0] + up[1] * this.dir[1] + up[2] * this.dir[2])));
      return;
    }
    this.pitch = Math.asin(Math.max(-1, Math.min(1, v[1] / d)));
    this.yaw = Math.atan2(v[0], v[2]);
  }

  frame(min: Vec3, max: Vec3): void {
    this.target = scale(add(min, max), 0.5);
    const radius = Math.max(1e-3, 0.5 * Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]));
    this.distance = (radius / Math.sin(this.fov / 2)) * 1.1;
  }

  ray(x: number, y: number, width: number, height: number): { origin: Vec3; dir: Vec3 } | null {
    const inv = invert(this.viewProjection());
    if (!inv) return null;
    const nx = (2 * x) / width - 1;
    const ny = 1 - (2 * y) / height;
    const a = transformPoint(inv, [nx, ny, -1]);
    const b = transformPoint(inv, [nx, ny, 1]);
    const near: Vec3 = [a[0] / a[3], a[1] / a[3], a[2] / a[3]];
    const far: Vec3 = [b[0] / b[3], b[1] / b[3], b[2] / b[3]];
    return { origin: near, dir: normalize(sub(far, near)) };
  }

  worldPerPixel(height: number): number {
    return (2 * this.distance * Math.tan(this.fov / 2)) / Math.max(height, 1);
  }

  /** First-person look: turns the view about the eye (the eye stays put, the target swings round it). Drags in
   * pixels, the same feel as orbiting. */
  look(dx: number, dy: number): void {
    const eye = this.eye();
    if (this.trackball) this.setTrackball(false);
    this.yaw += dx * 0.004;
    this.pitch = Math.max(-Math.PI / 2 + 1e-3, Math.min(Math.PI / 2 - 1e-3, this.pitch + dy * 0.004));
    // The target is put back in front of the eye along the new direction, at the same distance.
    const d = this.turntableDir();
    this.target = sub(eye, scale(d, this.distance));
  }

  /** First-person move: forward along the view, right across it and up the world's vertical, in world units. The
   * eye and the target move together. */
  walk(forward: number, right: number, up: number): void {
    const f = normalize(sub(this.target, this.eye()));
    const r = normalize(cross(f, [0, 1, 0]));
    this.target = add(this.target, add(add(scale(f, forward), scale(r, right)), [0, up, 0]));
  }

  /** Where the view is, as a history step records it: target, distance, the eye's direction as yaw and pitch in
   * degrees (the engine's get_view convention) and orthographic. A trackball view's roll is not kept. */
  pose(): CameraPose {
    const d = this.trackball ? this.dir : this.turntableDir();
    const deg = 180 / Math.PI;
    return { target: [...this.target], distance: this.distance, yaw: Math.atan2(d[0], d[2]) * deg, pitch: Math.asin(Math.max(-1, Math.min(1, d[1]))) * deg, ortho: this.orthographic };
  }

  /** Moves the view to a recorded pose; a trackball view comes back upright. */
  setPose(p: CameraPose): void {
    const rad = Math.PI / 180;
    this.target = [p.target[0], p.target[1], p.target[2]];
    this.distance = Math.max(1e-3, Math.min(1e5, p.distance));
    this.yaw = p.yaw * rad;
    this.pitch = Math.max(-Math.PI / 2 + 1e-3, Math.min(Math.PI / 2 - 1e-3, p.pitch * rad));
    this.orthographic = p.ortho;
    if (this.trackball) {
      this.dir = this.turntableDir();
      this.up = [0, 1, 0];
      if (Math.abs(this.dir[1]) > 0.999) this.up = [0, 0, -Math.sign(this.dir[1])];
    }
  }

  snap(view: ViewName): void {
    [this.yaw, this.pitch] = VIEWS[view];
    if (this.trackball) {
      // A snapped view is upright again: the roll is dropped.
      this.dir = this.turntableDir();
      this.up = [0, 1, 0];
      if (Math.abs(this.dir[1]) > 0.999) this.up = [0, 0, -Math.sign(this.dir[1])];
    }
  }
}

/** `v` turned by `angle` radians about the unit `axis` (Rodrigues' formula). */
function rotate(v: Vec3, axis: Vec3, angle: number): Vec3 {
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  const d = (axis[0] * v[0] + axis[1] * v[1] + axis[2] * v[2]) * (1 - c);
  const k = cross(axis, v);
  return [v[0] * c + k[0] * s + axis[0] * d, v[1] * c + k[1] * s + axis[1] * d, v[2] * c + k[2] * s + axis[2] * d];
}
