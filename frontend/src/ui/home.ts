import '../styles/home.css';
import { library, type SculptSummary, type TemplateSummary, type TrashedSculpt } from '../engine/library';
import { browserEngine, dispatch } from '../engine/service';
import { brandMark } from './brand';
import { commands, type Command } from './commands';
import { DEMOS, type DemoId } from './demos';
import { sceneSaved, unsavedWork } from './desktop-shell';
import { anyDialogOpen, Dialog, overlayRoot } from './dialog';
import { fromHtml, patchList, setClass, setHidden } from './dom';
import { onLanguage, t } from './i18n';
import { icon } from './icons';
import { loadPrefs, updatePrefs } from './prefs';
import { dismissToast, pushToast, toast, toastRoot } from './toast';

export type HomePage = 'start' | 'recent' | 'templates' | 'trash';

export interface HomeHooks {
  openSculpt(id: string): void;
  closeSculpt(id: string): void;
  createSculpt(template: string, title?: string): Promise<void>;
  blankScene(): void;
  openFiles(files: File[]): void;
  preferences(): void;
  agents(): void;
  demo(id: DemoId): void;
  back(): void;
  autoShow(): boolean;
  afterClose(): boolean;
  onVisible?(open: boolean): void;
}

export interface HomeHost {
  objects(): number;
  sceneName(): string;
  replaced(): void;
  refresh(): Promise<void>;
  frame(): void;
  note(text: string): void;
  importFile(file: File): Promise<string | null>;
  picture(): HTMLCanvasElement;
  download(): Promise<boolean>;
}

interface OpenSculpt { id: string | null; title: string; blank: boolean }

const ACCEPT = '.fsc,.obj,.stl,.ply,.glb,.gltf,.3mf,.fbx,.usda,.usd';
const FORMATS = ['FSC', 'OBJ', 'STL', 'PLY', 'GLB', 'GLTF', '3MF', 'FBX', 'USD', 'USDA'];
const CATEGORIES: Array<[string, string]> = [['all', 'All'], ['bases', 'Sculpting bases'], ['primitives', 'Primitives']];
const ALL = CATEGORIES[0][1];
const UNTITLED = 'Untitled sculpt';
const NAV_KEY = 'formsculpt.homeNav';
const NAV_HIDDEN_KEY = 'formsculpt.homeNavHidden';
const SESSION_KEY = 'formsculpt.home.current';

const esc = (s: string): string => s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c] ?? c);
const write = (node: Element | null, value: string): void => {
  if (node && node.textContent !== value) node.textContent = value;
};
const why = (e: unknown): string => t('That did not work: {error}').replace('{error}', e instanceof Error ? e.message : String(e));
const count = (n: number, one: string, many: string): string => (n === 1 ? one : many.replace('{n}', String(n)));

function ago(iso: string): string {
  const s = (Date.now() - Date.parse(iso)) / 1000;
  if (!Number.isFinite(s)) return '';
  if (s < 60) return t('just now');
  if (s < 3600) return t('{n} min ago').replace('{n}', String(Math.round(s / 60)));
  if (s < 86400) return t('{n} h ago').replace('{n}', String(Math.round(s / 3600)));
  return new Date(iso).toLocaleDateString([], { month: 'short', day: 'numeric' });
}

