// The browser engine: native/crates/fsc-wasm built to /engine/fsc_engine.wasm (./formsculptor engine wasm). The same
// command registry, mesh payloads and live strokes as the engine service, run in the page with no service behind it.
// Its C ABI: input is copied into memory from fsc_alloc and freed after the call; each call leaves its result in one
// output buffer read through fsc_out_ptr and fsc_out_len. Never touches the DOM.

interface Exports {
  memory: WebAssembly.Memory;
  fsc_init(): void;
  fsc_alloc(len: number): number;
  fsc_free(ptr: number, len: number): void;
  fsc_out_ptr(): number;
  fsc_out_len(): number;
  fsc_version(): void;
  fsc_catalogue(): void;
  fsc_dispatch(name: number, nameLen: number, args: number, argsLen: number): void;
  fsc_mesh(id: number, smooth: number): number;
  fsc_stroke_begin(id: number, settings: number, len: number): number;
  fsc_stroke_sample(ox: number, oy: number, oz: number, dx: number, dy: number, dz: number, pressure: number, tilt: number): number;
  fsc_stroke_end(): void;
  fsc_save(): number;
  fsc_open(data: number, len: number): void;
  fsc_import(name: number, nameLen: number, data: number, len: number): void;
  /** Absent from browser engines built before 0.92.0, which import without options. */
  fsc_import_with?(name: number, nameLen: number, data: number, len: number, opts: number, optsLen: number): void;
  fsc_export(name: number, nameLen: number, ids: number, idsLen: number): number;
  fsc_maps(id: number): number;
  fsc_boolean_preview(a: number, b: number, op: number): number;
}

export interface WasmReply {
  ok: boolean;
  result?: unknown;
  error?: { code: string; message: string };
}

const enc = new TextEncoder();
const dec = new TextDecoder();

export class WasmEngine {
  /** The last panic message, when the module stopped. */
  lastPanic = '';
  private dead = false;

  private constructor(private readonly x: Exports) {}

