/**
 * The on-canvas transform gizmo: three axis arrows that move, three rings that turn and three squares that scale
 * along an axis, plus a centre square that scales evenly, drawn in the stage overlay around the active object's bounds
 * centre. Dragging previews the change on the GPU (a model matrix) and the release sends one transform_object.
 * Handles keep a constant size on screen.
 */

import type { Vec3 } from '../viewport/math';

export type Project = (p: Vec3) => [number, number] | null;

export type GizmoHandle = { kind: 'move' | 'rotate' | 'scale'; axis: 0 | 1 | 2 } | { kind: 'uniform'; axis: -1 };

export interface GizmoDrag {
  handle: GizmoHandle;
  start: [number, number];
  /** The result so far: world translation, degrees about the handle's axis, and per-axis scale. */
  translate: Vec3;
  degrees: number;
  scale: Vec3;
}

const AXES: Vec3[] = [
  [1, 0, 0],
  [0, 1, 0],
  [0, 0, 1],
];

/** Two unit vectors spanning the plane across `axis`. */
function across(axis: number): [Vec3, Vec3] {
  return [AXES[(axis + 1) % 3], AXES[(axis + 2) % 3]];
}

const at = (c: Vec3, d: Vec3, s: number): Vec3 => [c[0] + d[0] * s, c[1] + d[1] * s, c[2] + d[2] * s];

export class Gizmo {
  /** Draws the handles; `len` is the arrow length in world units (the caller keeps it a constant size on screen). */
  static paths(center: Vec3, len: number, project: Project): { axes: string[]; uniform: string } {
    const c = project(center);
    if (!c) return { axes: ['', '', ''], uniform: '' };
    const axes = AXES.map((dir, a) => {
      const tip = project(at(center, dir, len));
      const box = project(at(center, dir, len * 0.62));
      let d = '';
      if (tip) {
        d += `M${c[0].toFixed(1)},${c[1].toFixed(1)}L${tip[0].toFixed(1)},${tip[1].toFixed(1)}`;
        // An arrowhead: a short V back from the tip, square to the projected axis.
        const ux = tip[0] - c[0];
        const uy = tip[1] - c[1];
        const l = Math.hypot(ux, uy) || 1;
        const [nx, ny] = [ux / l, uy / l];
        d += `M${(tip[0] - nx * 9 - ny * 5).toFixed(1)},${(tip[1] - ny * 9 + nx * 5).toFixed(1)}L${tip[0].toFixed(1)},${tip[1].toFixed(1)}L${(tip[0] - nx * 9 + ny * 5).toFixed(1)},${(tip[1] - ny * 9 - nx * 5).toFixed(1)}`;
      }
      if (box) d += `M${(box[0] - 4).toFixed(1)},${(box[1] - 4).toFixed(1)}h8v8h-8z`;
      const ring = Gizmo.ring(center, len * 0.82, a, project);
      if (ring.length > 1) d += `M${ring.map((q) => `${q[0].toFixed(1)},${q[1].toFixed(1)}`).join('L')}`;
      return d;
    });
    const uniform = `M${(c[0] - 6).toFixed(1)},${(c[1] - 6).toFixed(1)}h12v12h-12z`;
    return { axes, uniform };
  }

  static ring(center: Vec3, r: number, axis: number, project: Project): [number, number][] {
    const [u, v] = across(axis);
    const out: [number, number][] = [];
    for (let i = 0; i <= 64; i++) {
      const t = (i / 64) * Math.PI * 2;
      const q = project([center[0] + (u[0] * Math.cos(t) + v[0] * Math.sin(t)) * r, center[1] + (u[1] * Math.cos(t) + v[1] * Math.sin(t)) * r, center[2] + (u[2] * Math.cos(t) + v[2] * Math.sin(t)) * r]);
      if (q) out.push(q);
    }
    return out;
  }

