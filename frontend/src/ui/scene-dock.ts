// The Objects and Layers dock of the tablet layout: a floating panel on the left with two tabs. Objects lists the
// scene (choose one to make it active, show or hide it, lock it against strokes) with add, duplicate, rename and
// delete; Layers lists the active object's sculpt layers (choose where strokes record, show or hide a layer, drag its
// strength) with add, apply and delete. Rows are patched in place from each scene summary, so nothing is rebuilt
// while nothing changes; every action is an engine command or a registry command, so it is one undo step and an
// agent sees it.

import type { ObjectSummary, SceneSummary } from '../engine/service';
import { el, setAttr, setText } from './dom';
import { t } from './i18n';
import { glyph, iconOnly, labelled } from './tablet-layout';
import { folded, objectsUnder, treeOrder, wireTreeDrag } from './scene-tree';

export interface DockHooks {
  /** Runs an engine command and refreshes the window afterwards. */
  run: (name: string, args: Record<string, unknown>) => Promise<void>;
  isLocked: (id: number) => boolean;
  toggleLock: (id: number) => void;
  /** Moves the view to a saved view; saves the current view under a new name; deletes one. */
  recallView: (name: string) => void;
  saveView: () => void;
  deleteView: (name: string) => void;
}

type Tab = 'objects' | 'layers' | 'views';
const TAB_STORE = 'formsculptor.dock';

