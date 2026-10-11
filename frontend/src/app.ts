import { VERSION } from './version';
import { ICONS } from './ui/icons';
import { el, setAttr, setText } from './ui/dom';
import { dispatch, fetchMesh, fetchVersion, type ObjectSummary, type SceneSummary } from './engine/service';
import { Viewport } from './viewport/viewport';
import type { ViewName } from './viewport/camera';

interface Shell {
  engineDot: HTMLElement;
  engineText: HTMLElement;
  engineVersion: HTMLElement;
  canvas: HTMLCanvasElement;
  addButtons: HTMLButtonElement[];
  objectList: HTMLUListElement;
  statVerts: HTMLElement;
  statTris: HTMLElement;
  statRenderer: HTMLElement;
  statFrame: HTMLElement;
  message: HTMLElement;
}

const PRIMITIVES: [string, string][] = [
  ['sphere', 'Sphere'],
  ['cube', 'Cube'],
  ['cylinder', 'Cylinder'],
  ['cone', 'Cone'],
  ['torus', 'Torus'],
  ['plane', 'Plane'],
];

function buildShell(root: HTMLElement): Shell {
  const bar = el('header', 'appbar');
  const brand = el('div', 'brand');
  brand.innerHTML = ICONS.mark;
  brand.append(el('span', '', 'formsculptor'), el('span', 'ver', `v${VERSION}`));
  bar.append(brand, el('div', 'spacer'));

  const rail = el('nav', 'rail');
  rail.setAttribute('aria-label', 'Tools');

  const stage = el('main', 'stage');
  const canvas = el('canvas', 'viewport');
  canvas.setAttribute('aria-label', 'Viewport');
  canvas.tabIndex = 0;
  stage.append(canvas);

  const panel = el('aside', 'panel');
  panel.append(el('h2', '', 'Add'));
  const grid = el('div', 'add-grid');
  const addButtons = PRIMITIVES.map(([kind, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.kind = kind;
    grid.append(b);
    return b;
  });
  panel.append(grid);

  const message = el('p', 'panel-note', '');
  panel.append(message);

  panel.append(el('h2', 'gap', 'Active mesh'));
  const stats = el('dl', 'kv');
  const statVerts = el('dd', '', '0');
  const statTris = el('dd', '', '0');
  stats.append(el('dt', '', 'Vertices'), statVerts, el('dt', '', 'Triangles'), statTris);
  panel.append(stats);

  panel.append(el('h2', 'gap', 'Engine'));
  const kv = el('dl', 'kv');
  const engineVersion = el('dd', '', '…');
  const statRenderer = el('dd', '', '…');
  const statFrame = el('dd', '', '…');
  kv.append(el('dt', '', 'App'), el('dd', '', VERSION), el('dt', '', 'Service'), engineVersion, el('dt', '', 'Renderer'), statRenderer, el('dt', '', 'Frame'), statFrame);
  panel.append(kv);

  panel.append(el('h2', 'gap', 'Scene'));
  const objectList = el('ul', 'objects');
  objectList.setAttribute('aria-label', 'Objects');
  panel.append(objectList);

  const status = el('footer', 'status');
  const state = el('span', 'engine-state');
  const engineDot = el('i', 'dot');
  const engineText = el('span', '', 'connecting');
  state.append(engineDot, engineText);
  status.append(state);

  root.append(bar, rail, stage, panel, status);
  return { engineDot, engineText, engineVersion, canvas, addButtons, objectList, statVerts, statTris, statRenderer, statFrame, message };
}

class SceneView {
  private rows = new Map<number, HTMLLIElement>();
  private scene: SceneSummary = { objects: [], active: null };

  constructor(private shell: Shell, private viewport: Viewport) {
    shell.objectList.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (row?.dataset.id) void this.run('select_object', { id: Number(row.dataset.id) });
    });
    shell.addButtons.forEach((b) => b.addEventListener('click', () => void this.add(b.dataset.kind ?? 'sphere')));
    shell.canvas.addEventListener('keydown', (e) => this.key(e));
  }

  async refresh(): Promise<void> {
    const reply = await dispatch<SceneSummary>('get_scene');
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    await this.apply(reply.result);
  }

  private async apply(next: SceneSummary): Promise<void> {
    for (const o of next.objects) {
      if (!this.viewport.renderer.has(o.id, o.revision)) {
        const mesh = await fetchMesh(o.id);
        if (mesh) this.viewport.setMesh(mesh);
      }
    }
    const keep = new Set(next.objects.map((o) => o.id));
    for (const id of this.viewport.renderer.ids()) if (!keep.has(id)) this.viewport.removeMesh(id);
    this.viewport.setActive(next.active);
    this.patchList(next);
    const active = next.objects.find((o) => o.id === next.active);
    setText(this.shell.statVerts, active ? active.vertices.toLocaleString('en-US') : '0');
    setText(this.shell.statTris, active ? active.triangles.toLocaleString('en-US') : '0');
    this.scene = next;
  }

  private patchList(next: SceneSummary): void {
    const keep = new Set(next.objects.map((o) => o.id));
    for (const [id, row] of this.rows) {
      if (!keep.has(id)) {
        row.remove();
        this.rows.delete(id);
      }
    }
    for (const o of next.objects) {
      let row = this.rows.get(o.id);
      if (!row) {
        row = el('li', 'object', '');
        row.dataset.id = String(o.id);
        row.append(el('span', 'name', o.name), el('span', 'count', ''));
        this.shell.objectList.append(row);
        this.rows.set(o.id, row);
      }
      setText(row.children[0], o.name);
      setText(row.children[1], `${o.triangles.toLocaleString('en-US')} tris`);
      setAttr(row, 'aria-current', o.id === next.active ? 'true' : 'false');
    }
  }

  private note(text: string): void {
    setText(this.shell.message, text);
  }

  private async run(name: string, args: Record<string, unknown> = {}): Promise<void> {
    const reply = await dispatch<unknown>(name, args);
    this.note(reply.ok ? '' : reply.error.message);
    await this.refresh();
  }

  private async add(kind: string): Promise<void> {
    const reply = await dispatch<{ object: ObjectSummary }>('add_primitive', { kind });
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    this.note('');
    const first = this.scene.objects.length === 0;
    await this.refresh();
    if (first) this.frameActive();
  }

  private frameActive(): void {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (o) this.viewport.frameBounds(o.bounds.min, o.bounds.max);
  }

  private key(e: KeyboardEvent): void {
    const views: Record<string, [ViewName, ViewName]> = { '1': ['front', 'back'], '3': ['right', 'left'], '7': ['top', 'bottom'] };
    const pair = views[e.key];
    if (pair) this.viewport.snap(pair[e.ctrlKey ? 1 : 0]);
    else if (e.key === '5') this.viewport.toggleOrtho();
    else if (e.key === 'f' || e.key === 'F') this.frameActive();
    else if ((e.key === 'Delete' || e.key === 'Backspace') && this.scene.active !== null) void this.run('delete_object', { id: this.scene.active });
    else return;
    e.preventDefault();
  }
}

async function poll(shell: Shell): Promise<void> {
  const version = await fetchVersion();
  if (version) {
    setAttr(shell.engineDot, 'data-state', 'ok');
    setText(shell.engineText, 'service connected');
    setText(shell.engineVersion, version.version);
  } else {
    setAttr(shell.engineDot, 'data-state', 'down');
    setText(shell.engineText, 'service unreachable');
    setText(shell.engineVersion, 'offline');
  }
}

export function start(root: HTMLElement): void {
  const shell = buildShell(root);
  document.body.classList.add('app-ready');
  try {
    const viewport = new Viewport(shell.canvas);
    setText(shell.statRenderer, viewport.renderer.kind);
    viewport.onFrame = (s) => setText(shell.statFrame, `${s.frameMs.toFixed(1)} ms`);
    void new SceneView(shell, viewport).refresh();
  } catch (err) {
    setText(shell.statRenderer, 'unavailable');
    setText(shell.message, `The viewport could not start: ${String(err)}`);
  }
  void poll(shell);
  window.setInterval(() => void poll(shell), 5000);
}
