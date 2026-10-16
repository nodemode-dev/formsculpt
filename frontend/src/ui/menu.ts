import { fromHtml, q, setClass, setDisabled, setText, setTitle } from './dom';
import { icon } from './icons';
import type { Commands } from './commands';
import { onLanguage, t } from './i18n';
import { overlayRoot } from './dialog';

export type MenuItem = string | '-' | SubMenu;
export interface SubMenu {
  label: string;
  items: MenuItem[];
}
const isSub = (i: MenuItem): i is SubMenu => typeof i === 'object';

export class PopupMenu {
  readonly panel: HTMLElement;
  private rows: Array<{ id: string; node: HTMLButtonElement }> = [];
  private subs: Array<{ sub: SubMenu; node: HTMLButtonElement; menu: PopupMenu }> = [];
  private key = '';
  private openSub: PopupMenu | null = null;
  private subTop = 0;
  onClose: (() => void) | null = null;

  constructor(
    private readonly commands: Commands,
    private readonly parent: PopupMenu | null = null,
    standalone = false,
    layer: 'menu' | 'context' = 'menu',
  ) {
    this.panel = fromHtml<HTMLElement>(`<div class="fs-menu fs-menu-${layer}" role="menu" hidden></div>`);
    overlayRoot().appendChild(this.panel);
    this.panel.addEventListener('contextmenu', (e) => e.preventDefault());
    this.panel.addEventListener(
      'scroll',
      () => {
        if (this.panel.scrollTop !== this.subTop) this.closeSub();
      },
      { passive: true },
    );
    this.layer = layer;
    if (standalone) {
      document.addEventListener(
        'pointerdown',
        (e) => {
          if (this.isOpen && !this.contains(e.target as Node)) this.close();
        },
        true,
      );
      document.addEventListener(
        'keydown',
        (e) => {
          if (this.isOpen && e.key === 'Escape') this.close();
        },
        true,
      );
      window.addEventListener('blur', () => this.close());
    }
  }

  private readonly layer: 'menu' | 'context';

  get isOpen(): boolean {
    return !this.panel.hidden;
  }

  contains(node: Node): boolean {
    return this.panel.contains(node) || !!this.openSub?.contains(node);
  }

  setItems(items: MenuItem[]): void {
    const key = JSON.stringify(items);
    if (key === this.key) return;
    this.key = key;
    this.closeSub();
    for (const s of this.subs) s.menu.panel.remove();
    this.panel.replaceChildren();
    this.rows = [];
    this.subs = [];
    this.panel.classList.toggle('has-checks', items.some((i) => typeof i === 'string' && i !== '-' && !!this.commands.get(i)?.checked));
    for (const item of items) {
      if (item === '-') {
        this.panel.appendChild(fromHtml('<div class="fs-menu-sep"></div>'));
        continue;
      }
      if (isSub(item)) {
        const node = fromHtml<HTMLButtonElement>(
          `<button class="fs-menu-item fs-menu-sub" role="menuitem" aria-haspopup="menu" type="button"><span class="fs-menu-check"></span><span class="fs-menu-label"></span><span class="fs-menu-keys">${icon('chevronRight', 12)}</span></button>`,
        );
        setText(q(node, '.fs-menu-label'), t(item.label));
        const menu = new PopupMenu(this.commands, this, false, this.layer);
        menu.setItems(item.items);
        node.addEventListener('pointerenter', () => this.showSub(menu, node));
        node.addEventListener('click', () => this.showSub(menu, node));
        this.subs.push({ sub: item, node, menu });
        this.panel.appendChild(node);
        continue;
      }
      const cmd = this.commands.get(item);
      if (!cmd) continue;
      const node = fromHtml<HTMLButtonElement>(
        `<button class="fs-menu-item" role="menuitem" type="button"><span class="fs-menu-check">${icon('check', 14)}</span><span class="fs-menu-label"></span><span class="fs-menu-keys"></span></button>`,
      );
      setText(q(node, '.fs-menu-keys'), this.commands.keyLabel(cmd));
      node.addEventListener('pointerenter', () => this.closeSub());
      node.addEventListener('click', () => {
        this.root().close();
        this.commands.run(item);
      });
      this.rows.push({ id: item, node });
      this.panel.appendChild(node);
    }
  }

  private root(): PopupMenu {
    let m: PopupMenu = this;
    while (m.parent) m = m.parent;
    return m;
  }

  private showSub(menu: PopupMenu, row: HTMLElement): void {
    if (this.openSub === menu && menu.isOpen) return;
    this.closeSub();
    this.openSub = menu;
    this.subTop = this.panel.scrollTop;
    setClass(row, 'is-open', true);
    menu.onClose = () => setClass(row, 'is-open', false);
    menu.openBeside(row.getBoundingClientRect());
  }

  private closeSub(): void {
    const s = this.openSub;
    this.openSub = null;
    s?.close();
  }

