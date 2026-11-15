import type { Commands } from './commands';
import type { MenuDef, MenuItem as MenuSpec } from './menu';
import { anyDialogOpen } from './dialog';
import { t } from './i18n';

export const MENUS_IN_WINDOW_KEY = 'formsculpt.menusInWindow';
export function menusInWindow(): boolean {
  try {
    return localStorage.getItem(MENUS_IN_WINDOW_KEY) === '1';
  } catch {
    return false;
  }
}
export const isMacDesktop = (): boolean => '__TAURI_INTERNALS__' in window && /Mac/.test(navigator.platform);
export const wantsNativeMenus = (): boolean => isMacDesktop() && !menusInWindow();

const NAMES: Record<string, string> = {
  Mod: 'CmdOrCtrl', Alt: 'Alt', Shift: 'Shift', Ctrl: 'Ctrl', Plus: 'Plus', '=': '=', '-': '-', '[': '[', ']': ']', ',': ',', '.': '.', '/': '/', ';': ';', "'": "'", '\\': '\\', '`': '`',
  Delete: 'Delete', Backspace: 'Backspace', Escape: 'Escape', Enter: 'Enter', Tab: 'Tab', Space: 'Space',
  ArrowUp: 'Up', ArrowDown: 'Down', ArrowLeft: 'Left', ArrowRight: 'Right', Up: 'Up', Down: 'Down', Left: 'Left', Right: 'Right',
  Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
};

export function accelerator(combo: string | undefined): string | null {
  if (!combo) return null;
  const parts = combo.endsWith('++') ? [...combo.slice(0, -2).split('+'), 'Plus'] : combo.split('+');
  const key = parts[parts.length - 1];
  const fkey = /^F([1-9]|1[0-2])$/.test(key);
  if (!fkey && !parts.some((p) => p === 'Mod' || p === 'Ctrl' || p === 'Alt')) return null;
  const out: string[] = [];
  for (const p of parts) {
    if (NAMES[p]) out.push(NAMES[p]);
    else if (/^[A-Za-z0-9]$/.test(p)) out.push(p.toUpperCase());
    else if (/^F([1-9]|1[0-2])$/.test(p)) out.push(p);
    else return null;
  }
  return out.join('+');
}

export type TextEdit = 'undo' | 'redo' | 'cut' | 'copy' | 'paste' | 'select-all';

export const TEXT_EDITS: Readonly<Record<string, TextEdit>> = {
  'history.undo': 'undo',
  'history.redo': 'redo',
  'edit.cut': 'cut',
  'edit.copy': 'copy',
  'edit.paste': 'paste',
  'edit.select_all': 'select-all',
};

const NOT_TEXT = new Set(['button', 'checkbox', 'radio', 'range', 'color', 'file', 'submit', 'reset', 'image']);

export function textFieldFocused(): boolean {
  const a = document.activeElement;
  if (a instanceof HTMLTextAreaElement) return !a.readOnly && !a.disabled;
  if (a instanceof HTMLInputElement) return !NOT_TEXT.has(a.type) && !a.readOnly && !a.disabled;
  return a instanceof HTMLElement && a.isContentEditable;
}

export function textEditReady(edit: TextEdit): boolean {
  return textFieldFocused() || (edit === 'copy' && !!window.getSelection()?.toString());
}

const RESERVED: readonly string[] = ['CmdOrCtrl+Q', 'CmdOrCtrl+H', 'CmdOrCtrl+M'];
const PREFERENCES = 'app.preferences';
const PRODUCT = 'FormSculpt';

type Api = typeof import('@tauri-apps/api/menu');
type Live = { setEnabled(on: boolean): Promise<void>; setChecked?(on: boolean): Promise<void> };

export class NativeMenus {
  private items: Array<{ id: string; item: Live }> = [];
  private building = false;
  private again = false;
  private stateTimer = 0;
  private shape = '';
  private builtAt = 0;
  private inflight = '';

  constructor(private readonly commands: Commands, private readonly menus: () => MenuDef[]) {
    for (const ev of ['pointerup', 'keyup', 'focusin', 'focusout'] as const) {
      window.addEventListener(ev, () => { if (this.items.length) this.refresh(); }, { passive: true });
    }
  }

  private signature(): string {
    return JSON.stringify(this.menus().map((d) => (typeof d.items === 'function' ? d.items() : d.items)));
  }

  private enabled(id: string): boolean {
    if (TEXT_EDITS[id] && textEditReady(TEXT_EDITS[id])) return true;
    if (anyDialogOpen()) return false;
    const cmd = this.commands.get(id);
    return !!cmd && this.commands.enabled(cmd);
  }

