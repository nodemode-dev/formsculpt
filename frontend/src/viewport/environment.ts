/**
 * Environment maps for image-based lighting: equirectangular images in linear light, generated in code (studio,
 * sunset, overcast, night) or read from a Radiance .hdr file or an ordinary image. Each becomes a float mip chain,
 * built here by box filtering because WebGL2 cannot generate mips for half-float textures without an extension; the
 * renderer samples a low mip for diffuse light and a mip chosen by roughness for reflections.
 */

export interface EnvImage {
  width: number;
  height: number;
  /** RGB per pixel, linear, row 0 at the top (straight up). */
  rgb: Float32Array;
}

export const ENV_PRESETS: [string, string][] = [
  ['studio', 'Studio'],
  ['sunset', 'Sunset'],
  ['overcast', 'Overcast'],
  ['night', 'Night'],
];

const smooth = (a: number, b: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - a) / (b - a)));
  return t * t * (3 - 2 * t);
};

/** A small light: a soft disc of `power` around a direction given by azimuth and elevation (radians). */
type Lamp = { az: number; el: number; size: number; color: [number, number, number] };

function generate(sky: (el: number, az: number) => [number, number, number], lamps: Lamp[], width = 256): EnvImage {
  const height = width / 2;
  const rgb = new Float32Array(width * height * 3);
  const dirs = lamps.map((l) => ({ l, d: [Math.cos(l.el) * Math.sin(l.az), Math.sin(l.el), Math.cos(l.el) * Math.cos(l.az)] }));
  for (let y = 0; y < height; y++) {
    const el = Math.PI / 2 - ((y + 0.5) / height) * Math.PI;
    for (let x = 0; x < width; x++) {
      const az = ((x + 0.5) / width) * Math.PI * 2 - Math.PI;
      const c = sky(el, az);
      const d = [Math.cos(el) * Math.sin(az), Math.sin(el), Math.cos(el) * Math.cos(az)];
      for (const { l, d: ld } of dirs) {
        const cos = d[0] * ld[0] + d[1] * ld[1] + d[2] * ld[2];
        const w = smooth(Math.cos(l.size), Math.cos(l.size * 0.4), cos);
        c[0] += l.color[0] * w;
        c[1] += l.color[1] * w;
        c[2] += l.color[2] * w;
      }
      rgb.set(c, (y * width + x) * 3);
    }
  }
  return { width, height, rgb };
}

export function presetEnvironment(name: string): EnvImage {
  switch (name) {
    case 'sunset':
      return generate(
        (el) => {
          const t = smooth(-0.1, 0.6, el);
          const ground = el < 0 ? 0.12 : 1;
          return [(1.3 * (1 - t) + 0.25 * t) * ground, (0.55 * (1 - t) + 0.35 * t) * ground, (0.3 * (1 - t) + 0.6 * t) * ground];
        },
        [{ az: -1.2, el: 0.08, size: 0.12, color: [40, 22, 9] }],
      );
    case 'overcast':
      return generate((el) => {
        const v = el < 0 ? 0.25 : 0.7 + 0.5 * Math.sin(el);
        return [v, v * 1.01, v * 1.05];
      }, []);
    case 'night':
      return generate(
        (el) => (el < 0 ? [0.01, 0.012, 0.02] : [0.02, 0.03, 0.07 + 0.05 * Math.sin(el)]),
        [{ az: 0.9, el: 0.7, size: 0.06, color: [6, 7, 9] }],
      );
    default:
      // A photo studio: a dim room, a large soft key at front left, a strip light at the right and a rim behind.
      return generate(
        (el) => {
          const v = el < 0 ? 0.08 + 0.04 * (1 + el) : 0.16 + 0.08 * Math.sin(el);
          return [v, v, v * 1.03];
        },
        [
          { az: -0.7, el: 0.55, size: 0.45, color: [4.5, 4.3, 4.0] },
          { az: 1.4, el: 0.2, size: 0.22, color: [2.2, 2.4, 2.7] },
          { az: 3.0, el: 0.4, size: 0.18, color: [3.0, 3.0, 3.0] },
        ],
      );
  }
}

