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
  const paint = buffer.byteLength >= offset + count * 20 ? new Float32Array(buffer, offset, count * 5) : null;
  return { id, revision, indices, positions, normals, mask, paint };
}

export async function fetchVersion(signal?: AbortSignal): Promise<Version | null> {
  try {
    const response = await fetch('/api/version', { signal, cache: 'no-store' });
    if (!response.ok) return null;
    return (await response.json()) as Version;
  } catch {
    return null;
  }
}

export async function dispatch<T>(name: string, args: Record<string, unknown> = {}): Promise<Reply<T>> {
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

export async function fetchFile(path: string): Promise<Blob | string> {
  try {
    const response = await fetch(path, { cache: 'no-store' });
    if (!response.ok) return (await response.text()) || response.statusText;
    return await response.blob();
  } catch (err) {
    return String(err);
  }
}

export async function postJson<T>(path: string): Promise<Reply<T>> {
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
  const response = await fetch(`/api/engine/mesh/${id}${smooth > 0 ? `?smooth=${smooth}` : ''}`, { cache: 'no-store' });
  if (!response.ok) return null;
  const buffer = await response.arrayBuffer();
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
  const paint = (flags & 2) === 2 ? new Float32Array(buffer, offset, vertices * 5) : null;
  if (paint) offset += vertices * 20;
  const triangles = indexCount / 3;
  const groups = (flags & 4) === 4 ? new Uint32Array(buffer, offset, triangles) : null;
  if (groups) offset += triangles * 4;
  const hidden = (flags & 8) === 8 ? new Uint8Array(buffer, offset, triangles) : null;
  return { id, revision, positions, normals, indices, mask, paint, groups, hidden };
}
