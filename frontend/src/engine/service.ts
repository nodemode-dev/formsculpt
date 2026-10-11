export interface Version {
  app: string;
  version: string;
}

export interface ObjectSummary {
  id: number;
  name: string;
  visible: boolean;
  vertices: number;
  triangles: number;
  revision: number;
  level: number;
  levels: number;
  layers: { name: string; strength: number }[];
  active_layer: number | null;
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
}

export interface VertexUpdate {
  id: number;
  revision: number;
  indices: Uint32Array;
  positions: Float32Array;
  normals: Float32Array;
  mask: Float32Array;
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
  return { id, revision, indices, positions, normals, mask };
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

export async function fetchMesh(id: number): Promise<MeshBuffers | null> {
  const response = await fetch(`/api/engine/mesh/${id}`, { cache: 'no-store' });
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
  const hasMask = (head.getUint32(16, true) & 1) === 1;
  const mask = hasMask ? new Float32Array(buffer, offset, vertices) : new Float32Array(vertices);
  return { id, revision, positions, normals, indices, mask };
}
