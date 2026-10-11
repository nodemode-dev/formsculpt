import { parseUpdate, type VertexUpdate } from './service';

export interface BrushState {
  brush: string;
  radiusPx: number;
  strength: number;
  invert: boolean;
  falloff: string;
  lazy: number;
  symmetry: { x: boolean; y: boolean; z: boolean; radial: number };
  dyntopo: number | null;
  dyntopoSize: number | null;
  dyntopoScreenPx: number | null;
}

export interface StrokeResult {
  id: number;
  dabs: number;
  vertices_changed: number;
  revision: number;
}

type Vec3 = [number, number, number];

export class StrokeClient {
  private socket: WebSocket | null = null;
  private opening: Promise<WebSocket> | null = null;
  private seq = 0;
  private firstSeq = -1;
  private miss = false;
  private live = false;
  onUpdate: ((update: VertexUpdate) => void) | null = null;
  onEnd: ((result: StrokeResult | null) => void) | null = null;
  onError: ((message: string) => void) | null = null;
  onTopology: ((objectId: number) => void) | null = null;
  private objectId = 0;

  private open(): Promise<WebSocket> {
    if (this.socket && this.socket.readyState === WebSocket.OPEN) return Promise.resolve(this.socket);
    if (this.opening) return this.opening;
    this.opening = new Promise((resolve, reject) => {
      const url = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/api/engine/stroke`;
      const ws = new WebSocket(url);
      ws.binaryType = 'arraybuffer';
      ws.onopen = () => {
        this.socket = ws;
        this.opening = null;
        resolve(ws);
      };
      ws.onerror = () => {
        this.opening = null;
        reject(new Error('the stroke channel could not open'));
      };
      ws.onclose = () => {
        if (this.socket === ws) this.socket = null;
      };
      ws.onmessage = (e) => this.receive(e.data);
    });
    return this.opening;
  }

  warm(): void {
    void this.open().catch(() => undefined);
  }

  private receive(data: unknown): void {
    if (data instanceof ArrayBuffer) {
      const update = parseUpdate(data);
      if (update) this.onUpdate?.(update);
      return;
    }
    let msg: { type: string; seq?: number; hit?: boolean; topology?: boolean; result?: { ok: boolean; result: StrokeResult | null }; message?: string };
    try {
      msg = JSON.parse(String(data));
    } catch {
      return;
    }
    if (msg.type === 'sample' && msg.topology) this.onTopology?.(this.objectId);
    if (msg.type === 'sample' && msg.seq === this.firstSeq && msg.hit === false) this.miss = true;
    else if (msg.type === 'end') this.onEnd?.(msg.result?.ok ? msg.result.result : null);
    else if (msg.type === 'error') this.onError?.(msg.message ?? 'stroke error');
  }

  private send(payload: Record<string, unknown>): void {
    void this.open()
      .then((ws) => ws.send(JSON.stringify(payload)))
      .catch((err: Error) => this.onError?.(err.message));
  }

  begin(objectId: number, brush: BrushState, worldPerPixel: number, invert: boolean, origin: Vec3, dir: Vec3, pressure: number): void {
    this.miss = false;
    this.live = true;
    this.objectId = objectId;
    const radius = Math.max(1e-4, brush.radiusPx * worldPerPixel);
    const size = brush.dyntopoScreenPx !== null ? Math.max(1e-5, brush.dyntopoScreenPx * worldPerPixel) : brush.dyntopoSize;
    this.send({
      type: 'begin',
      id: objectId,
      brush: { brush: brush.brush, radius, strength: brush.strength, invert: brush.invert !== invert, falloff: brush.falloff, lazy: brush.lazy, symmetry: brush.symmetry, dyntopo: brush.dyntopo, dyntopo_size: size },
    });
    this.firstSeq = ++this.seq;
    this.send({ type: 'sample', seq: this.firstSeq, origin, dir, pressure });
  }

  move(origin: Vec3, dir: Vec3, pressure: number): void {
    if (!this.live) return;
    this.send({ type: 'sample', seq: ++this.seq, origin, dir, pressure });
  }

  end(): void {
    if (!this.live) return;
    this.live = false;
    this.send({ type: 'end' });
  }

  missed(): boolean {
    return this.miss;
  }

  active(): boolean {
    return this.live;
  }
}