  /** The handle under a canvas point, if any: squares first, then arrows, then rings. */
  static hit(p: [number, number], center: Vec3, len: number, project: Project): GizmoHandle | null {
    const c = project(center);
    if (!c) return null;
    if (Math.abs(p[0] - c[0]) <= 8 && Math.abs(p[1] - c[1]) <= 8) return { kind: 'uniform', axis: -1 };
    const segment = (a: [number, number], b: [number, number]): number => {
      const [dx, dy] = [b[0] - a[0], b[1] - a[1]];
      const t = Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy || 1)));
      return Math.hypot(p[0] - a[0] - dx * t, p[1] - a[1] - dy * t);
    };
    for (let a = 0; a < 3; a++) {
      const box = project(at(center, AXES[a], len * 0.62));
      if (box && Math.abs(p[0] - box[0]) <= 7 && Math.abs(p[1] - box[1]) <= 7) return { kind: 'scale', axis: a as 0 | 1 | 2 };
    }
    // Where arrows or rings cross on screen, the nearest one wins.
    let best: { d: number; h: GizmoHandle } | null = null;
    for (let a = 0; a < 3; a++) {
      const tip = project(at(center, AXES[a], len));
      if (!tip || Math.hypot(p[0] - c[0], p[1] - c[1]) <= 10) continue;
      const d = segment(c, tip);
      if (d <= 7 && (!best || d < best.d)) best = { d, h: { kind: 'move', axis: a as 0 | 1 | 2 } };
    }
    if (best) return best.h;
    for (let a = 0; a < 3; a++) {
      const ring = Gizmo.ring(center, len * 0.82, a, project);
      for (let i = 1; i < ring.length; i++) {
        const d = segment(ring[i - 1], ring[i]);
        if (d <= 6 && (!best || d < best.d)) best = { d, h: { kind: 'rotate', axis: a as 0 | 1 | 2 } };
      }
    }
    return best ? best.h : null;
  }

  /** Updates a drag from the pointer: moves follow the projected axis, rings follow the angle about the centre,
   * squares scale by the distance travelled along their axis. `eyeDir` points from the centre toward the camera. */
  static update(drag: GizmoDrag, p: [number, number], center: Vec3, len: number, project: Project, eyeDir: Vec3): void {
    const c = project(center);
    if (!c) return;
    const h = drag.handle;
    const [dx, dy] = [p[0] - drag.start[0], p[1] - drag.start[1]];
    if (h.kind === 'uniform') {
      const f = Math.max(0.05, 1 + (dx - dy) / 150);
      drag.scale = [f, f, f];
      return;
    }
    const tip = project(at(center, AXES[h.axis], len));
    if (!tip) return;
    const [sx, sy] = [tip[0] - c[0], tip[1] - c[1]];
    const s2 = sx * sx + sy * sy || 1;
    const along = ((dx * sx + dy * sy) / s2) * len;
    if (h.kind === 'move') {
      drag.translate = at([0, 0, 0], AXES[h.axis], along);
    } else if (h.kind === 'scale') {
      const f = Math.max(0.05, 1 + along / len);
      drag.scale = [1, 1, 1];
      drag.scale[h.axis] = f;
    } else {
      const a0 = Math.atan2(drag.start[1] - c[1], drag.start[0] - c[0]);
      const a1 = Math.atan2(p[1] - c[1], p[0] - c[0]);
      let deg = ((a1 - a0) * 180) / Math.PI;
      deg = ((deg + 540) % 360) - 180;
      // Screen y points down, so a clockwise drag on screen is a positive angle here; it turns the object
      // counter-clockwise about an axis pointing at the viewer, and the other way about one pointing away.
      const facing = eyeDir[h.axis] >= 0 ? -1 : 1;
      drag.degrees = deg * facing;
    }
  }

  /** The preview matrix (column-major) for a drag: about `center`, scale, then turn, then move. */
  static matrix(drag: GizmoDrag, center: Vec3): Float32Array {
    const r = (drag.degrees * Math.PI) / 180;
    const [cs, sn] = [Math.cos(r), Math.sin(r)];
    // Rotation about the handle's axis (identity for other handles).
    const R = [1, 0, 0, 0, 1, 0, 0, 0, 1];
    if (drag.handle.kind === 'rotate') {
      const a = drag.handle.axis;
      if (a === 0) R.splice(0, 9, 1, 0, 0, 0, cs, sn, 0, -sn, cs);
      else if (a === 1) R.splice(0, 9, cs, 0, -sn, 0, 1, 0, sn, 0, cs);
      else R.splice(0, 9, cs, sn, 0, -sn, cs, 0, 0, 0, 1);
    }
    const S = drag.scale;
    // M = T(center + translate) * R * S * T(-center), column-major.
    const m = new Float32Array(16);
    for (let col = 0; col < 3; col++) for (let row = 0; row < 3; row++) m[col * 4 + row] = R[col * 3 + row] * S[col];
    m[15] = 1;
    for (let row = 0; row < 3; row++) {
      let rc = 0;
      for (let col = 0; col < 3; col++) rc += m[col * 4 + row] * center[col];
      m[12 + row] = center[row] + drag.translate[row] - rc;
    }
    return m;
  }
}
