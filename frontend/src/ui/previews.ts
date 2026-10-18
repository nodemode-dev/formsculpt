/**
 * Thumbnails for the ZBrush layout's pop-ups and shelf (ZBrush shows its brush, stroke, alpha and material as preview
 * icons that open pop-ups of thumbnails, per its Starting Guide). Each is computed from what the window actually uses,
 * not drawn to look like it: a material is the mesh shader's matcap sum over a sphere (gl-renderer.ts MESH_FS with the
 * SHADINGS values), an alpha is the engine's own generator (fsc-sculpt alpha.rs Alpha::builtin, ported line for line),
 * and a stroke mode is its dabs laid along a curve. Lit (PBR) shading has no matcap; its sphere is the key light's
 * diffuse and a highlight, an approximation of the lit shader, and says so in its tooltip.
 */

import { SHADINGS } from '../viewport/gl-renderer';

const cache = new Map<string, string>();

/** A canvas drawn once per key, kept as a data URL so every copy is a plain image. */
function image(key: string, size: number, draw: (g: CanvasRenderingContext2D, n: number) => void): HTMLImageElement {
  let url = cache.get(key);
  if (!url) {
    const c = document.createElement('canvas');
    c.width = c.height = size;
    const g = c.getContext('2d');
    if (g) draw(g, size);
    url = c.toDataURL();
    cache.set(key, url);
  }
  const img = document.createElement('img');
  img.src = url;
  img.width = img.height = size;
  img.alt = '';
  img.draggable = false;
  return img;
}

const clamp = (v: number, lo = 0, hi = 1): number => Math.min(hi, Math.max(lo, v));
const smoothstep = (a: number, b: number, x: number): number => {
  const t = clamp((x - a) / (b - a));
  return t * t * (3 - 2 * t);
};

/** A material preset on a sphere, by the mesh shader's matcap sum (the default key light, unpainted grey). */
export function materialPreview(name: string, size = 48): HTMLImageElement {
  return image(`m:${name}:${size}`, size, (g, n) => {
    const s = SHADINGS[name] ?? SHADINGS.clay;
    const data = g.createImageData(n, n);
    const az = (-37 * Math.PI) / 180;
    const el = (41 * Math.PI) / 180;
    const key: [number, number, number] = [Math.cos(el) * Math.sin(az), Math.sin(el), Math.cos(el) * Math.cos(az)];
    const keyStrength = 2.6 * (1 + 0.94 + 0.87) * 0.1282;
    const fillDir = [0.6, 0.1, 0.5].map((v) => v / Math.hypot(0.6, 0.1, 0.5));
    const base = [0.72, 0.7, 0.68];
    for (let j = 0; j < n; j++) {
      for (let i = 0; i < n; i++) {
        const x = ((i + 0.5) / n) * 2 - 1;
        const y = 1 - ((j + 0.5) / n) * 2;
        const r2 = x * x + y * y;
        const at = (j * n + i) * 4;
        if (r2 > 1) continue;
        const z = Math.sqrt(1 - r2);
        const nrm = [x, y, z];
        const dot = (a: number[], b: number[]): number => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let col: number[];
        if (s.lit) {
          const d = clamp(dot(nrm, key));
          const h = [key[0], key[1], key[2] + 1].map((v, k, arr) => v / Math.hypot(arr[0], arr[1], arr[2]));
          const spec = Math.pow(clamp(dot(nrm, h)), 40) * 0.6;
          col = base.map((b) => b * (0.15 + 0.85 * d) + spec);
        } else {
          // reflect(-v, n) with v = (0, 0, 1), then the matcap coordinates.
          const rx = 2 * nrm[2] * nrm[0];
          const ry = 2 * nrm[2] * nrm[1];
          const rz = 2 * nrm[2] * nrm[2] - 1;
          const m = 2 * Math.sqrt(rx * rx + ry * ry + (rz + 1) * (rz + 1));
          const mx = rx / m + 0.5;
          const my = ry / m + 0.5;
          const k = clamp(dot(nrm, key)) * keyStrength;
          const fill = clamp(dot(nrm, fillDir)) * 0.25;
          const rim = Math.pow(1 - clamp(z), 3) * s.cap[3];
          const spec = Math.pow(clamp(1 - Math.hypot(mx - 0.34, my - 0.68) * 2.2), s.cap[2]) * s.cap[1];
          const cavity = 0.55 + 0.45 * smoothstep(-0.2, 0.9, y * 0.5 + 0.5);
          col = base.map((b, c) => {
            const t = s.tint[c];
            return b * t * (0.18 + k * s.cap[0] + fill) * cavity + rim + spec * (1 + (t - 1) * 0.6);
          });
          col = col.map((v) => Math.pow(Math.max(0, v), 1 / 1.1));
        }
        // Edge softening over the last pixel.
        const edge = clamp((1 - Math.sqrt(r2)) * n * 0.5);
        data.data[at] = clamp(col[0]) * 255;
        data.data[at + 1] = clamp(col[1]) * 255;
        data.data[at + 2] = clamp(col[2]) * 255;
        data.data[at + 3] = edge * 255;
      }
    }
    g.putImageData(data, 0, 0);
  });
}