  /** Loads and starts the module, or returns null when it is not there or cannot run. */
  static async load(url = '/engine/fsc_engine.wasm'): Promise<WasmEngine | null> {
    try {
      const response = await fetch(url, { cache: 'no-cache' });
      if (!response.ok) return null;
      let engine: WasmEngine | null = null;
      const imports = {
        env: {
          fsc_now_ms: (): number => performance.now(),
          fsc_log: (ptr: number, len: number): void => {
            if (engine) engine.lastPanic = dec.decode(new Uint8Array(engine.x.memory.buffer, ptr, len).slice());
          },
        },
      };
      const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), imports);
      engine = new WasmEngine(instance.exports as unknown as Exports);
      engine.x.fsc_init();
      return engine;
    } catch {
      return null;
    }
  }

  /** A copy of the output buffer in its own ArrayBuffer (the next call replaces the output, and memory can grow). */
  private out(): Uint8Array<ArrayBuffer> {
    const copy = new Uint8Array(this.x.fsc_out_len());
    copy.set(new Uint8Array(this.x.memory.buffer, this.x.fsc_out_ptr(), copy.length));
    return copy;
  }

  private outText(): string {
    return dec.decode(this.out());
  }

  /** Runs `f` with each of `inputs` copied into the module's memory as (pointer, length), then frees them. */
  private with<T>(inputs: Uint8Array[], f: (...args: number[]) => T): T {
    if (this.dead) throw new Error(`the browser engine stopped${this.lastPanic ? `: ${this.lastPanic}` : ''}; reload the page`);
    const held = inputs.map((bytes) => {
      const ptr = this.x.fsc_alloc(bytes.length);
      new Uint8Array(this.x.memory.buffer, ptr, bytes.length).set(bytes);
      return [ptr, bytes.length] as const;
    });
    try {
      return f(...held.flat());
    } catch (err) {
      // A panic traps the module; its state can no longer be trusted.
      this.dead = true;
      throw new Error(`the browser engine stopped${this.lastPanic ? `: ${this.lastPanic}` : ` (${String(err)})`}; reload the page`);
    } finally {
      if (!this.dead) for (const [ptr, len] of held) this.x.fsc_free(ptr, len);
    }
  }

  private failed(): WasmReply {
    try {
      return { ok: false, error: JSON.parse(this.outText()) as { code: string; message: string } };
    } catch {
      return { ok: false, error: { code: 'engine', message: this.outText() } };
    }
  }

  version(): string {
    return this.with([], () => {
      this.x.fsc_version();
      return this.outText();
    });
  }

  catalogue(): unknown {
    return this.with([], () => {
      this.x.fsc_catalogue();
      return JSON.parse(this.outText());
    });
  }

  dispatch(name: string, args: Record<string, unknown>): WasmReply {
    try {
      return this.with([enc.encode(name), enc.encode(JSON.stringify(args))], (np, nl, ap, al) => {
        this.x.fsc_dispatch(np, nl, ap, al);
        return JSON.parse(this.outText()) as WasmReply;
      });
    } catch (err) {
      return { ok: false, error: { code: 'engine_stopped', message: String((err as Error).message ?? err) } };
    }
  }

  /** The FSCM payload of an object, or null when there is no such object. */
  mesh(id: number, smooth: number): ArrayBuffer | null {
    return this.with([], () => (this.x.fsc_mesh(id, smooth) === 0 ? this.out().buffer : null));
  }

  /** Starts a live stroke; null, or the refusal's message. */
  strokeBegin(id: number, settings: Record<string, unknown>): string | null {
    return this.with([enc.encode(JSON.stringify(settings))], (p, l) => (this.x.fsc_stroke_begin(id, p, l) === 0 ? null : (this.failed().error?.message ?? 'stroke refused')));
  }

  strokeSample(origin: number[], dir: number[], pressure: number, tilt: number): { hit: boolean; topology: boolean; update: ArrayBuffer | null; error?: string } {
    return this.with([], () => {
      const flags = this.x.fsc_stroke_sample(origin[0], origin[1], origin[2], dir[0], dir[1], dir[2], pressure, tilt);
      if (flags < 0) return { hit: false, topology: false, update: null, error: this.failed().error?.message };
      const update = this.out();
      return { hit: (flags & 1) === 1, topology: (flags & 2) === 2, update: update.length > 16 ? update.buffer : null };
    });
  }

  strokeEnd(): WasmReply {
    return this.with([], () => {
      this.x.fsc_stroke_end();
      return JSON.parse(this.outText()) as WasmReply;
    });
  }

  /** The scene as .fsc bytes, or the error's message. */
  save(): Uint8Array | string {
    return this.with([], () => (this.x.fsc_save() === 0 ? this.out() : (this.failed().error?.message ?? 'save failed')));
  }

  open(bytes: Uint8Array): WasmReply {
    return this.with([bytes], (p, l) => {
      this.x.fsc_open(p, l);
      return JSON.parse(this.outText()) as WasmReply;
    });
  }

  /** Imports a mesh file; options are import_mesh's (scale, up_axis, merge). */
  importFile(name: string, bytes: Uint8Array, options: Record<string, unknown> = {}): WasmReply {
    return this.with([enc.encode(name), bytes, enc.encode(JSON.stringify(options))], (np, nl, dp, dl, op, ol) => {
      if (this.x.fsc_import_with) this.x.fsc_import_with(np, nl, dp, dl, op, ol);
      else this.x.fsc_import(np, nl, dp, dl);
      return JSON.parse(this.outText()) as WasmReply;
    });
  }

  /** A file of the given name's format from the chosen objects (every visible one when null), with export_mesh's
   * options (scale, up_axis, triangulate), or the error's message. */
  exportFile(name: string, ids: number[] | null, options: Record<string, unknown> = {}): Uint8Array | string {
    const args = JSON.stringify(ids ? { ...options, ids } : options);
    return this.with([enc.encode(name), enc.encode(args)], (np, nl, ip, il) => (this.x.fsc_export(np, nl, ip, il) === 0 ? this.out() : (this.failed().error?.message ?? 'export failed')));
  }

  /** The live boolean preview of two objects as an FSCM payload, or the refusal's message. */
  booleanPreview(a: number, b: number, op: string): ArrayBuffer | string {
    const code = ['union', 'difference', 'intersection'].indexOf(op);
    return this.with([], () => (this.x.fsc_boolean_preview(a, b, code) === 0 ? this.out().buffer : (this.failed().error?.message ?? 'boolean failed')));
  }

  /** An object's baked maps as a zip, or the error's message. */
  maps(id: number): Uint8Array | string {
    return this.with([], () => (this.x.fsc_maps(id) === 0 ? this.out() : (this.failed().error?.message ?? 'no maps')));
  }
}
