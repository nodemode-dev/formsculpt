import { VERSION } from './version';
import { ICONS } from './ui/icons';
import { el, setAttr, setText } from './ui/dom';
import { dispatch, fetchMesh, fetchVersion, type ObjectSummary, type SceneSummary } from './engine/service';
import { Viewport } from './viewport/viewport';
import type { ViewName } from './viewport/camera';
import { StrokeClient, type BrushState } from './engine/stroke-client';

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
  brushButtons: HTMLButtonElement[];
  radius: HTMLInputElement;
  strength: HTMLInputElement;
  invert: HTMLInputElement;
  symX: HTMLInputElement;
  radial: HTMLInputElement;
  actionButtons: HTMLButtonElement[];
}

const BRUSHES: [string, string][] = [
  ['orbit', 'Orbit'],
  ['draw', 'Draw'],
  ['clay', 'Clay'],
  ['inflate', 'Inflate'],
  ['pinch', 'Pinch'],
  ['crease', 'Crease'],
  ['flatten', 'Flatten'],
  ['smooth', 'Smooth'],
  ['grab', 'Grab'],
  ['mask', 'Mask'],
];

const ACTIONS: [string, string][] = [
  ['undo', 'Undo'],
  ['redo', 'Redo'],
  ['mask_invert', 'Invert mask'],
  ['mask_blur', 'Blur mask'],
  ['mask_sharpen', 'Sharpen mask'],
  ['mask_clear', 'Clear mask'],
];

function field(label: string, input: HTMLInputElement): HTMLLabelElement {
  const row = el('label', 'field');
  row.append(el('span', '', label), input);
  return row;
}

function input(type: string, attrs: Record<string, string>): HTMLInputElement {
  const node = el('input');
  node.type = type;
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
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

  panel.append(el('h2', 'gap', 'Brush'));
  const brushGrid = el('div', 'add-grid');
  const brushButtons = BRUSHES.map(([kind, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.brush = kind;
    b.setAttribute('aria-pressed', kind === 'draw' ? 'true' : 'false');
    brushGrid.append(b);
    return b;
  });
  panel.append(brushGrid);
  const radius = input('range', { min: '4', max: '300', value: '60', 'aria-label': 'Radius' });
  const strength = input('range', { min: '0', max: '100', value: '50', 'aria-label': 'Strength' });
  const invert = input('checkbox', { 'aria-label': 'Invert' });
  const symX = input('checkbox', { 'aria-label': 'Mirror X' });
  const radial = input('number', { min: '1', max: '150', value: '1', 'aria-label': 'Radial' });
  const fields = el('div', 'fields');
  fields.append(field('Radius', radius), field('Strength', strength), field('Invert (Ctrl)', invert), field('Mirror X', symX), field('Radial', radial));
  panel.append(fields);

  panel.append(el('h2', 'gap', 'Edit'));
  const actionGrid = el('div', 'add-grid two');
  const actionButtons = ACTIONS.map(([name, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = name;
    actionGrid.append(b);
    return b;
  });
  panel.append(actionGrid);

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
  return { engineDot, engineText, engineVersion, canvas, addButtons, objectList, statVerts, statTris, statRenderer, statFrame, message, brushButtons, radius, strength, invert, symX, radial, actionButtons };
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
    shell.brushButtons.forEach((b) =>
      b.addEventListener('click', () => {
        this.brush = b.dataset.brush ?? 'draw';
        shell.brushButtons.forEach((x) => setAttr(x, 'aria-pressed', x === b ? 'true' : 'false'));
      }),
    );
    shell.actionButtons.forEach((b) => b.addEventListener('click', () => void this.run(b.dataset.action ?? 'undo')));
    this.strokes.onUpdate = (u) => this.viewport.patchMesh(u);
    this.strokes.onEnd = (r) => {
      if (r) this.viewport.renderer.setRevision(r.id, r.revision);
      void this.refresh();
    };
    this.strokes.onError = (m) => this.note(m);
    this.strokes.warm();
    viewport.strokes = {
      enabled: () => this.brush !== 'orbit' && this.scene.active !== null,
      begin: (ray, pressure, wpp, inv) => {
        if (this.scene.active === null) return;
        this.strokes.begin(this.scene.active, this.brushState(), wpp, inv, ray.origin, ray.dir, pressure);
      },
      move: (ray, pressure) => this.strokes.move(ray.origin, ray.dir, pressure),
      end: () => this.strokes.end(),
      missed: () => this.strokes.missed(),
    };
  }

  private brush = 'draw';
  private strokes = new StrokeClient();

  private brushState(): BrushState {
    const s = this.shell;
    return {
      brush: this.brush,
      radiusPx: Number(s.radius.value),
      strength: Number(s.strength.value) / 100,
      invert: s.invert.checked,
      falloff: 'smooth',
      lazy: 0,
      symmetry: { x: s.symX.checked, y: false, z: false, radial: Math.max(1, Math.min(150, Number(s.radial.value) || 1)) },
    };
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
    else if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z')) void this.run(e.shiftKey ? 'redo' : 'undo');
    else if ((e.ctrlKey || e.metaKey) && e.key === 'y') void this.run('redo');
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
