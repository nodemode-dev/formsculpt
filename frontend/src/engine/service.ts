import type { WasmEngine } from './wasm';

/** The browser engine when the window runs without the engine service; null while the service is in use. */
let local: WasmEngine | null = null;

/** Sends every engine call from here on to the browser engine instead of the service. */
export function useBrowserEngine(engine: WasmEngine): void {
  local = engine;
}

/** The browser engine in use, or null when the window talks to the engine service. */
export function browserEngine(): WasmEngine | null {
  return local;
}

const NEEDS_SERVICE = 'This needs the engine service (./formsculptor boot); the window is running the browser engine';

export interface Version {
  app: string;
  version: string;
}

export interface ObjectSummary {
  id: number;
  name: string;
  visible: boolean;
  folder?: string;
  vertices: number;
  triangles: number;
  revision: number;
  level: number;
  levels: number;
  layers: { name: string; strength: number }[];
  active_layer: number | null;
  /** Paint layers, bottom first, and the one painted into (null: the base). */
  paint_layers?: { name: string; blend: string; opacity: number; visible: boolean }[];
  active_paint_layer?: number | null;
  /** The settings a primitive was made with, while it can still be edited (null once anything else changed it). */
  primitive?: Record<string, unknown> | null;
  /** The source a linked copy follows (null once it is unlinked, or for an ordinary object). */
  instance_of?: number | null;
  /** "tube" or "lathe" while edit_build can still remake the object (null once anything else changed it). */
  editable_build?: string | null;
  /** Whether the object has UVs (uv_unwrap; topology changes clear them). */
  uvs?: boolean;
  material?: { subsurface: number; subsurface_color: [number, number, number]; subsurface_radius: number; transmission: number; ior: number };
  bounds: { min: [number, number, number]; max: [number, number, number] };
}

export interface SceneSummary {
  objects: ObjectSummary[];
  active: number | null;
}

export interface CommandError {
  code: string;
  message: string;
}

export type Reply<T> = { ok: true; result: T } | { ok: false; error: CommandError };

/** Floats of paint per vertex in the engine's buffers: colour, roughness, metalness, opacity, emissive. */
export const PAINT_FLOATS = 7;

export interface MeshBuffers {
  id: number;
  revision: number;
  positions: Float32Array;
  normals: Float32Array;
  indices: Uint32Array;
  mask: Float32Array;
  paint: Float32Array | null;
  /** Face group per triangle, or null when the object has none. */
  groups?: Uint32Array | null;
  /** Hidden flag per triangle (non-zero hides it), or null when nothing is hidden. */
  hidden?: Uint8Array | null;
}

export interface VertexUpdate {
  id: number;
  revision: number;
  indices: Uint32Array;
  positions: Float32Array;
  normals: Float32Array;
  mask: Float32Array;
  paint: Float32Array | null;
}

export function parseUpdate(buffer: ArrayBuffer): VertexUpdate | null {
  const head = new DataView(buffer, 0, 16);
  if (String.fromCharCode(head.getUint8(0), head.getUint8(1), head.getUint8(2), head.getUint8(3)) !== 'FSCU') return null;
  const count = head.getUint32(4, true);
  const revision = head.getUint32(8, true);
  const id = head.getUint32(12, true);
  let offset = 16;
  const indices = new Uint32Array(buffer, offset, count);
  offset += count * 4;
  const positions = new Float32Array(buffer, offset, count * 3);
  offset += count * 12;
  const normals = new Float32Array(buffer, offset, count * 3);
  offset += count * 12;
  const mask = new Float32Array(buffer, offset, count);
  offset += count * 4;
  const paint = buffer.byteLength >= offset + count * PAINT_FLOATS * 4 ? new Float32Array(buffer, offset, count * PAINT_FLOATS) : null;
  return { id, revision, indices, positions, normals, mask, paint };
}

export async function fetchVersion(signal?: AbortSignal): Promise<Version | null> {
  if (local) return { app: 'formsculptor', version: local.version() };
  try {
    const response = await fetch('/api/version', { signal, cache: 'no-store' });
    if (!response.ok) return null;
    return (await response.json()) as Version;
  } catch {
    return null;
  }
}

export async function dispatch<T>(name: string, args: Record<string, unknown> = {}): Promise<Reply<T>> {
  if (local) return local.dispatch(name, args) as Reply<T>;
  try {
    const response = await fetch('/api/engine/dispatch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name, args }),
    });
    if (!response.ok) {
      const detail = await response.text();
      return { ok: false, error: { code: `http_${response.status}`, message: detail || response.statusText } };
    }
    return (await response.json()) as Reply<T>;
  } catch (err) {
    return { ok: false, error: { code: 'unreachable', message: String(err) } };
  }
}

export async function uploadFile<T>(file: Blob, name: string): Promise<Reply<T>> {
  if (local) {
    const bytes = new Uint8Array(await file.arrayBuffer());
    return (name.toLowerCase().endsWith('.fsc') ? local.open(bytes) : local.importFile(name, bytes)) as Reply<T>;
  }
  try {
    const form = new FormData();
    form.append('file', file, name);
    const response = await fetch('/api/engine/import', { method: 'POST', body: form });
    if (!response.ok) return { ok: false, error: { code: `http_${response.status}`, message: (await response.text()) || response.statusText } };
    return (await response.json()) as Reply<T>;
  } catch (err) {
    return { ok: false, error: { code: 'unreachable', message: String(err) } };
  }
}

