import { LANG_NAMES, LANG_PARENT, isLang, type Lang } from './i18n/langs';

export type { Lang } from './i18n/langs';

type Table = Record<string, string>;
interface Pattern { re: RegExp; names: string[]; out: string }

const STORE = 'formsculptor.language';
const FILES = import.meta.glob<Table>('../i18n/*.json', { import: 'default' });
const listeners = new Set<() => void>();
const tables = new Map<Lang, Table>([['en', {}]]);
const patterns = new Map<Lang, Pattern[]>();
const cache = new Map<string, string>();

function savedLanguage(): Lang {
  try {
    const v = localStorage.getItem(STORE);
    return isLang(v) ? v : 'en';
  } catch {
    return 'en';
  }
}

let current: Lang = 'en';
let wanted: Lang = savedLanguage();

function compile(table: Table): Pattern[] {
  const out: Pattern[] = [];
  for (const [en, tr] of Object.entries(table)) {
    if (!/\{\w+\}/.test(en)) continue;
    const names: string[] = [];
    const source = en.replace(/[.*+?^$()|[\]\\]/g, '\\$&').replace(/\{(\w+)\}/g, (_, n: string) => { names.push(n); return '(.+?)'; });
    out.push({ re: new RegExp(`^${source}$`), names, out: tr });
  }
  return out.sort((a, b) => b.re.source.length - a.re.source.length);
}

async function load(l: Lang): Promise<void> {
  for (const code of [LANG_PARENT[l], l]) {
    if (!code || code === 'en' || tables.has(code)) continue;
    const loader = FILES[`../i18n/${code}.json`];
    const table = loader ? await loader() : {};
    tables.set(code, table);
    patterns.set(code, compile(table));
  }
}

function lookup(text: string): string | undefined {
  const parent = LANG_PARENT[current];
  for (const code of [current, parent]) {
    if (!code) continue;
    const hit = tables.get(code)?.[text];
    if (hit !== undefined) return hit;
  }
  for (const code of [current, parent]) {
    if (!code) continue;
    for (const p of patterns.get(code) ?? []) {
      const m = p.re.exec(text);
      if (!m) continue;
      return p.out.replace(/\{(\w+)\}/g, (all, n: string) => {
        const i = p.names.indexOf(n);
        return i >= 0 ? (m[i + 1] ?? '') : all;
      });
    }
  }
  return undefined;
}

export function t(english: string): string {
  if (current === 'en' || !english) return english;
  const key = `${current}\u0000${english}`;
  const hit = cache.get(key);
  if (hit !== undefined) return hit;
  const tail = english.endsWith('...') ? '...' : english.endsWith(':') && !english.endsWith('::') ? ':' : '';
  const base = tail ? english.slice(0, -tail.length) : english;
  let out = lookup(english);
  if (out === undefined && tail) {
    const b = lookup(base);
    if (b !== undefined) out = b + tail;
  }
  const result = out ?? english;
  if (cache.size > 20000) cache.clear();
  cache.set(key, result);
  return result;
}

export function language(): string {
  return current;
}

export const languages = (): ReadonlyArray<readonly [Lang, string]> => LANG_NAMES;

export async function setLanguage(l: Lang): Promise<void> {
  wanted = l;
  try {
    localStorage.setItem(STORE, l);
  } catch {
    // storage blocked; the choice lasts for this session
  }
  try {
    await load(l);
  } catch {
    return;
  }
  if (wanted !== l || l === current) return;
  current = l;
  document.documentElement.lang = l;
  for (const fn of listeners) fn();
  translatePage();
}

export async function startLanguage(): Promise<void> {
  const l = wanted;
  if (l === 'en') return;
  try {
    await load(l);
  } catch {
    return;
  }
  if (wanted !== l) return;
  current = l;
  document.documentElement.lang = l;
}

export function onLanguage(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

const ATTRS = ['title', 'aria-label', 'placeholder'] as const;
const SKIP = 'textarea, input, script, style, canvas, svg, [contenteditable], [data-no-i18n], .xterm, .fl-chat-msg, .fl-md, .fl-chat-tool, .fl-dock-tab-title, .objects .name, .history';
const SKIP_ATTR = SKIP.replace('textarea, input, ', '');
const textEn = new WeakMap<Text, { en: string; shown: string }>();
const attrEn = new WeakMap<Element, Map<string, { en: string; shown: string }>>();
let observer: MutationObserver | null = null;

const known = (text: string): boolean => current !== 'en' && t(text) !== text;
const skipped = (n: Node | null): boolean => {
  const e = n instanceof Element ? n : n?.parentElement ?? null;
  return !e || !!e.closest(SKIP);
};

function translateText(node: Text): void {
  const raw = node.data;
  const rec = textEn.get(node);
  if (rec && raw === rec.shown) {
    const next = t(rec.en);
    if (next !== raw) {
      rec.shown = next;
      node.data = next;
    }
    if (current === 'en' || next === rec.en) textEn.delete(node);
    return;
  }
  const trimmed = raw.trim();
  if (!trimmed || trimmed.length > 300 || !known(trimmed) || skipped(node)) {
    if (rec) textEn.delete(node);
    return;
  }
  const next = raw.replace(trimmed, t(trimmed));
  textEn.set(node, { en: raw, shown: next });
  node.data = next;
}

function translateAttrs(el: Element): void {
  for (const a of ATTRS) {
    const v = el.getAttribute(a);
    if (v === null) continue;
    let recs = attrEn.get(el);
    const rec = recs?.get(a);
    if (rec && v === rec.shown) {
      const next = t(rec.en);
      if (next !== v) {
        rec.shown = next;
        el.setAttribute(a, next);
      }
      continue;
    }
    if (!known(v) || el.closest(SKIP_ATTR)) {
      recs?.delete(a);
      continue;
    }
    if (!recs) {
      recs = new Map();
      attrEn.set(el, recs);
    }
    const next = t(v);
    recs.set(a, { en: v, shown: next });
    el.setAttribute(a, next);
  }
}

function walk(root: Node): void {
  if (root instanceof Text) {
    translateText(root);
    return;
  }
  if (!(root instanceof Element)) return;
  if (root.closest(SKIP)) {
    if (root.matches('input, textarea')) translateAttrs(root);
    return;
  }
  translateAttrs(root);
  const it = document.createTreeWalker(root, NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => {
      if (!(n instanceof Element) || !n.matches(SKIP)) return NodeFilter.FILTER_ACCEPT;
      if (n.matches('input, textarea')) translateAttrs(n);
      return NodeFilter.FILTER_REJECT;
    },
  });
  for (let n = it.nextNode(); n; n = it.nextNode()) {
    if (n instanceof Text) translateText(n);
    else if (n instanceof Element) translateAttrs(n);
  }
}

const OBSERVE: MutationObserverInit = { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: [...ATTRS] };

function translatePage(): void {
  if (typeof document === 'undefined') return;
  observer?.disconnect();
  walk(document.body);
  observer?.observe(document.body, OBSERVE);
}

export function startPageTranslator(): void {
  if (observer || typeof MutationObserver === 'undefined') return;
  observer = new MutationObserver((records) => {
    if (current === 'en') return;
    observer?.disconnect();
    for (const r of records) {
      if (r.type === 'childList') r.addedNodes.forEach((n) => walk(n));
      else if (r.type === 'characterData' && r.target instanceof Text) translateText(r.target);
      else if (r.type === 'attributes' && r.target instanceof Element) translateAttrs(r.target);
    }
    observer?.observe(document.body, OBSERVE);
  });
  translatePage();
}