/** The engine's hash for its generated alphas (alpha.rs), in 32-bit unsigned arithmetic. */
function hash(x: number, y: number): number {
  let h = (Math.imul(x | 0, 0x8da6b343) ^ Math.imul(y | 0, 0xd8163841)) >>> 0;
  h = (h ^ (h >>> 13)) >>> 0;
  h = Math.imul(h, 0x5bd1e995) >>> 0;
  return (h & 0xffff) / 65535;
}

/** The value of a built-in alpha at (x, y), -1 to 1 across, y up: alpha.rs's Alpha::builtin, line for line. */
function alphaAt(name: string, x: number, y: number): number {
  switch (name) {
    case 'square':
      return Math.abs(x) < 0.8 && Math.abs(y) < 0.8 ? 1 : 0;
    case 'noise': {
      const gx = (x + 1) * 6;
      const gy = (y + 1) * 6;
      const ix = Math.floor(gx);
      const iy = Math.floor(gy);
      const fx = gx - ix;
      const fy = gy - iy;
      const s = (t: number): number => t * t * (3 - 2 * t);
      const a = hash(ix, iy) + (hash(ix + 1, iy) - hash(ix, iy)) * s(fx);
      const b = hash(ix, iy + 1) + (hash(ix + 1, iy + 1) - hash(ix, iy + 1)) * s(fx);
      return a + (b - a) * s(fy);
    }
    case 'cracks': {
      const gx = (x + 1) * 3;
      const gy = (y + 1) * 3;
      const ix = Math.floor(gx);
      const iy = Math.floor(gy);
      let d1 = Infinity;
      let d2 = Infinity;
      for (let oy = -1; oy <= 1; oy++) {
        for (let ox = -1; ox <= 1; ox++) {
          const cx = ix + ox;
          const cy = iy + oy;
          const px = cx + hash(cx, cy);
          const py = cy + hash(cy * 7, cx * 13);
          const d = Math.hypot(gx - px, gy - py);
          if (d < d1) {
            d2 = d1;
            d1 = d;
          } else if (d < d2) d2 = d;
        }
      }
      return 1 - Math.min(1, (d2 - d1) / 0.12);
    }
    case 'rings':
      return 0.5 + 0.5 * Math.cos(Math.hypot(x, y) * Math.PI * 5);
    case 'star': {
      const r = Math.hypot(x, y);
      const a = Math.atan2(y, x);
      return r < 0.55 + 0.35 * Math.cos(a * 5) ? 1 : 0;
    }
    case 'stripes':
      return 0.5 + 0.5 * Math.cos(y * Math.PI * 6);
    case 'dots': {
      const fx = (((x + 1) * 3) % 1) - 0.5;
      const fy = (((y + 1) * 3) % 1) - 0.5;
      return fx * fx + fy * fy < 0.09 ? 1 : 0;
    }
    default:
      // No alpha: the brush's plain round falloff, white in the middle.
      return clamp(1 - Math.hypot(x, y));
  }
}

/** A built-in alpha as the engine generates it, white where it pushes. */
export function alphaPreview(name: string, size = 48): HTMLImageElement {
  return image(`a:${name}:${size}`, size, (g, n) => {
    const data = g.createImageData(n, n);
    for (let j = 0; j < n; j++) {
      for (let i = 0; i < n; i++) {
        const v = clamp(alphaAt(name, ((i + 0.5) / n) * 2 - 1, 1 - ((j + 0.5) / n) * 2)) * 255;
        const at = (j * n + i) * 4;
        data.data[at] = data.data[at + 1] = data.data[at + 2] = v;
        data.data[at + 3] = 255;
      }
    }
    g.putImageData(data, 0, 0);
  });
}

/** A stroke mode's dabs along a curve: dots upright, roll turned along the path, spray scattered. */
export function strokePreview(mode: string, size = 48): HTMLImageElement {
  return image(`s:${mode}:${size}`, size, (g, n) => {
    g.fillStyle = '#1d1d1d';
    g.fillRect(0, 0, n, n);
    g.strokeStyle = '#e8e8e8';
    g.fillStyle = '#e8e8e8';
    g.lineWidth = Math.max(1, n / 32);
    const steps = 7;
    for (let k = 0; k < steps; k++) {
      const t = k / (steps - 1);
      const px = n * (0.14 + 0.72 * t);
      const py = n * (0.62 - 0.28 * Math.sin(t * Math.PI));
      const tangent = Math.atan2(-0.28 * Math.PI * Math.cos(t * Math.PI) * n, 0.72 * n);
      const r = n * 0.07;
      if (mode === 'spray') {
        const jx = (hash(k, 3) - 0.5) * n * 0.22;
        const jy = (hash(k, 9) - 0.5) * n * 0.22;
        g.beginPath();
        g.arc(px + jx, py + jy, r * (0.6 + hash(k, 5) * 0.6), 0, Math.PI * 2);
        g.fill();
      } else {
        g.save();
        g.translate(px, py);
        if (mode === 'roll') g.rotate(tangent);
        g.strokeRect(-r, -r, r * 2, r * 2);
        g.beginPath();
        g.moveTo(0, -r);
        g.lineTo(0, -r * 1.6);
        g.stroke();
        g.restore();
      }
    }
  });
}