/** A service file route answered by the browser engine: the project, exports, baked maps and view pictures. */
function localFile(engine: WasmEngine, path: string): Blob | string {
  const url = new URL(path, location.origin);
  const q = url.searchParams;
  const bytes = (r: Uint8Array | string, type: string): Blob | string => (typeof r === 'string' ? r : new Blob([r as BlobPart], { type }));
  if (url.pathname === '/api/engine/project') return bytes(engine.save(), 'application/x-formsculptor');
  if (url.pathname === '/api/engine/export') {
    const ids = q.get('ids');
    return bytes(engine.exportFile(`export.${q.get('format') ?? 'glb'}`, ids ? ids.split(',').map(Number) : null), 'application/octet-stream');
  }
  const maps = /^\/api\/engine\/maps\/(\d+)$/.exec(url.pathname);
  if (maps) return bytes(engine.maps(Number(maps[1])), 'application/zip');
  if (url.pathname === '/api/engine/view') {
    const args: Record<string, unknown> = {};
    for (const [k, v] of q) args[k] = k === 'width' || k === 'height' ? Number(v) : v;
    const reply = engine.dispatch('get_view', args);
    if (!reply.ok) return reply.error?.message ?? 'render failed';
    const raw = atob((reply.result as { png_base64: string }).png_base64);
    const png = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i++) png[i] = raw.charCodeAt(i);
    return new Blob([png], { type: 'image/png' });
  }
  return NEEDS_SERVICE;
}

export async function fetchFile(path: string): Promise<Blob | string> {
  if (local) return localFile(local, path);
  try {
    const response = await fetch(path, { cache: 'no-store' });
    if (!response.ok) return (await response.text()) || response.statusText;
    return await response.blob();
  } catch (err) {
    return String(err);
  }
}

export async function postJson<T>(path: string): Promise<Reply<T>> {
  if (local) return { ok: false, error: { code: 'no_service', message: NEEDS_SERVICE } };
  try {
    const response = await fetch(path, { method: 'POST' });
    if (!response.ok) return { ok: false, error: { code: `http_${response.status}`, message: (await response.text()) || response.statusText } };
    return (await response.json()) as Reply<T>;
  } catch (err) {
    return { ok: false, error: { code: 'unreachable', message: String(err) } };
  }
}

/** POSTs JSON to a plain service route (not the dispatcher); a refusal's FastAPI `detail` becomes the message. */
export async function postBody<T>(path: string, body: unknown): Promise<Reply<T>> {
  if (local) return { ok: false, error: { code: 'no_service', message: NEEDS_SERVICE } };
  try {
    const response = await fetch(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
    const text = await response.text();
    if (!response.ok) {
      let message = text || response.statusText;
      try {
        const detail = (JSON.parse(text) as { detail?: unknown }).detail;
        if (typeof detail === 'string') message = detail;
      } catch {
        // not JSON; keep the text
      }
      return { ok: false, error: { code: `http_${response.status}`, message } };
    }
    return { ok: true, result: JSON.parse(text) as T };
  } catch (err) {
    return { ok: false, error: { code: 'unreachable', message: String(err) } };
  }
}

/** An object's mesh; with `smooth` 1 to 3, a dynamic-subdivision preview of it for display (the sculpted mesh unchanged). */
export async function fetchMesh(id: number, smooth = 0): Promise<MeshBuffers | null> {
  if (local) {
    const buffer = local.mesh(id, smooth);
    return buffer ? parseMesh(id, buffer) : null;
  }
  const response = await fetch(`/api/engine/mesh/${id}${smooth > 0 ? `?smooth=${smooth}` : ''}`, { cache: 'no-store' });
  if (!response.ok) return null;
  return parseMesh(id, await response.arrayBuffer());
}

/** The engine's catalogue of commands (for keys and menus), or null when it cannot be reached. */
export async function fetchCatalogue<T>(): Promise<T | null> {
  if (local) return local.catalogue() as T;
  try {
    const res = await fetch('/api/engine/catalogue');
    return res.ok ? ((await res.json()) as T) : null;
  } catch {
    return null;
  }
}

/** An FSCM payload (as the service sends it, or the browser engine builds it) as typed arrays over its bytes. */
function parseMesh(id: number, buffer: ArrayBuffer): MeshBuffers | null {
  const head = new DataView(buffer, 0, 20);
  const magic = String.fromCharCode(head.getUint8(0), head.getUint8(1), head.getUint8(2), head.getUint8(3));
  if (magic !== 'FSCM') return null;
  const vertices = head.getUint32(4, true);
  const indexCount = head.getUint32(8, true);
  const revision = head.getUint32(12, true);
  let offset = 20;
  const positions = new Float32Array(buffer, offset, vertices * 3);
  offset += vertices * 12;
  const normals = new Float32Array(buffer, offset, vertices * 3);
  offset += vertices * 12;
  const indices = new Uint32Array(buffer, offset, indexCount);
  offset += indexCount * 4;
  const flags = head.getUint32(16, true);
  const hasMask = (flags & 1) === 1;
  const mask = hasMask ? new Float32Array(buffer, offset, vertices) : new Float32Array(vertices);
  if (hasMask) offset += vertices * 4;
  const paint = (flags & 2) === 2 ? new Float32Array(buffer, offset, vertices * PAINT_FLOATS) : null;
  if (paint) offset += vertices * PAINT_FLOATS * 4;
  const triangles = indexCount / 3;
  const groups = (flags & 4) === 4 ? new Uint32Array(buffer, offset, triangles) : null;
  if (groups) offset += triangles * 4;
  const hidden = (flags & 8) === 8 ? new Uint8Array(buffer, offset, triangles) : null;
  return { id, revision, positions, normals, indices, mask, paint, groups, hidden };
}