const count = (n: number): string => (n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(n >= 1e5 ? 0 : 1)}k` : String(n));

function toggle(icon: string, label: string): HTMLButtonElement {
  const b = iconOnly(icon, label, 'tb-dock-toggle');
  b.setAttribute('aria-pressed', 'false');
  return b;
}

/** A layer's strength as a bar dragged left and right (0 to 200 percent); sent to the engine when let go. */
class StrengthBar {
  readonly node = el('div', 'tb-dock-strength');
  private readonly fill = el('span', 'tb-dock-strength-fill');
  private readonly text = el('span', 'tb-dock-strength-num');
  private value = 1;
  private dragging = false;

  constructor(label: string, onSet: (v: number) => void) {
    this.node.tabIndex = 0;
    this.node.setAttribute('role', 'slider');
    this.node.setAttribute('aria-label', label);
    this.node.setAttribute('aria-valuemin', '0');
    this.node.setAttribute('aria-valuemax', '200');
    this.node.append(this.fill, this.text);
    const at = (e: PointerEvent): number => {
      const r = this.node.getBoundingClientRect();
      return Math.round(Math.max(0, Math.min(1, (e.clientX - r.left) / r.width)) * 40) / 20;
    };
    this.node.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      this.node.setPointerCapture(e.pointerId);
      this.dragging = true;
      this.show(at(e));
      const move = (m: PointerEvent): void => this.show(at(m));
      const up = (): void => {
        this.node.removeEventListener('pointermove', move);
        this.node.removeEventListener('pointerup', up);
        this.node.removeEventListener('pointercancel', up);
        this.dragging = false;
        onSet(this.value);
      };
      this.node.addEventListener('pointermove', move);
      this.node.addEventListener('pointerup', up);
      this.node.addEventListener('pointercancel', up);
    });
    this.node.addEventListener('click', (e) => e.stopPropagation());
    this.node.addEventListener('keydown', (e) => {
      const step = e.key === 'ArrowRight' || e.key === 'ArrowUp' ? 0.05 : e.key === 'ArrowLeft' || e.key === 'ArrowDown' ? -0.05 : 0;
      if (!step) return;
      e.preventDefault();
      this.show(Math.max(0, Math.min(2, Math.round((this.value + step) * 20) / 20)));
      onSet(this.value);
    });
  }

  /** Shows a value; the engine's value is shown only when the bar is not being dragged. */
  set(v: number): void {
    if (!this.dragging) this.show(v);
  }

  private show(v: number): void {
    this.value = v;
    this.fill.style.setProperty('--share', `${((Math.max(0, Math.min(2, v)) / 2) * 100).toFixed(1)}%`);
    setText(this.text, `${Math.round(v * 100)}%`);
    setAttr(this.node, 'aria-valuenow', String(Math.round(v * 100)));
  }
}

interface LayerRow {
  row: HTMLLIElement;
  name: HTMLElement;
  eye: HTMLButtonElement;
  bar: StrengthBar | null;
}

export class SceneDock {
  readonly element = el('section', 'tb-dock');
  private readonly tabs = new Map<Tab, HTMLButtonElement>();
  private readonly pages = new Map<Tab, HTMLElement>();
  private readonly objectList = el('ul', 'tb-dock-list');
  /** Object and group rows by id (the two share id numbers, so one id is never both). */
  private readonly objectRows = new Map<number, { kind: 'object' | 'group'; row: HTMLLIElement; name: HTMLElement; count: HTMLElement; eye: HTMLButtonElement; lock: HTMLButtonElement }>();
  /** Groups whose rows are folded shut. */
  private readonly collapsed = new Set<number>();
  private readonly layerOwner = el('p', 'tb-dock-owner');
  private readonly layerList = el('ul', 'tb-dock-list');
  private readonly layerRows: LayerRow[] = [];
  private readonly layerActions: HTMLButtonElement[] = [];
  private readonly emptyObjects = el('p', 'tb-dock-empty', '');
  private readonly emptyLayers = el('p', 'tb-dock-empty', '');
  private readonly viewList = el('ul', 'tb-dock-list');
  private readonly viewRows = new Map<string, HTMLLIElement>();
  private readonly emptyViews = el('p', 'tb-dock-empty', '');
  /** The saved view last recalled, marked in the list until another is chosen. */
  private lastView = '';
  private tab: Tab = 'objects';
  private scene: SceneSummary = { objects: [], active: null };
  /** The strength a hidden layer had, so showing it again gives it back (per object id and layer position). */
  private readonly hiddenStrength = new Map<string, number>();

  constructor(private readonly hooks: DockHooks) {
    this.element.setAttribute('aria-label', t('Objects and layers'));
    const head = el('div', 'tb-dock-head');
    head.setAttribute('role', 'tablist');
    for (const [tab, label] of [['objects', 'Objects'], ['layers', 'Layers'], ['views', 'Views']] as const) {
      const b = el('button', 'tb-dock-tab', t(label));
      b.type = 'button';
      b.setAttribute('role', 'tab');
      b.addEventListener('click', () => this.show(tab));
      this.tabs.set(tab, b);
      head.append(b);
    }
    const fold = iconOnly('collapse', t('Fold the panel'), 'tb-head-btn tb-dock-fold');
    fold.addEventListener('click', () => {
      const folded = this.element.classList.toggle('folded');
      setAttr(fold, 'aria-expanded', folded ? 'false' : 'true');
    });
    fold.setAttribute('aria-expanded', 'true');
    head.append(fold);

    // Objects: the list, then add, duplicate, rename and delete (registry commands, run by the root click handler).
    const objects = el('div', 'tb-dock-page');
    const objectActions = el('div', 'tb-dock-actions');
    for (const [icon, label, cmd] of [['plus', 'Add Sphere', 'add.sphere'], ['duplicate', 'Duplicate', 'object.duplicate'], ['rename', 'Rename', 'object.rename'], ['trash', 'Delete', 'object.delete']] as const) {
      const b = labelled(icon, label, 'tb-dock-act');
      b.dataset.cmd = cmd;
      objectActions.append(b);
    }
    // Group: a new empty group holding the active object (Nomad's group, a node to move things together).
    const group = labelled('group_node', 'Group', 'tb-dock-act');
    group.addEventListener('click', () => void this.hooks.run('add_group', this.scene.active ? { children: [this.scene.active] } : {}));
    objectActions.insertBefore(group, objectActions.lastElementChild);
    wireTreeDrag(this.objectList, (id, parent) => void this.hooks.run('set_parent', { ids: [id], parent }));
    setText(this.emptyObjects, t('The scene is empty: add a sphere to start sculpting'));
    objects.append(this.objectList, this.emptyObjects, objectActions);
    this.objectList.setAttribute('aria-label', t('Objects'));
    this.objectList.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const row = target.closest<HTMLLIElement>('li');
      const id = Number(row?.dataset.id);
      if (!row || !id) return;
      const act = target.closest<HTMLElement>('[data-act]')?.dataset.act;
      if (row.dataset.kind === 'group') {
        // A group row: its eye hides or shows everything under it, its cross deletes it (the children move up), and
        // the row itself folds or unfolds.
        if (act === 'visible') {
          const shown = objectsUnder(treeOrder(this.scene), id).some((o) => o.visible);
          void this.hooks.run('set_visibility', { action: shown ? 'hide' : 'show', ids: [id] });
        } else if (act === 'delete') void this.hooks.run('delete_group', { id });
        else {
          if (!this.collapsed.delete(id)) this.collapsed.add(id);
          this.patchObjects();
        }
        return;
      }
      if (act === 'visible') {
        const o = this.scene.objects.find((x) => x.id === id);
        void this.hooks.run('set_visibility', { action: o?.visible ? 'hide' : 'show', ids: [id] });
      } else if (act === 'lock') {
        this.hooks.toggleLock(id);
        this.update(this.scene);
      } else if (id !== this.scene.active) {
        void this.hooks.run('select_object', { id });
      }
    });

    // Layers: whose layers they are, the base mesh and each layer, then add, apply and delete.
    const layers = el('div', 'tb-dock-page');
    const layerActions = el('div', 'tb-dock-actions');
    for (const [icon, label, action] of [['plus', 'Add layer', 'add'], ['apply', 'Apply', 'apply'], ['trash', 'Delete', 'delete']] as const) {
      const b = labelled(icon, label, 'tb-dock-act');
      b.dataset.layerAct = action;
      b.addEventListener('click', () => this.layerAction(action));
      layerActions.append(b);
      this.layerActions.push(b);
    }
    setText(this.emptyLayers, t('Layers record strokes apart from the base mesh, so their strength can fade or exaggerate them later'));
    this.layerList.setAttribute('aria-label', t('Layers'));
    layers.append(this.layerOwner, this.layerList, this.emptyLayers, layerActions);

    // Views: the scene's saved camera views; a row moves the view there, its cross deletes it; Save view keeps the
    // current one under a new name.
    const views = el('div', 'tb-dock-page');
    const viewActions = el('div', 'tb-dock-actions');
    const save = labelled('plus', t('Save view'), 'tb-dock-act');
    save.addEventListener('click', () => this.hooks.saveView());
    viewActions.append(save);
    setText(this.emptyViews, t('No saved views yet: turn the view where you want it, then Save view'));
    this.viewList.setAttribute('aria-label', t('Views'));
    this.viewList.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      const row = target.closest<HTMLLIElement>('li');
      const name = row?.dataset.view;
      if (!row || name === undefined) return;
      if (target.closest('[data-act="delete"]')) this.hooks.deleteView(name);
      else this.recall(name);
    });
    views.append(this.viewList, this.emptyViews, viewActions);

    this.pages.set('objects', objects);
    this.pages.set('layers', layers);
    this.pages.set('views', views);
    const body = el('div', 'tb-dock-body');
    body.append(objects, layers, views);
    this.element.append(head, body);
    try {
      const kept = localStorage.getItem(TAB_STORE);
      if (kept === 'layers' || kept === 'views') this.tab = kept;
    } catch {
      // storage blocked; the dock opens on Objects
    }
    this.show(this.tab, false);
  }

  private show(tab: Tab, keep = true): void {
    this.tab = tab;
    for (const [k, b] of this.tabs) setAttr(b, 'aria-selected', k === tab ? 'true' : 'false');
    for (const [k, p] of this.pages) if (p.hidden !== (k !== tab)) p.hidden = k !== tab;
    this.element.classList.remove('folded');
    if (!keep) return;
    try {
      localStorage.setItem(TAB_STORE, tab);
    } catch {
      // storage blocked; the tab lasts for this session
    }
  }

  private active(): ObjectSummary | undefined {
    return this.scene.objects.find((o) => o.id === this.scene.active);
  }

  /** Patches both lists from a scene summary (after every refresh). */
  update(scene: SceneSummary): void {
    this.scene = scene;
    this.patchObjects();
    this.patchLayers();
    this.patchViews();
  }

  /** Moves to a saved view and marks it as the one in use. */
  recall(name: string): void {
    this.lastView = name;
    this.hooks.recallView(name);
    this.patchViews();
  }

  private patchViews(): void {
    const names = this.scene.views ?? [];
    const keep = new Set(names);
    for (const [name, row] of this.viewRows) {
      if (!keep.has(name)) {
        row.remove();
        this.viewRows.delete(name);
      }
    }
    names.forEach((name, i) => {
      let row = this.viewRows.get(name);
      if (!row) {
        row = el('li', 'tb-dock-row');
        row.dataset.view = name;
        row.tabIndex = 0;
        const icon = el('span', 'tb-dock-glyph');
        icon.append(glyph('camera'));
        const del = iconOnly('close', t('Delete view'), 'tb-dock-toggle');
        del.dataset.act = 'delete';
        del.setAttribute('aria-pressed', 'true');
        row.append(icon, el('span', 'tb-dock-name', name), del);
        const r = row;
        row.addEventListener('keydown', (e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            r.click();
          }
        });
        this.viewRows.set(name, row);
      }
      if (this.viewList.children[i] !== row) this.viewList.insertBefore(row, this.viewList.children[i] ?? null);
      setAttr(row, 'aria-current', name === this.lastView ? 'true' : 'false');
    });
    const empty = names.length === 0;
    if (this.emptyViews.hidden !== !empty) this.emptyViews.hidden = !empty;
  }

  private patchObjects(): void {
    const nodes = treeOrder(this.scene);
    const keep = new Map(nodes.map((n) => [n.id, n.kind]));
    for (const [id, r] of this.objectRows) {
      if (keep.get(id) !== r.kind) {
        r.row.remove();
        this.objectRows.delete(id);
      }
    }
    for (const id of this.collapsed) if (keep.get(id) !== 'group') this.collapsed.delete(id);
    nodes.forEach((n, i) => {
      let r = this.objectRows.get(n.id);
      if (!r) {
        const group = n.kind === 'group';
        const row = el('li', group ? 'tb-dock-row tb-dock-group' : 'tb-dock-row');
        row.dataset.id = String(n.id);
        row.dataset.kind = n.kind;
        row.tabIndex = 0;
        const name = el('span', 'tb-dock-name', '');
        const c = el('span', 'tb-dock-count', '');
        const eye = toggle('eye', 'Visible');
        eye.dataset.act = 'visible';
        // An object's last toggle locks it; a group's deletes the group.
        const lock = group ? toggle('close', 'Delete group') : toggle('lock', 'Locked');
        lock.dataset.act = group ? 'delete' : 'lock';
        if (group) {
          const mark = glyph('group_node');
          mark.classList.add('tb-dock-glyph');
          row.append(mark);
        }
        row.append(name, c, eye, lock);
        row.addEventListener('keydown', (e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            row.click();
          }
        });
        r = { kind: n.kind, row, name, count: c, eye, lock };
        this.objectRows.set(n.id, r);
      }
      // Keep the tree's order without moving rows that are already in place.
      if (this.objectList.children[i] !== r.row) this.objectList.insertBefore(r.row, this.objectList.children[i] ?? null);
      const depth = String(n.depth);
      if (r.row.style.getPropertyValue('--depth') !== depth) r.row.style.setProperty('--depth', depth);
      const hide = folded(nodes, n.id, this.collapsed);
      if (r.row.hidden !== hide) r.row.hidden = hide;
      const parent = n.parent === null ? '' : String(n.parent);
      if (r.row.dataset.parent !== parent) r.row.dataset.parent = parent;
      if (n.kind === 'group' && n.group) {
        const g = n.group;
        const shown = objectsUnder(nodes, g.id).some((o) => o.visible);
        setText(r.name, g.name);
        setText(r.count, String(g.objects));
        setAttr(r.row, 'aria-expanded', this.collapsed.has(g.id) ? 'false' : 'true');
        setAttr(r.row, 'data-hidden', shown || g.objects === 0 ? 'false' : 'true');
        setAttr(r.eye, 'aria-pressed', shown ? 'true' : 'false');
        return;
      }
      const o = n.object as ObjectSummary;
      setText(r.name, o.folder ? `${o.folder} / ${o.name}` : o.name);
      setText(r.count, count(o.vertices));
      setAttr(r.row, 'aria-current', o.id === this.scene.active ? 'true' : 'false');
      setAttr(r.row, 'data-hidden', o.visible ? 'false' : 'true');
      setAttr(r.eye, 'aria-pressed', o.visible ? 'true' : 'false');
      setAttr(r.lock, 'aria-pressed', this.hooks.isLocked(o.id) ? 'true' : 'false');
    });
    const empty = nodes.length === 0;
    if (this.emptyObjects.hidden !== !empty) this.emptyObjects.hidden = !empty;
  }

  private patchLayers(): void {
    const o = this.active();
    const layers = o?.layers ?? [];
    setText(this.layerOwner, o ? `${t('Layers of')} ${o.name}` : t('Choose an object to see its layers'));
    // Rows: the base mesh, then each layer in order.
    const want = o ? layers.length + 1 : 0;
    while (this.layerRows.length > want) this.layerRows.pop()?.row.remove();
    while (this.layerRows.length < want) {
      const index = this.layerRows.length - 1;
      const row = el('li', 'tb-dock-row tb-dock-layer');
      row.tabIndex = 0;
      const name = el('span', 'tb-dock-name', '');
      const eye = toggle(index < 0 ? 'base' : 'eye', index < 0 ? 'Base mesh' : 'Visible');
      let bar: StrengthBar | null = null;
      if (index < 0) {
        eye.disabled = true;
        row.append(eye, name);
      } else {
        bar = new StrengthBar(t('Strength'), (v) => this.setStrength(index, v));
        eye.addEventListener('click', (e) => {
          e.stopPropagation();
          this.toggleLayer(index);
        });
        row.append(eye, name, bar.node);
      }
      const pick = (): void => {
        const owner = this.active();
        if (owner && owner.active_layer !== (index < 0 ? null : index)) void this.hooks.run('layer_select', { id: owner.id, index: index < 0 ? null : index });
      };
      row.addEventListener('click', pick);
      row.addEventListener('keydown', (e) => {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          pick();
        }
      });
      this.layerList.append(row);
      this.layerRows.push({ row, name, eye, bar });
    }
    this.layerRows.forEach((r, i) => {
      const index = i - 1;
      const layer = index >= 0 ? layers[index] : null;
      setText(r.name, layer ? layer.name : t('Base mesh'));
      const current = (o?.active_layer ?? null) === (index < 0 ? null : index);
      setAttr(r.row, 'aria-current', current ? 'true' : 'false');
      if (layer && r.bar) {
        r.bar.set(layer.strength);
        setAttr(r.eye, 'aria-pressed', layer.strength !== 0 ? 'true' : 'false');
        setAttr(r.row, 'data-hidden', layer.strength === 0 ? 'true' : 'false');
      }
    });
    const none = !o || layers.length === 0;
    if (this.emptyLayers.hidden !== !none) this.emptyLayers.hidden = !none;
    const hasLayer = o?.active_layer !== null && o?.active_layer !== undefined;
    for (const b of this.layerActions) b.disabled = !o || (b.dataset.layerAct !== 'add' && !hasLayer);
  }

  private setStrength(index: number, v: number): void {
    const o = this.active();
    if (o) void this.hooks.run('layer_set_strength', { id: o.id, index, strength: v });
  }

  /** Hides a layer (strength 0) or shows it again at the strength it had. */
  private toggleLayer(index: number): void {
    const o = this.active();
    const layer = o?.layers[index];
    if (!o || !layer) return;
    const key = `${o.id}:${index}`;
    if (layer.strength !== 0) {
      this.hiddenStrength.set(key, layer.strength);
      this.setStrength(index, 0);
    } else {
      this.setStrength(index, this.hiddenStrength.get(key) ?? 1);
      this.hiddenStrength.delete(key);
    }
  }

  private layerAction(action: 'add' | 'apply' | 'delete'): void {
    const o = this.active();
    if (!o) return;
    if (action === 'add') {
      void this.hooks.run('layer_add', { id: o.id, name: `${t('Layer')} ${o.layers.length + 1}` });
      return;
    }
    if (o.active_layer === null) return;
    void this.hooks.run(action === 'apply' ? 'layer_apply' : 'layer_delete', { id: o.id, index: o.active_layer });
  }
}