/** Reads a Radiance RGBE (.hdr) file, flat or run-length encoded. */
export function parseHdr(buf: ArrayBuffer): EnvImage {
  const bytes = new Uint8Array(buf);
  let pos = 0;
  const line = (): string => {
    let s = '';
    while (pos < bytes.length && bytes[pos] !== 10) s += String.fromCharCode(bytes[pos++]);
    pos++;
    return s;
  };
  const first = line();
  if (!first.startsWith('#?')) throw new Error('not a Radiance HDR file');
  let size = '';
  for (;;) {
    const l = line();
    if (pos >= bytes.length) throw new Error('the HDR header has no size');
    if (/^[-+]Y \d+ [-+]X \d+$/.test(l.trim())) {
      size = l.trim();
      break;
    }
  }
  const m = /^([-+])Y (\d+) ([-+])X (\d+)$/.exec(size);
  if (!m) throw new Error('unsupported HDR orientation');
  const height = Number(m[2]);
  const width = Number(m[4]);
  if (width * height > 8192 * 4096) throw new Error('the HDR image is too large');
  const rgb = new Float32Array(width * height * 3);
  const scan = new Uint8Array(width * 4);
  for (let y = 0; y < height; y++) {
    if (width >= 8 && width < 32768 && bytes[pos] === 2 && bytes[pos + 1] === 2 && ((bytes[pos + 2] << 8) | bytes[pos + 3]) === width) {
      pos += 4;
      for (let c = 0; c < 4; c++) {
        let x = 0;
        while (x < width) {
          let n = bytes[pos++];
          if (n > 128) {
            n -= 128;
            const v = bytes[pos++];
            for (let k = 0; k < n; k++) scan[(x++) * 4 + c] = v;
          } else {
            for (let k = 0; k < n; k++) scan[(x++) * 4 + c] = bytes[pos++];
          }
        }
      }
    } else {
      scan.set(bytes.subarray(pos, pos + width * 4));
      pos += width * 4;
    }
    const row = m[1] === '-' ? y : height - 1 - y;
    for (let x = 0; x < width; x++) {
      const e = scan[x * 4 + 3];
      const f = e ? Math.pow(2, e - 136) : 0;
      const at = (row * width + (m[3] === '+' ? x : width - 1 - x)) * 3;
      rgb[at] = scan[x * 4] * f;
      rgb[at + 1] = scan[x * 4 + 1] * f;
      rgb[at + 2] = scan[x * 4 + 2] * f;
    }
  }
  return { width, height, rgb };
}

/** An ordinary image (PNG, JPEG, WebP) taken as an sRGB environment, brightened a little so it lights. */
export async function imageEnvironment(file: Blob): Promise<EnvImage> {
  const bitmap = await createImageBitmap(file);
  const width = Math.min(2048, bitmap.width);
  const height = Math.max(1, Math.round((width * bitmap.height) / bitmap.width));
  const canvas = new OffscreenCanvas(width, height);
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('no 2D canvas');
  ctx.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  const px = ctx.getImageData(0, 0, width, height).data;
  const rgb = new Float32Array(width * height * 3);
  for (let i = 0; i < width * height; i++) {
    for (let c = 0; c < 3; c++) rgb[i * 3 + c] = Math.pow(px[i * 4 + c] / 255, 2.2) * 1.5;
  }
  return { width, height, rgb };
}

/** Halves an image (box filter), down to 1 pixel high; level 0 is the image itself. */
export function mipChain(img: EnvImage): EnvImage[] {
  const out = [img];
  let cur = img;
  while (cur.height > 1 || cur.width > 1) {
    const w = Math.max(1, cur.width >> 1);
    const h = Math.max(1, cur.height >> 1);
    const rgb = new Float32Array(w * h * 3);
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        for (let c = 0; c < 3; c++) {
          let s = 0;
          for (const [dx, dy] of [[0, 0], [1, 0], [0, 1], [1, 1]]) {
            const sx = Math.min(cur.width - 1, x * 2 + dx);
            const sy = Math.min(cur.height - 1, y * 2 + dy);
            s += cur.rgb[(sy * cur.width + sx) * 3 + c];
          }
          rgb[(y * w + x) * 3 + c] = s / 4;
        }
      }
    }
    cur = { width: w, height: h, rgb };
    out.push(cur);
  }
  return out;
}