  async build(force = false): Promise<void> {
    const sigNow = this.signature();
    if (this.building) {
      if (!force && sigNow === this.inflight) return;
      this.again = true;
      return;
    }
    if (!force && this.items.length && performance.now() - this.builtAt < 300 && sigNow === this.shape) return;
    this.building = true;
    this.inflight = sigNow;
    try {
      const api: Api = await import('@tauri-apps/api/menu');
      const items: typeof this.items = [];
      const taken = new Set<string>(RESERVED);
      const sep =(): Promise<unknown> => api.PredefinedMenuItem.new({ item: 'Separator' });
      const make = async (spec: MenuSpec): Promise<unknown> => {
        if (spec === '-') return sep();
        if (typeof spec === 'object') return api.Submenu.new({ text: t(spec.label), items: (await Promise.all(spec.items.map(make))).filter(Boolean) as never[] });
        const cmd = this.commands.get(spec);
        if (!cmd) return null;
        const text = t(this.commands.label(cmd));
        const key = accelerator(cmd.keys?.[0]);
        const acc = key && !taken.has(key) ? key : null;
        if (acc) taken.add(acc);
        const edit = TEXT_EDITS[spec];
        const opts = {
          text,
          enabled: this.enabled(spec),
          action: () => {
            if (edit && textEditReady(edit)) {
              void import('@tauri-apps/api/core').then((m) => m.invoke('text_edit', { action: edit })).catch(() => { document.execCommand(edit === 'select-all' ? 'selectAll' : edit); });
            } else if (!anyDialogOpen()) this.commands.run(spec);
            this.refresh();
          },
        };
        let item: Live;
        if (cmd.checked) {
          const copts = { ...opts, checked: !!cmd.checked() };
          try { item = await api.CheckMenuItem.new(acc ? { ...copts, accelerator: acc } : copts); } catch { item = await api.CheckMenuItem.new(copts); }
        } else {
          try { item = await api.MenuItem.new(acc ? { ...opts, accelerator: acc } : opts); } catch { item = await api.MenuItem.new(opts); }
        }
        items.push({ id: spec, item });
        return item;
      };
      const top: unknown[] = [];
      top.push(await api.Submenu.new({ text: PRODUCT, items: [
        await api.PredefinedMenuItem.new({ item: { About: { name: PRODUCT, copyright: 'Copyright 2026 nodemode-ai. Apache-2.0', license: 'Apache-2.0' } }, text: t('About formsculpt') }),
        await sep(),
        ...(this.commands.get(PREFERENCES) ? [await make(PREFERENCES), await sep()] : []),
        await api.PredefinedMenuItem.new({ item: 'Services' }),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'Hide' }),
        await api.PredefinedMenuItem.new({ item: 'HideOthers' }),
        await api.PredefinedMenuItem.new({ item: 'ShowAll' }),
        await sep(),
        await api.PredefinedMenuItem.new({ item: 'Quit' }),
      ] as never[] }));
      let windows: { setAsWindowsMenuForNSApp(): Promise<void> } | null = null;
      let help: { setAsHelpMenuForNSApp(): Promise<void> } | null = null;
      for (const def of this.menus()) {
        const list = typeof def.items === 'function' ? def.items() : def.items;
        const made = (await Promise.all(list.map(make))).filter(Boolean);
        if (def.label === 'Window') {
          made.push(
            ...(made.length ? [await sep()] : []),
            await api.PredefinedMenuItem.new({ item: 'Minimize' }),
            await api.PredefinedMenuItem.new({ item: 'Maximize' }),
            await api.PredefinedMenuItem.new({ item: 'Fullscreen' }),
            await sep(),
            await api.PredefinedMenuItem.new({ item: 'BringAllToFront' }),
          );
        }
        const sub = await api.Submenu.new({ text: t(def.label), items: made as never[] });
        if (def.label === 'Window') windows = sub;
        if (def.label === 'Help') help = sub;
        top.push(sub);
      }
      const menu = await api.Menu.new({ items: top as never[] });
      await menu.setAsAppMenu();
      if (windows) void windows.setAsWindowsMenuForNSApp().catch(() => undefined);
      if (help) void help.setAsHelpMenuForNSApp().catch(() => undefined);
      this.items = items;
      this.shape = sigNow;
      this.builtAt = performance.now();
      document.documentElement.classList.add('has-native-menus');
    } catch (err) {
      if (!this.items.length) document.documentElement.classList.remove('has-native-menus');
      throw err;
    } finally {
      this.building = false;
      if (this.again) {
        this.again = false;
        void this.build().catch(() => undefined);
      }
    }
  }

  refresh(): void {
    if (this.stateTimer || !this.items.length) return;
    this.stateTimer = window.setTimeout(() => {
      this.stateTimer = 0;
      if (this.signature() !== this.shape) {
        void this.build().catch(() => undefined);
        return;
      }
      for (const { id, item } of this.items) {
        const cmd = this.commands.get(id);
        if (!cmd) continue;
        void item.setEnabled(this.enabled(id)).catch(() => undefined);
        if (cmd.checked && item.setChecked) void item.setChecked(!!cmd.checked()).catch(() => undefined);
      }
    }, 150);
  }
}
