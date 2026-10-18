/**
 * The scene hierarchy as both object lists show it: rows in tree order (each node's groups, then its objects, then
 * theirs below them), indented by depth, and Nomad Sculpt's way of parenting: drag a row onto another row to hang it
 * there, or onto the list's empty space to bring it to the top. A mouse drag starts once the pointer has moved a
 * little; on a touch screen it starts after a long press, so a short swipe still scrolls the list.
 */
import type { GroupSummary, ObjectSummary, SceneSummary } from '../engine/service';

export interface TreeNode {
  kind: 'object' | 'group';
  id: number;
  depth: number;
  object?: ObjectSummary;
  group?: GroupSummary;
  /** The node's own parent while it exists, else null (a deleted parent counts as none). */
  parent: number | null;
}

/** Every object and group, depth first. */
export function treeOrder(scene: SceneSummary): TreeNode[] {
  const groups = scene.groups ?? [];
  const known = new Set<number>([...scene.objects.map((o) => o.id), ...groups.map((g) => g.id)]);
  const up = (p: number | null | undefined): number | null => (p != null && known.has(p) ? p : null);
  const under = new Map<number | null, TreeNode[]>();
  const add = (n: TreeNode): void => {
    const list = under.get(n.parent) ?? [];
    list.push(n);
    under.set(n.parent, list);
  };
  for (const g of groups) add({ kind: 'group', id: g.id, depth: 0, group: g, parent: up(g.parent) });
  for (const o of scene.objects) add({ kind: 'object', id: o.id, depth: 0, object: o, parent: up(o.parent) });
  const out: TreeNode[] = [];
  const seen = new Set<number>();
  const walk = (parent: number | null, depth: number): void => {
    for (const n of under.get(parent) ?? []) {
      if (seen.has(n.id)) continue;
      seen.add(n.id);
      n.depth = depth;
      out.push(n);
      walk(n.id, depth + 1);
    }
  };
  walk(null, 0);
  // Anything a cycle kept out of the walk still shows, at the top.
  for (const list of under.values()) for (const n of list) if (!seen.has(n.id)) out.push({ ...n, depth: 0 });
  return out;
}

/** Whether `id` sits below a collapsed group (so its row is hidden). */
export function folded(nodes: TreeNode[], id: number, collapsed: ReadonlySet<number>): boolean {
  const parentOf = new Map(nodes.map((n) => [n.id, n.parent]));
  let at = parentOf.get(id) ?? null;
  for (let i = 0; at !== null && i < nodes.length; i++) {
    if (collapsed.has(at)) return true;
    at = parentOf.get(at) ?? null;
  }
  return false;
}

/** The objects anywhere below `id`. */
export function objectsUnder(nodes: TreeNode[], id: number): ObjectSummary[] {
  const parentOf = new Map(nodes.map((n) => [n.id, n.parent]));
  const below = (n: TreeNode): boolean => {
    let at = n.parent;
    for (let i = 0; at !== null && i < nodes.length; i++) {
      if (at === id) return true;
      at = parentOf.get(at) ?? null;
    }
    return false;
  };
  return nodes.filter((n) => n.kind === 'object' && n.object && below(n)).map((n) => n.object as ObjectSummary);
}

const HOLD_MS = 420;
const SLOP = 6;

/**
 * Lets rows (li[data-id]) of `list` be dragged onto each other. `drop` gets the dragged id and the new parent (null:
 * the top). Presses on buttons inside a row are left alone. The click a drag ends with is swallowed.
 */
export function wireTreeDrag(list: HTMLElement, drop: (id: number, parent: number | null) => void): void {
  let press: { id: number; row: HTMLElement; x: number; y: number; pointer: number; touch: boolean; timer: number } | null = null;
  let dragging = false;
  let over: HTMLElement | null = null;
  // Over a row's middle half the drop hangs the dragged row inside it; over its top or bottom quarter, beside it
  // (under the same parent), which is how a row comes back out to the top in a list with no empty space.
  let into = true;
  const mark = (row: HTMLElement | null, inside = true): void => {
    if (over === row && into === inside) return;
    over?.removeAttribute('data-drop');
    over = row;
    into = inside;
    over?.setAttribute('data-drop', inside ? 'into' : 'beside');
  };
  const start = (): void => {
    if (!press) return;
    dragging = true;
    press.row.setAttribute('data-dragging', 'true');
    list.setAttribute('data-dragging', 'true');
  };
  const end = (): void => {
    if (press) window.clearTimeout(press.timer);
    press?.row.removeAttribute('data-dragging');
    list.removeAttribute('data-dragging');
    mark(null);
    press = null;
    dragging = false;
  };
  list.addEventListener('pointerdown', (e) => {
    const row = (e.target as HTMLElement).closest<HTMLElement>('li[data-id]');
    if (!row || !list.contains(row) || (e.target as HTMLElement).closest('button') || e.button !== 0) return;
    const touch = e.pointerType === 'touch';
    press = { id: Number(row.dataset.id), row, x: e.clientX, y: e.clientY, pointer: e.pointerId, touch, timer: 0 };
    if (touch) press.timer = window.setTimeout(start, HOLD_MS);
  });
  list.addEventListener('pointermove', (e) => {
    if (!press || e.pointerId !== press.pointer) return;
    const moved = Math.hypot(e.clientX - press.x, e.clientY - press.y) > SLOP;
    if (!dragging) {
      // A touch that moves before the hold is a scroll; a mouse that moves is a drag.
      if (moved && press.touch) end();
      else if (moved) start();
      return;
    }
    const hit = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
    const row = hit?.closest<HTMLElement>('li[data-id]') ?? null;
    if (!row || !list.contains(row) || row === press.row) {
      mark(null);
      return;
    }
    const box = row.getBoundingClientRect();
    const y = (e.clientY - box.top) / Math.max(1, box.height);
    mark(row, y > 0.25 && y < 0.75);
  });
  // While dragging on a touch screen the list must not scroll.
  list.addEventListener('touchmove', (e) => { if (dragging) e.preventDefault(); }, { passive: false });
  list.addEventListener('pointerup', (e) => {
    if (!press || e.pointerId !== press.pointer) return;
    if (dragging) {
      const id = press.id;
      // Into a row: that row; beside a row: that row's parent (rows carry it in data-parent, empty at the top); on the
      // list's empty space: the top; outside the list: nothing changes.
      const inside = list.contains(document.elementFromPoint(e.clientX, e.clientY));
      const parent: number | null | undefined = over ? (into ? Number(over.dataset.id) : over.dataset.parent ? Number(over.dataset.parent) : null) : inside ? null : undefined;
      // Swallow the click this release may make, so the row is not also selected; a release over another element
      // makes none, so the swallower goes after this turn rather than waiting for the next real click.
      const swallow = (c: Event): void => c.stopPropagation();
      list.addEventListener('click', swallow, { capture: true });
      window.setTimeout(() => list.removeEventListener('click', swallow, { capture: true }), 0);
      end();
      if (parent !== undefined) drop(id, parent);
      return;
    }
    end();
  });
  list.addEventListener('pointercancel', end);
}