  openAt(x: number, y: number): void {
    this.refresh();
    this.panel.style.visibility = 'hidden';
    this.panel.hidden = false;
    this.panel.scrollTop = 0;
    const { width: w, height: h } = this.panel.getBoundingClientRect();
    const fitsRight = x + w <= window.innerWidth - 4;
    const fitsBelow = y + h <= window.innerHeight - 4;
    const left = Math.max(4, Math.min(fitsRight ? x : x - w, window.innerWidth - w - 4));
    const top = Math.max(4, Math.min(fitsBelow ? y : y - h, window.innerHeight - h - 4));
    this.panel.style.left = `${Math.round(left)}px`;
    this.panel.style.top = `${Math.round(top)}px`;
    this.panel.style.visibility = '';
  }

  private openBeside(row: DOMRect): void {
    this.refresh();
    this.panel.style.visibility = 'hidden';
    this.panel.hidden = false;
    this.panel.scrollTop = 0;
    const w = this.panel.offsetWidth;
    const h = this.panel.offsetHeight;
    const pad = 4;
    const right = row.right - 2;
    const left = row.left - w + 2;
    const x = right + w <= window.innerWidth - pad ? right : Math.max(pad, left);
    const y = Math.max(pad, Math.min(row.top - pad, window.innerHeight - h - pad));
    this.panel.style.left = `${Math.round(x)}px`;
    this.panel.style.top = `${Math.round(y)}px`;
    this.panel.style.visibility = '';
  }

  private refresh(): void {
    for (const r of this.rows) {
      const cmd = this.commands.get(r.id);
      if (!cmd) continue;
      setText(q(r.node, '.fs-menu-label'), t(this.commands.label(cmd)));
      setText(q(r.node, '.fs-menu-keys'), this.commands.keyLabel(cmd));
      setDisabled(r.node, !this.commands.enabled(cmd));
      const why = t(this.commands.why(cmd));
      if (why || r.node.dataset.why) {
        setTitle(r.node, why);
        r.node.dataset.why = why;
      }
      setClass(r.node, 'is-checked', !!cmd.checked?.());
    }
  }

  relabel(): void {
    this.refresh();
    for (const s of this.subs) {
      setText(q(s.node, '.fs-menu-label'), t(s.sub.label));
      s.menu.relabel();
    }
  }

  close(): void {
    this.closeSub();
    if (this.panel.hidden) return;
    this.panel.hidden = true;
    this.onClose?.();
  }
}

export interface MenuDef {
  label: string;
  items: MenuItem[] | (() => MenuItem[]);
}

export const itemsOf = (def: MenuDef): MenuItem[] => (typeof def.items === 'function' ? def.items() : def.items);

export class MenuBar {
  private readonly bars: Array<{ btn: HTMLButtonElement; menu: PopupMenu; def: MenuDef }> = [];
  private openIndex = -1;

  constructor(host: HTMLElement, commands: Commands, defs: MenuDef[]) {
    host.setAttribute('role', 'menubar');
    for (const def of defs) {
      const btn = fromHtml<HTMLButtonElement>('<button class="fs-menu-btn" type="button" aria-haspopup="menu"></button>');
      setText(btn, t(def.label));
      host.appendChild(btn);
      const menu = new PopupMenu(commands);
      menu.setItems(itemsOf(def));
      const idx = this.bars.length;
      menu.onClose = () => {
        if (this.openIndex === idx) {
          this.openIndex = -1;
          setClass(btn, 'is-open', false);
        }
      };
      btn.addEventListener('pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        if (this.openIndex === idx) this.close();
        else this.open(idx);
      });
      btn.addEventListener('pointerenter', () => {
        if (this.openIndex >= 0 && this.openIndex !== idx) this.open(idx);
      });
      this.bars.push({ btn, menu, def });
    }
    document.addEventListener(
      'pointerdown',
      (e) => {
        if (this.openIndex < 0) return;
        const m = this.bars[this.openIndex];
        const target = e.target as Node;
        if (!m.menu.contains(target) && !m.btn.contains(target)) this.close();
      },
      true,
    );
    document.addEventListener(
      'keydown',
      (e) => {
        if (this.openIndex >= 0 && e.key === 'Escape') this.close();
      },
      true,
    );
    window.addEventListener('blur', () => this.close());
    window.addEventListener('resize', () => this.close());
    onLanguage(() => {
      for (const b of this.bars) {
        setText(b.btn, t(b.def.label));
        b.menu.relabel();
      }
    });
  }

  get isOpen(): boolean {
    return this.openIndex >= 0;
  }

  open(i: number): void {
    this.close();
    const b = this.bars[i];
    b.menu.setItems(itemsOf(b.def));
    this.openIndex = i;
    setClass(b.btn, 'is-open', true);
    const r = b.btn.getBoundingClientRect();
    b.menu.openAt(r.left, r.bottom + 4);
  }

  close(): void {
    if (this.openIndex < 0) return;
    const b = this.bars[this.openIndex];
    this.openIndex = -1;
    setClass(b.btn, 'is-open', false);
    b.menu.close();
  }
}
