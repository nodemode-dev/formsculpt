/** Light colour from a colour temperature in kelvin: the black-body locus as Kang et al. (2002) fit it in CIE 1931 xy,
 * taken to linear Rec. 709 and scaled so the brightest channel is 1. The engine's path render uses the same fit
 * (native fsc-paint kelvin.rs), so a light's colour matches in both. */

export const KELVIN_MIN = 1667;
export const KELVIN_MAX = 25000;

/** The linear colour of light at `kelvin` (clamped to 1667 to 25000), brightest channel 1. */
export function kelvinColor(kelvin: number): [number, number, number] {
  const t = Math.min(KELVIN_MAX, Math.max(KELVIN_MIN, kelvin));
  const [t2, t3] = [t * t, t * t * t];
  const x = t <= 4000 ? -0.2661239e9 / t3 - 0.2343589e6 / t2 + 0.8776956e3 / t + 0.17991 : -3.0258469e9 / t3 + 2.1070379e6 / t2 + 0.2226347e3 / t + 0.24039;
  const [x2, x3] = [x * x, x * x * x];
  const y =
    t <= 2222
      ? -1.1063814 * x3 - 1.3481102 * x2 + 2.18555832 * x - 0.20219683
      : t <= 4000
        ? -0.9549476 * x3 - 1.37418593 * x2 + 2.09137015 * x - 0.16748867
        : 3.081758 * x3 - 5.8733867 * x2 + 3.75112997 * x - 0.37001483;
  const [cx, cy, cz] = [x / y, 1, (1 - x - y) / y];
  const rgb = [3.2404542 * cx - 1.5371385 * cy - 0.4985314 * cz, -0.969266 * cx + 1.8760108 * cy + 0.041556 * cz, 0.0556434 * cx - 0.2040259 * cy + 1.0572252 * cz].map((v) => Math.max(0, v));
  const top = Math.max(1e-6, ...rgb);
  return [rgb[0] / top, rgb[1] / top, rgb[2] / top];
}

/** A linear colour as a #rrggbb hex for a colour field (gamma 2.2, the inverse of the window's hexToLinear). */
export function linearToHex(c: [number, number, number]): string {
  const enc = (v: number): string => Math.round(Math.pow(Math.min(1, Math.max(0, v)), 1 / 2.2) * 255).toString(16).padStart(2, '0');
  return `#${enc(c[0])}${enc(c[1])}${enc(c[2])}`;
}
