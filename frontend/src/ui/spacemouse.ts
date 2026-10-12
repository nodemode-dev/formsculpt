/**
 * 3D mouse support: 3Dconnexion devices (SpaceMouse, SpaceNavigator) read over WebHID. A device reports six axes as
 * signed 16-bit values: report 1 carries translation (and, on newer devices, rotation too, 12 bytes), report 2 carries
 * rotation. Pushing the cap pans, pulling or pushing it along its depth zooms, and tilting or twisting it turns the view,
 * every animation frame while it is held off centre.
 */

export interface SixAxis {
  tx: number;
  ty: number;
  tz: number;
  rx: number;
  ry: number;
  rz: number;
}

/** What a frame of motion does to the view: an orbit in pixels' worth, a pan in pixels, and a zoom factor. */
export interface ViewMotion {
  orbitX: number;
  orbitY: number;
  panX: number;
  panY: number;
  zoom: number;
}

const VENDORS = [0x256f, 0x046d];
const DEAD = 12;
const FULL = 350;

/** Folds one input report into the state; returns whether it was a motion report. */
export function readReport(id: number, data: DataView, state: SixAxis): boolean {
  const at = (i: number): number => (data.byteLength >= i + 2 ? data.getInt16(i, true) : 0);
  if (id === 1) {
    state.tx = at(0);
    state.ty = at(2);
    state.tz = at(4);
    if (data.byteLength >= 12) {
      state.rx = at(6);
      state.ry = at(8);
      state.rz = at(10);
    }
    return true;
  }
  if (id === 2) {
    state.rx = at(0);
    state.ry = at(2);
    state.rz = at(4);
    return true;
  }
  return false;
}

/** Maps the six axes to view motion for `dt` seconds, with a dead zone so a resting cap does not drift. */
export function motionOf(s: SixAxis, dt: number): ViewMotion {
  const f = (v: number): number => (Math.abs(v) < DEAD ? 0 : (v - Math.sign(v) * DEAD) / (FULL - DEAD));
  const k = Math.min(dt, 0.05) * 60;
  return {
    orbitX: -f(s.rz) * 14 * k,
    orbitY: f(s.rx) * 14 * k,
    panX: f(s.tx) * 18 * k,
    panY: -f(s.tz) * 18 * k,
    zoom: 1 + f(s.ty) * 0.04 * k,
  };
}

export class SpaceMouse {
  readonly state: SixAxis = { tx: 0, ty: 0, tz: 0, rx: 0, ry: 0, rz: 0 };
  private frame = 0;
  private last = 0;
  name = '';

  constructor(private apply: (m: ViewMotion) => void) {}

  static supported(): boolean {
    return typeof navigator !== 'undefined' && 'hid' in navigator;
  }

  /** Asks for a 3Dconnexion device (the browser shows its picker), opens it and starts listening. */
  async connect(): Promise<string> {
    const hid = (navigator as Navigator & { hid: HidLike }).hid;
    const [device] = await hid.requestDevice({ filters: VENDORS.map((vendorId) => ({ vendorId })) });
    if (!device) throw new Error('no 3D mouse was chosen');
    if (!device.opened) await device.open();
    device.addEventListener('inputreport', (e: Event) => {
      const r = e as Event & { reportId: number; data: DataView };
      this.feed(r.reportId, r.data);
    });
    this.name = device.productName || '3D mouse';
    return this.name;
  }

  /** One input report; motion starts the frame loop, which stops once the cap rests at centre. */
  feed(id: number, data: DataView): void {
    if (!readReport(id, data, this.state) || this.frame) return;
    this.last = performance.now();
    this.frame = requestAnimationFrame((t) => this.tick(t));
  }

  private tick(now: number): void {
    const dt = (now - this.last) / 1000;
    this.last = now;
    const m = motionOf(this.state, dt);
    if (m.orbitX === 0 && m.orbitY === 0 && m.panX === 0 && m.panY === 0 && m.zoom === 1) {
      this.frame = 0;
      return;
    }
    this.apply(m);
    this.frame = requestAnimationFrame((t) => this.tick(t));
  }
}

/** The part of WebHID this uses. */
interface HidDeviceLike extends EventTarget {
  opened: boolean;
  productName: string;
  open(): Promise<void>;
}
interface HidLike {
  requestDevice(o: { filters: { vendorId: number }[] }): Promise<HidDeviceLike[]>;
}
