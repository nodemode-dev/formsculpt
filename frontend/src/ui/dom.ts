export function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls?: string, text?: string): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function saveBlob(blob: Blob, name: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  a.hidden = true;
  document.body.append(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 10_000);
}

export function setText(node: Node, value: string): void {
  if (node.textContent !== value) node.textContent = value;
}

export function setAttr(node: Element, name: string, value: string): void {
  if (node.getAttribute(name) !== value) node.setAttribute(name, value);
}

export const isMac = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

export function fromHtml<T extends HTMLElement = HTMLElement>(markup: string): T {
  const tpl = document.createElement('template');
  tpl.innerHTML = markup.trim();
  return tpl.content.firstElementChild as T;
}

export function q<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
  const node = root.querySelector(selector);
  if (!node) throw new Error(`missing element: ${selector}`);
  return node as T;
}

export function setClass(node: Element | null, cls: string, on: boolean): void {
  if (node && node.classList.contains(cls) !== on) node.classList.toggle(cls, on);
}

export function setHidden(node: HTMLElement, hidden: boolean): void {
  if (node.hidden !== hidden) node.hidden = hidden;
}

export function setDisabled(node: HTMLButtonElement | HTMLInputElement | HTMLSelectElement, disabled: boolean): void {
  if (node.disabled !== disabled) node.disabled = disabled;
}

export function setTitle(node: HTMLElement, title: string): void {
  if (node.title !== title) node.title = title;
}

export function setHtml(node: Element, markup: string, key: string): void {
  const n = node as HTMLElement;
  if (n.dataset.htmlKey === key) return;
  n.dataset.htmlKey = key;
  const tpl = document.createElement('template');
  tpl.innerHTML = markup;
  n.replaceChildren(tpl.content);
}

export interface PatchListOptions<T> {
  key: (item: T) => string;
  create: (item: T) => HTMLElement;
  update: (node: HTMLElement, item: T) => void;
}

export function patchList<T>(container: HTMLElement, items: T[], opts: PatchListOptions<T>): void {
  const byKey = new Map<string, HTMLElement>();
  for (const child of Array.from(container.children)) {
    const k = (child as HTMLElement).dataset.key;
    if (k !== undefined) byKey.set(k, child as HTMLElement);
  }
  let cursor: Element | null = container.firstElementChild;
  const seen = new Set<string>();
  for (const item of items) {
    const k = opts.key(item);
    seen.add(k);
    let node = byKey.get(k);
    if (node) opts.update(node, item);
    else {
      node = opts.create(item);
      node.dataset.key = k;
    }
    if (node !== cursor) container.insertBefore(node, cursor);
    else cursor = cursor.nextElementSibling;
  }
  for (const [k, node] of byKey) if (!seen.has(k)) node.remove();
}
