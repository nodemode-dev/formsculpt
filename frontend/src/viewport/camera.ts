import { add, cross, lookAt, multiply, normalize, ortho, perspective, scale, sub, type Mat4, type Vec3 } from './math';

export type ViewName = 'front' | 'back' | 'left' | 'right' | 'top' | 'bottom';

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

  eye(): Vec3 {
    const cp = Math.cos(this.pitch);
    return add(this.target, [this.distance * cp * Math.sin(this.yaw), this.distance * Math.sin(this.pitch), this.distance * cp * Math.cos(this.yaw)]);
  }

  view(): Mat4 {
    return lookAt(this.eye(), this.target, [0, 1, 0]);
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
    this.yaw -= dx * 0.008;
    this.pitch = Math.max(-Math.PI / 2 + 1e-3, Math.min(Math.PI / 2 - 1e-3, this.pitch + dy * 0.008));
  }

  pan(dx: number, dy: number, viewportHeight: number): void {
    const forward = normalize(sub(this.target, this.eye()));
    const right = normalize(cross(forward, [0, 1, 0]));
    const up = cross(right, forward);
    const unit = (2 * this.distance * Math.tan(this.fov / 2)) / Math.max(viewportHeight, 1);
    this.target = add(this.target, add(scale(right, -dx * unit), scale(up, dy * unit)));
  }

  zoom(factor: number): void {
    this.distance = Math.max(1e-3, Math.min(1e5, this.distance * factor));
  }

  frame(min: Vec3, max: Vec3): void {
    this.target = scale(add(min, max), 0.5);
    const radius = Math.max(1e-3, 0.5 * Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]));
    this.distance = (radius / Math.sin(this.fov / 2)) * 1.1;
  }

  snap(view: ViewName): void {
    [this.yaw, this.pitch] = VIEWS[view];
  }
}