function size(bytes: number): string {
  return bytes >= 1048576 ? `${(bytes / 1048576).toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

function showPicture(img: HTMLImageElement, url: string, fail: () => void): void {
  if (img.dataset.want === url) return;
  img.dataset.want = url;
  const probe = new Image();
  probe.decoding = 'async';
  probe.onload = () => {
    if (img.dataset.want !== url) return;
    const show = (): void => {
      if (img.dataset.want === url) setHidden(img, false);
    };
    img.src = url;
    if (typeof img.decode === 'function') img.decode().then(show, show);
    else show();
  };
  probe.onerror = () => {
    if (img.dataset.want === url) fail();
  };
  probe.src = url;
}

const previewHtml = (glyph: string): string => `<span class="fs-card-preview"><img alt="" decoding="async" draggable="false" hidden><span class="fs-card-glyph" hidden>${icon(glyph, 28)}</span></span>`;

function setPreview(card: HTMLElement, url: string): void {
  const box = card.querySelector('.fs-card-preview') as HTMLElement;
  const img = box.querySelector('img') as HTMLImageElement;
  const glyph = box.querySelector('.fs-card-glyph') as HTMLElement;
  const blank = (): void => {
    img.dataset.want = '';
    setHidden(img, true);
    setHidden(glyph, false);
    setClass(box, 'is-blank', true);
  };
  if (!url) {
    blank();
    return;
  }
  if (img.dataset.want === url) return;
  setClass(box, 'is-blank', false);
  setHidden(glyph, true);
  showPicture(img, url, blank);
}

export class Home {
  readonly root: HTMLElement;
  private view: HomePage = 'start';
  private templates: TemplateSummary[] = [];
  private categories: string[] = CATEGORIES.map((c) => c[1]);
  private category = ALL;
  private query = '';
  private sculpts: SculptSummary[] = [];
  private readonly tplGrid: HTMLElement;
  private readonly tplGridFull: HTMLElement;
  private readonly recentGrid: HTMLElement;
  private readonly recentGridFull: HTMLElement;
  private pinned: string[] = [];
  private hiddenRecent: string[] = [];
  private readonly preview: TemplateDialog;
  private trashed: TrashedSculpt[] = [];
  private readonly selected = new Set<string>();
  private anchor = '';
  private readonly trashList: HTMLElement;
  private readonly confirmBar: HTMLElement;
  private onConfirm: (() => void) | null = null;
  private readonly menu: HTMLElement;
  private forced = false;
  private hadDocument = false;
  private documentOpen = false;
  private loads = 0;

  constructor(private readonly hooks: HomeHooks) {
    const n = (v: HomePage, label: string, glyph: string, badge = ''): string => `<button class="fs-home-nav-item" type="button" data-v="${v}">${icon(glyph, 16)}<span>${label}</span>${badge ? `<span class="fs-badge is-count ${badge}" data-no-i18n>0</span>` : ''}</button>`;
    const filters = `<div class="fs-home-filters fs-cats"></div><label class="fs-search">${icon('search', 14)}<input type="search" placeholder="Search templates" class="fs-tpl-search" spellcheck="false" autocomplete="off"></label>`;
    const demos = DEMOS.length
      ? `<section class="fs-home-section" data-section="demos"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Demos</h2><span class="fs-card-meta">Opens as a new sculpt in your library.</span></div><div class="fs-card-grid fs-demo-grid">${DEMOS.map((d) => `<button class="fs-card" type="button" data-demo="${esc(d.id)}"><span class="fs-card-preview" aria-hidden="true"><img alt="" decoding="async" draggable="false" src="${esc(d.picture)}"></span><span class="fs-card-body"><span class="fs-card-name" translate="no" data-no-i18n>${esc(d.title)}</span><span class="fs-card-meta" title="${esc(d.line)}">${esc(d.line)}</span></span></button>`).join('')}</div></section>`
      : '';
    this.root = fromHtml(`<section class="fs-home" aria-label="Home" tabindex="-1" hidden>
<nav class="fs-home-nav">
<div class="fs-home-brand">${brandMark(26)}formsculpt <span translate="no" data-no-i18n>by nodemode.ai</span></div>
${n('start', 'Start', 'home')}${n('recent', 'Recent sculpts', 'cube', 'fs-sculpt-count')}${n('templates', 'Templates', 'template', 'fs-tpl-count')}${n('trash', 'Trash', 'trash', 'fs-trash-count')}
<div class="fs-home-open" hidden><div class="fs-home-nav-sep"></div><div class="fs-home-nav-title">Open sculpt</div><div class="fs-home-open-list"></div></div>
<div class="fs-home-nav-sep"></div>
<div class="fs-home-nav-title">Agents</div>
<button class="fs-home-nav-item" type="button" data-a="agents">${icon('agent', 16)}<span translate="no">Claude Code, Codex, Cursor</span></button>
<div class="fs-home-nav-foot">
<button class="fs-home-nav-item fs-home-back" type="button" data-a="back" hidden>${icon('chevronLeft', 16)}<span>Back to sculpt</span></button>
<button class="fs-home-nav-item" type="button" data-a="local" title="Your sculpts stay in your Formsculpt folder on this computer.">${icon('cloudOff', 16)}<span>Local only</span></button>
<button class="fs-home-nav-item" type="button" data-a="prefs">${icon('settings', 16)}<span>Preferences</span></button>
</div></nav>
<div class="fs-home-resizer" role="separator" aria-orientation="vertical" aria-label="Resize the side panel"></div>
<button class="fs-home-nav-hide" type="button" title="Hide the side panel" aria-label="Hide the side panel">${icon('chevronLeft', 14)}</button>
<button class="fs-home-nav-show" type="button" title="Show the side panel" aria-label="Show the side panel">${icon('chevronRight', 14)}</button>
<div class="fs-home-main">
<div class="fs-home-narrow">${n('start', 'Start', 'home')}${n('recent', 'Recent', 'cube')}${n('templates', 'Templates', 'template')}${n('trash', 'Trash', 'trash')}</div>
<div class="fs-home-confirm" role="alertdialog" aria-live="assertive" hidden><span class="fs-home-confirm-text" data-no-i18n></span><button class="fs-btn fs-btn-sm" type="button" data-a="confirm-no">Cancel</button><button class="fs-btn fs-btn-sm fs-btn-danger" type="button" data-a="confirm-yes" data-no-i18n></button></div>
<div data-page="start">
<div class="fs-home-head"><div><h1 class="fs-home-title" data-no-i18n></h1><p class="fs-home-sub">Open a mesh or a project, or start from a template.</p></div><div class="fs-home-head-actions"><button class="fs-btn" type="button" data-a="open">${icon('open', 14)}<span>Open file</span></button><button class="fs-btn" type="button" data-a="blank">${icon('plus', 14)}<span>Blank scene</span></button></div></div>
<div class="fs-home-hero">
<span class="fs-home-hero-icon">${icon('inbox', 26)}</span>
<div class="fs-home-hero-text"><div class="fs-home-hero-title">Start from your mesh</div>
<div class="fs-home-hero-sub">Drop a mesh or a formsculpt project anywhere in the window to open it. A mesh comes in ready to sculpt; a project opens with every object, layer and mask as you left them.</div>
<div class="fs-home-formats" translate="no" data-no-i18n>${FORMATS.map((f) => `<span class="fs-home-format">${f}</span>`).join('')}</div></div>
<div class="fs-home-hero-actions"><button class="fs-btn fs-btn-primary" type="button" data-a="choose">${icon('upload', 14)}<span>Choose a file</span></button><button class="fs-btn" type="button" data-a="new">${icon('cube', 14)}<span>New sculpt</span></button><button class="fs-btn fs-btn-ghost" type="button" data-a="agents">${icon('agent', 14)}<span>Use your agent</span></button></div>
</div>
<section class="fs-home-section"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Recent sculpts</h2><button class="fs-btn fs-btn-ghost fs-btn-sm" type="button" data-v="recent">See all</button></div><div class="fs-card-grid fs-recent-grid"><button class="fs-card is-skeleton" type="button" tabindex="-1" data-key="__skeleton" aria-hidden="true"><span class="fs-card-preview"></span><span class="fs-card-body"><span class="fs-card-name" data-no-i18n>&nbsp;</span><span class="fs-card-meta" data-no-i18n>&nbsp;</span></span></button><button class="fs-card is-empty" type="button" tabindex="-1" hidden><span class="fs-card-preview is-blank">${icon('cube', 28)}</span><span class="fs-card-body"><span class="fs-card-name">No sculpts yet</span><span class="fs-card-meta">Start from your mesh or a template</span></span></button></div></section>
${demos}
<section class="fs-home-section"><div class="fs-home-section-head"><h2 class="fs-home-section-title">Templates</h2>${filters}</div><div class="fs-card-grid fs-tpl-grid"></div></section>
</div>
<div data-page="recent" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Recent sculpts</h1><p class="fs-home-sub">Everything in your library, newest first.</p></div></div><div class="fs-card-grid fs-recent-full"></div><p class="fs-note fs-recent-none" hidden>No sculpts yet</p></div>
<div data-page="trash" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Trash</h1><p class="fs-home-sub">Deleted sculpts wait here until you restore them or delete them permanently.</p></div><div class="fs-home-head-actions"><button class="fs-btn fs-btn-empty-trash" type="button" data-a="empty-trash" disabled>${icon('trash', 14)}<span>Empty Trash</span></button></div></div><div class="fs-trash-list"></div><p class="fs-note fs-trash-none">Trash is empty.</p></div>
<div data-page="templates" hidden><div class="fs-home-head"><div><h1 class="fs-home-title">Templates</h1><p class="fs-home-sub">Starting meshes built by the engine. Each opens as a new sculpt in your library.</p></div></div>
<div class="fs-home-section-head">${filters}</div><div class="fs-card-grid fs-tpl-full"></div></div>
</div></section>`);
    this.tplGrid = this.root.querySelector('.fs-tpl-grid') as HTMLElement;
    this.tplGridFull = this.root.querySelector('.fs-tpl-full') as HTMLElement;
    this.recentGrid = this.root.querySelector('.fs-recent-grid') as HTMLElement;
    this.recentGridFull = this.root.querySelector('.fs-recent-full') as HTMLElement;
    this.trashList = this.root.querySelector('.fs-trash-list') as HTMLElement;
    this.confirmBar = this.root.querySelector('.fs-home-confirm') as HTMLElement;
    this.menu = fromHtml('<div class="fs-menu fs-home-menu" role="menu" data-no-i18n hidden></div>');
    overlayRoot().appendChild(this.menu);
    this.preview = new TemplateDialog(this.hooks);
    if (document.documentElement.classList.contains('has-overlay-titlebar')) this.root.querySelector('.fs-home-brand')?.setAttribute('data-tauri-drag-region', '');
    document.getElementById('app')?.appendChild(this.root);
    this.wire();
    this.sidePanel();
    this.show('start');
    this.renderTemplates();
    onLanguage(() => {
      if (!this.isOpen) return;
      this.greet();
      this.renderSculpts();
      this.renderTrash();
    });
  }

  private sidePanel(): void {
    const root = this.root;
    const bar = root.querySelector<HTMLElement>('.fs-home-resizer');
    if (!bar) return;
    const save = (): void => {
      try {
        localStorage.setItem(NAV_KEY, root.style.getPropertyValue('--fs-home-nav-w'));
        localStorage.setItem(NAV_HIDDEN_KEY, root.classList.contains('is-nav-hidden') ? '1' : '0');
      } catch {
        return;
      }
    };
    const hide = (on: boolean): void => {
      setClass(root, 'is-nav-hidden', on);
      save();
    };
    try {
      const w = localStorage.getItem(NAV_KEY);
      if (w && /^\d{3}px$/.test(w.trim())) root.style.setProperty('--fs-home-nav-w', w.trim());
      if (localStorage.getItem(NAV_HIDDEN_KEY) === '1') root.classList.add('is-nav-hidden');
    } catch {
      void 0;
    }
    root.querySelector('.fs-home-nav-hide')?.addEventListener('click', () => hide(true));
    root.querySelector('.fs-home-nav-show')?.addEventListener('click', () => hide(false));
    bar.addEventListener('pointerdown', (e) => {
      e.preventDefault();
      const x0 = e.clientX;
      const start = root.querySelector('.fs-home-nav')?.getBoundingClientRect().width || 248;
      bar.classList.add('is-dragging');
      const mv = (ev: PointerEvent): void => {
        const raw = start + ev.clientX - x0;
        const collapse = raw < 120;
        setClass(root, 'is-nav-hidden', collapse);
        if (!collapse) root.style.setProperty('--fs-home-nav-w', `${Math.round(Math.max(180, Math.min(420, raw)))}px`);
      };
      const up = (): void => {
        bar.classList.remove('is-dragging');
        window.removeEventListener('pointermove', mv);
        window.removeEventListener('pointerup', up);
        window.removeEventListener('pointercancel', up);
        save();
      };
      window.addEventListener('pointermove', mv);
      window.addEventListener('pointerup', up);
      window.addEventListener('pointercancel', up);
    });
  }

  private wire(): void {
    this.root.addEventListener('click', (e) => {
      const target = e.target as HTMLElement;
      this.closeMenu();
      const v = (target.closest('[data-v]') as HTMLElement | null)?.dataset.v as HomePage | undefined;
      if (v) {
        this.show(v);
        return;
      }
      if (target.closest('[data-open-sculpt]')) {
        this.hooks.back();
        return;
      }
      const a = (target.closest('[data-a]') as HTMLElement | null)?.dataset.a;
      const row = (target.closest('[data-entry]') as HTMLElement | null)?.dataset.entry;
      if (a === 'confirm-yes') {
        const go = this.onConfirm;
        this.askClose();
        go?.();
        return;
      }
      if (a === 'confirm-no') {
        this.askClose();
        return;
      }
      if (a === 'empty-trash') {
        const n = this.trashed.length;
        this.ask(count(n, t('Permanently delete 1 sculpt in the Trash? This cannot be undone.'), t('Permanently delete {n} sculpts in the Trash? This cannot be undone.')), t('Empty Trash'), () => void this.purge(null));
        return;
      }
      if (a === 'restore' && row) {
        void this.restore([row]);
        return;
      }
      if (a === 'purge' && row) {
        const x = this.trashed.find((d) => d.entry === row);
        this.ask(t('Permanently delete "{name}"? This cannot be undone.').replace('{name}', x?.title ?? ''), t('Delete permanently'), () => void this.purge(row));
        return;
      }
      const more = target.closest('.fs-card-more') as HTMLElement | null;
      if (more) {
        e.stopPropagation();
        const id = (more.closest('.fs-card') as HTMLElement).dataset.sculpt;
        const r = more.getBoundingClientRect();
        if (id) this.openMenu(id, r.left, r.bottom + 4);
        return;
      }
      const card = target.closest('.fs-card') as HTMLElement | null;
      if (card?.dataset.sculpt && (e.ctrlKey || e.metaKey || e.shiftKey)) {
        this.pickCard(card, e.shiftKey);
        return;
      }
      if (a === 'blank') this.hooks.blankScene();
      else if (a === 'open' || a === 'choose') this.pick();
      else if (a === 'new') void this.hooks.createSculpt('');
      else if (a === 'prefs') this.hooks.preferences();
      else if (a === 'agents') this.hooks.agents();
      else if (a === 'back') this.hooks.back();
      else if (a === 'local') toast(t('Your sculpts stay in your Formsculpt folder on this computer.'));
      if (card?.dataset.demo) {
        this.hooks.demo(card.dataset.demo);
        return;
      }
      if (card?.dataset.sculpt) this.hooks.openSculpt(card.dataset.sculpt);
      if (card?.dataset.template) {
        const tpl = this.templates.find((x) => x.id === card.dataset.template);
        if (tpl) this.preview.show(tpl);
      }
      const chip = target.closest('.fs-chip') as HTMLElement | null;
      if (chip?.dataset.cat) {
        this.category = chip.dataset.cat;
        this.renderTemplates();
      }
    });
    for (const input of Array.from(this.root.querySelectorAll<HTMLInputElement>('.fs-tpl-search'))) {
      input.addEventListener('input', () => {
        this.query = input.value.trim().toLowerCase();
        for (const other of Array.from(this.root.querySelectorAll<HTMLInputElement>('.fs-tpl-search'))) if (other !== input && other.value !== input.value) other.value = input.value;
        this.renderTemplates();
      });
    }
    this.root.addEventListener('contextmenu', (e) => {
      const card = (e.target as HTMLElement).closest('.fs-card') as HTMLElement | null;
      if (!card?.dataset.sculpt) return;
      e.preventDefault();
      if (!this.selected.has(card.dataset.sculpt)) {
        this.clearSelection();
        this.setSelected(card.dataset.sculpt, true);
      }
      this.openMenu(card.dataset.sculpt, e.clientX, e.clientY);
    });
    const keys = (e: KeyboardEvent): void => {
      if (e.key === 'Escape') {
        if (this.escape()) e.preventDefault();
        return;
      }
      if (e.key !== 'Delete') return;
      const card = (e.target as HTMLElement).closest?.('.fs-card') as HTMLElement | null;
      const id = card?.dataset.sculpt;
      if (!id) return;
      e.preventDefault();
      this.requestDelete(this.selected.has(id) ? [...this.selected] : [id]);
    };
    this.root.addEventListener('keydown', (e) => {
      keys(e);
      if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
    });
    this.menu.addEventListener('keydown', (e) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        this.closeMenu();
        this.root.focus({ preventScroll: true });
      }
      if (!(e.metaKey || e.ctrlKey)) e.stopPropagation();
    });
    document.addEventListener('keydown', (e) => {
      if (e.key !== 'Escape' || e.defaultPrevented || !this.isOpen || anyDialogOpen()) return;
      if (this.escape()) e.preventDefault();
    });
    this.menu.addEventListener('contextmenu', (e) => e.preventDefault());
    document.addEventListener('pointerdown', (e) => {
      if (!this.menu.hidden && !this.menu.contains(e.target as Node)) this.closeMenu();
    }, true);
    const hero = this.root.querySelector('.fs-home-hero') as HTMLElement;
    const files = (e: DragEvent): boolean => Array.from(e.dataTransfer?.types ?? []).includes('Files');
    this.root.addEventListener('dragover', (e) => {
      if (!files(e)) return;
      e.preventDefault();
      setClass(hero, 'is-drag-over', true);
    });
    this.root.addEventListener('dragleave', (e) => {
      if (!e.relatedTarget || !this.root.contains(e.relatedTarget as Node)) setClass(hero, 'is-drag-over', false);
    });
    this.root.addEventListener('drop', (e) => {
      setClass(hero, 'is-drag-over', false);
      if (!files(e)) return;
      e.preventDefault();
      const dropped = Array.from(e.dataTransfer?.files ?? []);
      if (dropped.length) this.hooks.openFiles(dropped);
    });
  }

  private escape(): boolean {
    if (!this.menu.hidden) {
      this.closeMenu();
      return true;
    }
    if (!this.confirmBar.hidden) {
      this.askClose();
      return true;
    }
    if (this.selected.size) {
      this.clearSelection();
      return true;
    }
    if (!this.documentOpen) return false;
    this.hooks.back();
    return true;
  }

  private pick(): void {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = ACCEPT;
    input.onchange = () => {
      const chosen = Array.from(input.files ?? []);
      if (chosen.length) this.hooks.openFiles(chosen);
    };
    input.click();
  }

  private greet(): void {
    const h = new Date().getHours();
    write(this.root.querySelector('[data-page="start"] .fs-home-title'), h < 12 ? t('Good morning') : h < 18 ? t('Good afternoon') : t('Good evening'));
  }

  async load(): Promise<void> {
    const turn = ++this.loads;
    const [tp, l, tr, st] = await Promise.all([
      library.templates().catch(() => null),
      library.list().catch(() => null),
      library.trash().catch(() => ({ sculpts: [] as TrashedSculpt[] })),
      library.settings().catch(() => ({}) as { pinned?: unknown; recentHidden?: unknown }),
    ]);
    if (turn !== this.loads) return;
    const names = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);
    this.pinned = names(st.pinned);
    this.hiddenRecent = names(st.recentHidden);
    if (tp) {
      this.templates = tp.templates;
      this.categories = [ALL, ...tp.categories];
    }
    this.sculpts = l?.sculpts ?? [];
    this.trashed = tr.sculpts;
    for (const id of [...this.selected]) if (!this.sculpts.some((d) => d.id === id)) this.selected.delete(id);
    write(this.root.querySelector('.fs-tpl-count'), String(this.templates.length));
    this.renderTemplates();
    this.renderSculpts();
    this.renderTrash();
  }

  defaultTemplate(): string {
    return this.templates[0]?.id ?? 'sphere';
  }

  ask(text: string, action: string, go: () => void): void {
    this.onConfirm = go;
    write(this.confirmBar.querySelector('.fs-home-confirm-text'), text);
    write(this.confirmBar.querySelector('[data-a="confirm-yes"]'), action);
    setHidden(this.confirmBar, false);
    (this.confirmBar.querySelector('[data-a="confirm-no"]') as HTMLElement).focus();
  }

  private askClose(): void {
    this.onConfirm = null;
    setHidden(this.confirmBar, true);
  }

  private closeMenu(): void {
    setHidden(this.menu, true);
  }

  private openMenu(id: string, x: number, y: number): void {
    const ids = this.selected.has(id) && this.selected.size > 1 ? [...this.selected] : [id];
    const many = ids.length > 1;
    const rows: Array<[string, string, () => void]> = [];
    if (!many) {
      rows.push([t('Open'), 'open', () => this.hooks.openSculpt(id)]);
      rows.push([t('Duplicate'), 'dup', () => {
        void library.duplicate(id).then(() => this.load()).then(() => toast(t('Duplicated.'))).catch((e) => toast(why(e), 'error'));
      }]);
    }
    const allPinned = ids.every((x) => this.pinned.includes(x));
    rows.push([allPinned ? t('Unpin from list') : t('Pin to list'), 'pin', () => this.setPinned(ids, !allPinned)]);
    rows.push([t('Remove from list'), 'hide', () => this.setHiddenRecent(ids, true)]);
    rows.push([many ? t('Delete {n} sculpts').replace('{n}', String(ids.length)) : t('Delete'), 'del', () => this.requestDelete(ids)]);
    this.menu.textContent = '';
    for (const [label, key, run] of rows) {
      const b = fromHtml<HTMLButtonElement>(`<button class="fs-menu-item" role="menuitem" type="button" data-k="${key}"><span class="fs-menu-label"></span></button>`);
      write(b.querySelector('.fs-menu-label'), label);
      b.addEventListener('click', () => {
        this.closeMenu();
        run();
      });
      this.menu.appendChild(b);
    }
    this.menu.style.visibility = 'hidden';
    setHidden(this.menu, false);
    const w = this.menu.offsetWidth;
    const h = this.menu.offsetHeight;
    this.menu.style.left = `${Math.round(Math.max(4, Math.min(x, window.innerWidth - w - 4)))}px`;
    this.menu.style.top = `${Math.round(Math.max(4, Math.min(y, window.innerHeight - h - 4)))}px`;
    this.menu.style.visibility = '';
    (this.menu.firstElementChild as HTMLElement | null)?.focus();
  }

  private saveRecent(): void {
    this.renderSculpts();
    void library.setSettings({ pinned: this.pinned, recentHidden: this.hiddenRecent }).catch((e) => toast(why(e), 'error'));
  }

  private setPinned(ids: string[], on: boolean): void {
    this.pinned = on ? [...this.pinned.filter((x) => !ids.includes(x)), ...ids] : this.pinned.filter((x) => !ids.includes(x));
    if (on) this.hiddenRecent = this.hiddenRecent.filter((x) => !ids.includes(x));
    this.saveRecent();
  }

  private setHiddenRecent(ids: string[], on: boolean): void {
    this.hiddenRecent = on ? [...this.hiddenRecent.filter((x) => !ids.includes(x)), ...ids] : this.hiddenRecent.filter((x) => !ids.includes(x));
    if (on) this.pinned = this.pinned.filter((x) => !ids.includes(x));
    this.saveRecent();
  }

  private setSelected(id: string, on: boolean): void {
    if (on) this.selected.add(id);
    else this.selected.delete(id);
    for (const grid of [this.recentGrid, this.recentGridFull]) {
      for (const n of Array.from(grid.children) as HTMLElement[]) if (n.dataset.sculpt === id) setClass(n, 'is-selected', on);
    }
  }

  private clearSelection(): void {
    for (const id of [...this.selected]) this.setSelected(id, false);
    this.anchor = '';
  }

  private pickCard(card: HTMLElement, range: boolean): void {
    const id = card.dataset.sculpt as string;
    if (range && this.anchor && this.anchor !== id) {
      const order = Array.from((card.parentElement as HTMLElement).querySelectorAll<HTMLElement>('.fs-card[data-sculpt]')).map((n) => n.dataset.sculpt as string);
      const a = order.indexOf(this.anchor);
      const b = order.indexOf(id);
      if (a >= 0 && b >= 0) for (let i = Math.min(a, b); i <= Math.max(a, b); i++) this.setSelected(order[i], true);
      return;
    }
    this.setSelected(id, !this.selected.has(id));
    this.anchor = id;
  }

  private requestDelete(ids: string[]): void {
    if (!ids.length) return;
    if (ids.length === 1) {
      void this.trashSculpts(ids);
      return;
    }
    const n = String(ids.length);
    this.ask(t('Move {n} sculpts to the Trash?').replace('{n}', n), t('Move {n} to Trash').replace('{n}', n), () => void this.trashSculpts(ids));
  }

  private async trashSculpts(ids: string[]): Promise<void> {
    const done: string[] = [];
    const entries: string[] = [];
    for (const id of ids) {
      try {
        entries.push((await library.remove(id)).entry);
        done.push(id);
      } catch (e) {
        toast(why(e), 'error');
      }
    }
    if (!done.length) return;
    for (const id of done) {
      this.selected.delete(id);
      this.hooks.closeSculpt(id);
    }
    const title = this.sculpts.find((d) => d.id === done[0])?.title ?? done[0];
    this.sculpts = this.sculpts.filter((d) => !done.includes(d.id));
    this.pinned = this.pinned.filter((x) => !done.includes(x));
    this.hiddenRecent = this.hiddenRecent.filter((x) => !done.includes(x));
    this.renderSculpts();
    const tr = await library.trash().catch(() => null);
    if (tr) this.trashed = tr.sculpts;
    this.renderTrash();
    this.undoToast(done.length === 1 ? t('Moved "{name}" to the Trash.').replace('{name}', title) : t('Moved {n} sculpts to the Trash.').replace('{n}', String(done.length)), entries);
  }

  private undoToast(text: string, entries: string[]): void {
    if (!entries.length) {
      toast(text);
      return;
    }
    const node = fromHtml('<div class="fs-toast fs-toast-undo" role="status" data-no-i18n><span class="fs-toast-text"></span><button class="fs-btn fs-btn-ghost fs-btn-sm" type="button"></button></div>');
    write(node.querySelector('.fs-toast-text'), text);
    write(node.querySelector('button'), t('Undo'));
    node.querySelector('button')?.addEventListener('click', () => {
      dismissToast(node);
      void this.restore(entries);
    });
    pushToast(toastRoot(), node, 8000);
  }

  private async restore(entries: string[]): Promise<void> {
    let n = 0;
    for (const entry of entries) {
      try {
        await library.restore(entry);
        n++;
      } catch (e) {
        toast(why(e), 'error');
      }
    }
    if (!n) return;
    const [l, tr] = await Promise.all([library.list().catch(() => null), library.trash().catch(() => null)]);
    if (l) this.sculpts = l.sculpts;
    if (tr) this.trashed = tr.sculpts;
    this.renderSculpts();
    this.renderTrash();
    toast(count(n, t('Sculpt restored.'), t('{n} sculpts restored.')));
  }

  private async purge(entry: string | null): Promise<void> {
    try {
      if (entry) await library.purge(entry);
      else await library.emptyTrash();
    } catch (e) {
      toast(why(e), 'error');
    }
    const tr = await library.trash().catch(() => null);
    this.trashed = tr ? tr.sculpts : entry ? this.trashed.filter((d) => d.entry !== entry) : [];
    this.renderTrash();
  }

  private renderTrash(): void {
    write(this.root.querySelector('.fs-trash-count'), String(this.trashed.length));
    write(this.root.querySelector('.fs-sculpt-count'), String(this.sculpts.length));
    const empty = this.root.querySelector<HTMLButtonElement>('.fs-btn-empty-trash');
    if (empty && empty.disabled !== !this.trashed.length) empty.disabled = !this.trashed.length;
    const none = this.root.querySelector<HTMLElement>('.fs-trash-none');
    if (none) setHidden(none, this.trashed.length > 0);
    const update = (n: HTMLElement, d: TrashedSculpt): void => {
      write(n.querySelector('.fs-card-name'), d.title);
      const facts = d.objects === null ? '' : `${count(d.objects, t('1 object'), t('{n} objects'))} · `;
      write(n.querySelector('.fs-card-meta'), `${facts}${t('Deleted {when}').replace('{when}', ago(new Date(d.deletedAt * 1000).toISOString()))}`);
    };
    const create = (d: TrashedSculpt): HTMLElement => {
      const n = fromHtml(`<div class="fs-trash-row" data-entry="${esc(d.entry)}"><span class="fs-trash-icon">${icon('cube', 18)}</span><span class="fs-trash-text"><span class="fs-card-name" data-no-i18n></span><span class="fs-card-meta" data-no-i18n></span></span><button class="fs-btn fs-btn-sm" type="button" data-a="restore">${icon('undo', 14)}<span>Restore</span></button><button class="fs-btn fs-btn-sm fs-btn-danger" type="button" data-a="purge">${icon('trash', 14)}<span>Delete permanently</span></button></div>`);
      update(n, d);
      return n;
    };
    patchList(this.trashList, this.trashed, { key: (d) => d.entry, create, update });
  }

  show(v: HomePage = this.view): void {
    this.view = v;
    for (const page of Array.from(this.root.querySelectorAll<HTMLElement>('[data-page]'))) setHidden(page, page.dataset.page !== v);
    for (const b of Array.from(this.root.querySelectorAll<HTMLElement>('.fs-home-nav-item[data-v]'))) setClass(b, 'is-active', b.dataset.v === v);
  }

  private open(): void {
    const was = this.isOpen;
    this.greet();
    setHidden(this.root, false);
    document.body.classList.add('nm-home');
    void this.load();
    if (was) return;
    if (!this.root.contains(document.activeElement)) this.root.focus({ preventScroll: true });
    this.hooks.onVisible?.(true);
  }

  private close(): void {
    const was = this.isOpen;
    setHidden(this.root, true);
    this.closeMenu();
    this.askClose();
    this.clearSelection();
    document.body.classList.remove('nm-home');
    if (was) this.hooks.onVisible?.(false);
  }

  get isOpen(): boolean {
    return !this.root.hidden;
  }

  request(page?: HomePage): void {
    if (page) this.show(page);
    this.forced = true;
    this.sync();
  }

  leave(): void {
    this.forced = false;
    this.sync();
  }

  sync(): void {
    const on = this.forced || (!this.documentOpen && (this.hadDocument ? this.hooks.afterClose() : this.hooks.autoShow()));
    if (on && !this.isOpen) this.open();
    else if (!on && this.isOpen) this.close();
  }

  setOpenSculpts(list: Array<{ id: string; title: string }>): void {
    const was = this.documentOpen;
    this.documentOpen = list.length > 0;
    if (this.documentOpen) this.hadDocument = true;
    if (this.documentOpen && !was) this.forced = false;
    const group = this.root.querySelector('.fs-home-open') as HTMLElement;
    setHidden(group, !list.length);
    setHidden(this.root.querySelector('.fs-home-back') as HTMLElement, !list.length);
    const name = (d: { title: string }): string => d.title.trim() || t('Untitled sculpt');
    patchList(group.querySelector('.fs-home-open-list') as HTMLElement, list, {
      key: (d) => d.id,
      create: (d) => {
        const node = fromHtml(`<button class="fs-home-nav-item" type="button" data-open-sculpt>${icon('cube', 16)}<span class="fs-home-open-name" data-no-i18n></span></button>`);
        write(node.querySelector('.fs-home-open-name'), name(d));
        return node;
      },
      update: (node, d) => write(node.querySelector('.fs-home-open-name'), name(d)),
    });
    this.sync();
  }

  private renderTemplates(): void {
    const q = this.query;
    const list = this.templates.filter((x) => (this.category === ALL || x.category === this.category) && (!q || `${x.name} ${x.category} ${x.tags.join(' ')} ${x.description} ${t(x.name)} ${t(x.category)}`.toLowerCase().includes(q)));
    for (const host of Array.from(this.root.querySelectorAll<HTMLElement>('.fs-cats'))) {
      const chip = (n: HTMLElement, c: string): void => {
        setClass(n, 'is-active', c === this.category);
        write(n.querySelector('.fs-chip-count'), !this.templates.length ? '' : c === ALL ? String(this.templates.length) : String(this.templates.filter((x) => x.category === c).length));
      };
      patchList(host, this.categories, {
        key: (c) => c,
        create: (c) => {
          const n = fromHtml(`<button class="fs-chip" type="button" data-cat="${esc(c)}"><span>${esc(c)}</span><span class="fs-chip-count" data-no-i18n></span></button>`);
          chip(n, c);
          return n;
        },
        update: chip,
      });
    }
    const card = (x: TemplateSummary): HTMLElement => {
      const n = fromHtml(`<button class="fs-card" type="button" data-template="${esc(x.id)}">${previewHtml('template')}<span class="fs-card-body"><span class="fs-card-name"></span><span class="fs-card-meta"></span></span></button>`);
      write(n.querySelector('.fs-card-name'), x.name);
      write(n.querySelector('.fs-card-meta'), x.category);
      setPreview(n, x.thumb ? library.templateThumbUrl(x.id) : '');
      return n;
    };
    patchList(this.tplGrid, list.slice(0, 15), { key: (x) => x.id, create: card, update: () => undefined });
    patchList(this.tplGridFull, list, { key: (x) => x.id, create: card, update: () => undefined });
  }

  private renderSculpts(): void {
    const update = (n: HTMLElement, d: SculptSummary): void => {
      write(n.querySelector('.fs-card-name'), d.title);
      const facts = d.objects === null ? size(d.bytes) : count(d.objects, t('1 object'), t('{n} objects'));
      write(n.querySelector('.fs-card-meta'), `${this.pinned.includes(d.id) ? `${t('Pinned')} · ` : ''}${facts} · ${ago(d.modified)}`);
      setClass(n, 'is-pinned', this.pinned.includes(d.id));
      setClass(n, 'is-selected', this.selected.has(d.id));
      setPreview(n, d.thumb ? library.thumbUrl(d) : '');
    };
    const make = (d: SculptSummary): HTMLElement => {
      const n = fromHtml(`<button class="fs-card" type="button">${previewHtml('cube')}<span class="fs-card-body"><span class="fs-card-name" data-no-i18n></span><span class="fs-card-meta" data-no-i18n></span></span><span class="fs-card-more" role="button" tabindex="-1" title="More actions (right-click)" aria-label="More actions (right-click)">${icon('more', 16)}</span></button>`);
      n.dataset.sculpt = d.id;
      update(n, d);
      return n;
    };
    const pinnedFirst = (list: SculptSummary[]): SculptSummary[] => [...list.filter((d) => this.pinned.includes(d.id)), ...list.filter((d) => !this.pinned.includes(d.id))];
    const recent = pinnedFirst(this.sculpts.filter((d) => !this.hiddenRecent.includes(d.id)));
    patchList(this.recentGrid, recent.slice(0, 5), { key: (d) => d.id, create: make, update });
    patchList(this.recentGridFull, pinnedFirst(this.sculpts), { key: (d) => d.id, create: make, update });
    const none = this.recentGrid.querySelector<HTMLElement>('.fs-card.is-empty');
    if (none) setHidden(none, recent.length > 0);
    const noneFull = this.root.querySelector<HTMLElement>('.fs-recent-none');
    if (noneFull) setHidden(noneFull, this.sculpts.length > 0);
    write(this.root.querySelector('.fs-sculpt-count'), String(this.sculpts.length));
  }
}

class TemplateDialog extends Dialog {
  private tpl: TemplateSummary | null = null;
  private readonly picture: HTMLImageElement;
  private readonly title: HTMLInputElement;

  constructor(hooks: HomeHooks) {
    super('', 520);
    const body = fromHtml('<div class="fs-home-tpl"><span class="fs-home-tpl-preview"><img alt="" decoding="async" draggable="false"></span><p class="fs-note">Opens as a new sculpt in your library.</p><input class="fs-input" type="text" placeholder="Sculpt title" maxlength="64" spellcheck="false" autocomplete="off"></div>');
    this.picture = body.querySelector('img') as HTMLImageElement;
    this.title = body.querySelector('input') as HTMLInputElement;
    this.body.appendChild(body);
    const use = (): void => {
      const tpl = this.tpl;
      if (!tpl) return;
      this.close();
      void hooks.createSculpt(tpl.id, this.title.value.trim() || undefined);
    };
    this.button('Cancel', 'ghost', () => this.close());
    this.button(t('Use template'), 'primary', use);
    this.title.addEventListener('keydown', (e) => {
      if (e.key !== 'Enter') return;
      e.preventDefault();
      use();
    });
  }

  show(tpl: TemplateSummary): void {
    this.tpl = tpl;
    this.setTitle(tpl.name);
    this.title.value = '';
    const url = tpl.thumb ? library.templateThumbUrl(tpl.id) : '';
    if (url && this.picture.getAttribute('src') !== url) this.picture.src = url;
    else if (!url) this.picture.removeAttribute('src');
    this.open(this.title);
  }
}

class HomePrefsDialog extends Dialog {
  private readonly launch: HTMLInputElement;
  private readonly lastClose: HTMLInputElement;

  constructor(changed: () => void) {
    super('Preferences', 440);
    const row = (label: string): HTMLLabelElement => fromHtml<HTMLLabelElement>(`<label class="fs-home-check"><input type="checkbox"><span>${label}</span></label>`);
    const a = row('Show Home Screen at Launch');
    const b = row('Show Home Screen When the Last Document Closes');
    this.launch = a.querySelector('input') as HTMLInputElement;
    this.lastClose = b.querySelector('input') as HTMLInputElement;
    this.body.append(a, b);
    this.launch.addEventListener('change', () => {
      updatePrefs({ autoShowHome: this.launch.checked });
      changed();
    });
    this.lastClose.addEventListener('change', () => {
      updatePrefs({ homeOnLastClose: this.lastClose.checked });
      changed();
    });
    this.button('Close', 'primary', () => this.close());
  }

  show(): void {
    const p = loadPrefs();
    this.launch.checked = p.autoShowHome;
    this.lastClose.checked = p.homeOnLastClose;
    this.open(this.launch);
  }
}

let home: Home | null = null;
let host: HomeHost | null = null;
let current: OpenSculpt | null = null;
let decided = false;
let settle: () => void = () => undefined;
let prefsDialog: HomePrefsDialog | null = null;

export const homeReady: Promise<void> = new Promise((resolve) => {
  settle = resolve;
  window.setTimeout(resolve, 8000);
});

function remember(): void {
  try {
    if (current?.id) sessionStorage.setItem(SESSION_KEY, JSON.stringify({ id: current.id, title: current.title }));
    else sessionStorage.removeItem(SESSION_KEY);
  } catch {
    return;
  }
}

function remembered(): OpenSculpt | null {
  try {
    const raw = JSON.parse(sessionStorage.getItem(SESSION_KEY) ?? 'null') as { id?: unknown; title?: unknown } | null;
    return raw && typeof raw.id === 'string' && typeof raw.title === 'string' ? { id: raw.id, title: raw.title, blank: false } : null;
  } catch {
    return null;
  }
}

function setCurrent(next: OpenSculpt | null): void {
  current = next;
  remember();
  home?.setOpenSculpts(current ? [{ id: 'current', title: current.title }] : []);
}

function tidyName(raw: string): string {
  const name = raw.replace(/\.[A-Za-z0-9]{2,5}$/, '').replace(/[^A-Za-z0-9 _.-]+/g, ' ').replace(/\s+/g, ' ').trim().replace(/^[^A-Za-z0-9]+/, '').slice(0, 56).replace(/[ .]+$/, '');
  return name || UNTITLED;
}

async function openSculpt(id: string): Promise<void> {
  if (!host) return;
  if (current?.id === id) {
    home?.leave();
    return;
  }
  try {
    const reply = await library.open(id);
    host.replaced();
    setCurrent({ id: reply.sculpt.id, title: reply.sculpt.title, blank: false });
    home?.leave();
    await host.refresh();
    host.frame();
  } catch (e) {
    toast(why(e), 'error');
  }
}

async function createSculpt(template: string, title?: string): Promise<void> {
  if (!host) return;
  try {
    const made = await library.create(template || home?.defaultTemplate() || 'sphere', title ? tidyName(title) : undefined);
    await openSculpt(made.id);
  } catch (e) {
    toast(why(e), 'error');
  }
}

async function blankScene(): Promise<void> {
  if (!host) return;
  const reply = await dispatch('new_scene');
  if (!reply.ok) {
    toast(why(reply.error.message), 'error');
    return;
  }
  host.replaced();
  setCurrent({ id: null, title: '', blank: true });
  home?.leave();
  await host.refresh();
}

async function openFiles(files: File[]): Promise<void> {
  const file = files[0];
  if (!host || !file) return;
  const problem = await host.importFile(file);
  if (problem !== null) {
    toast(why(problem), 'error');
    return;
  }
  setCurrent({ id: null, title: tidyName(file.name), blank: host.objects() === 0 });
  home?.leave();
}

function agents(): void {
  const cmd = commands.get('agent.claude');
  if (!cmd || browserEngine() || cmd.enabled?.() === false) {
    toast(t('Agents need the engine service; this window is running the browser engine.'));
    return;
  }
  if (current) home?.leave();
  else void blankScene();
  commands.run('agent.claude');
}

function preferences(): void {
  if (commands.get('app.preferences')) {
    commands.run('app.preferences');
    return;
  }
  prefsDialog ??= new HomePrefsDialog(() => home?.sync());
  prefsDialog.show();
}

function guarded(go: () => void): void {
  if (!current || !unsavedWork()) go();
  else home?.ask(t('This sculpt has changes that are not saved. Discard them?'), t('Discard'), go);
}

export function mountHome(hooks: Partial<HomeHooks> = {}): Home {
  if (home) return home;
  home = new Home({
    openSculpt: (id) => {
      if (current?.id === id) home?.leave();
      else guarded(() => void openSculpt(id));
    },
    closeSculpt: (id) => {
      if (current?.id === id) setCurrent({ ...current, id: null });
    },
    createSculpt: (template, title) => {
      guarded(() => void createSculpt(template, title));
      return Promise.resolve();
    },
    blankScene: () => guarded(() => void blankScene()),
    openFiles: (files) => guarded(() => void openFiles(files)),
    preferences,
    agents,
    demo: () => undefined,
    back: () => home?.leave(),
    autoShow: () => loadPrefs().autoShowHome && !browserEngine(),
    afterClose: () => loadPrefs().homeOnLastClose && !browserEngine(),
    ...hooks,
  });
  return home;
}

export function attachHome(next: HomeHost): void {
  host = next;
}

export function showHome(page?: HomePage): void {
  home?.request(page);
}

export function hideHome(): void {
  home?.leave();
}

export function isHomeOpen(): boolean {
  return home?.isOpen ?? false;
}

export function homeScene(objects: number): void {
  if (!decided) {
    decided = true;
    setCurrent(objects > 0 ? remembered() ?? { id: null, title: '', blank: false } : null);
    settle();
    return;
  }
  if (objects > 0 && !current) setCurrent({ id: null, title: '', blank: false });
  else if (objects > 0 && current?.blank) current.blank = false;
}

export function homeSceneEnded(ended: boolean): void {
  if (ended && decided && current && !current.blank) setCurrent(null);
}

export function homeSceneReplaced(): void {
  if (current?.id) setCurrent({ ...current, id: null });
}

function thumbnail(canvas: HTMLCanvasElement): Promise<Blob | null> {
  return new Promise((resolve) => {
    const giveUp = window.setTimeout(() => resolve(null), 4000);
    requestAnimationFrame(() => {
      try {
        const out = document.createElement('canvas');
        out.width = 480;
        out.height = 270;
        const g = out.getContext('2d');
        if (!g || !canvas.width || !canvas.height) throw new Error('no picture');
        const scale = Math.max(out.width / canvas.width, out.height / canvas.height);
        const w = canvas.width * scale;
        const h = canvas.height * scale;
        g.drawImage(canvas, (out.width - w) / 2, (out.height - h) / 2, w, h);
        out.toBlob((blob) => {
          window.clearTimeout(giveUp);
          resolve(blob);
        }, 'image/webp', 0.82);
      } catch {
        window.clearTimeout(giveUp);
        resolve(null);
      }
    });
  });
}

export async function saveSculpt(): Promise<void> {
  if (!host) return;
  if (browserEngine()) {
    await host.download();
    return;
  }
  if (host.objects() === 0) {
    host.note(t('The scene is empty; there is nothing to save'));
    return;
  }
  try {
    const picture = await thumbnail(host.picture());
    const saved = current?.id ? await library.save(current.id, !!picture) : await library.keepScene(tidyName(current?.title || host.sceneName()), !!picture);
    if (picture) await library.putThumb(saved.id, picture).catch(() => undefined);
    setCurrent({ id: saved.id, title: saved.title, blank: false });
    sceneSaved();
    host.note(t('Saved {name} to the library').replace('{name}', saved.title));
    if (home?.isOpen) void home.load();
  } catch (e) {
    host.note(why(e));
  }
}

export function homeCommands(): Command[] {
  const flip = (key: 'autoShowHome' | 'homeOnLastClose'): void => {
    updatePrefs({ [key]: !loadPrefs()[key] });
    home?.sync();
  };
  return [
    { id: 'file.save_file', label: 'Save Project File...', icon: 'save', run: () => void host?.download() },
    { id: 'pref.home_launch', label: 'Show Home Screen at Launch', run: () => flip('autoShowHome'), checked: () => loadPrefs().autoShowHome },
    { id: 'pref.home_last_close', label: 'Show Home Screen When the Last Document Closes', run: () => flip('homeOnLastClose'), checked: () => loadPrefs().homeOnLastClose },
  ];
}
