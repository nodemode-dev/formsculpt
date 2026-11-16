import type { Commands } from './commands';
import { Dialog } from './dialog';
import { fromHtml, isMac, setText } from './dom';
import { onLanguage, t } from './i18n';
import type { MenuDef } from './menu';
import { isMacDesktop, MENUS_IN_WINDOW_KEY, menusInWindow, NativeMenus, TEXT_EDITS, textEditReady, textFieldFocused, wantsNativeMenus, type TextEdit } from './native-menu';

export const inShell = (): boolean => '__TAURI_INTERNALS__' in window;

async function invoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  const core = await import('@tauri-apps/api/core');
  return core.invoke<T>(cmd, args);
}

interface SceneSeen {
  objects: ReadonlyArray<{ revision: number }>;
  views?: unknown;
  groups?: unknown;
}

interface SceneMark {
  strict: string;
  loose: string;
  done: number;
  trail: string;
}

const SAVED_KEY = 'formsculpt.saved';
let seen: SceneMark | null = null;
let saved: SceneMark | null = null;
let empty = true;
let overwritten = false;
let lastSteps: readonly string[] = [];
let lastDone = 0;
let settle = true;
let firstSight = true;
let epoch = 0;

export const sceneEpoch = (): number => epoch;

function keepSaved(): void {
  try {
    if (saved) sessionStorage.setItem(SAVED_KEY, JSON.stringify(saved));
    else sessionStorage.removeItem(SAVED_KEY);
  } catch {
    return;
  }
}

function keptSaved(): SceneMark | null {
  try {
    const raw = JSON.parse(sessionStorage.getItem(SAVED_KEY) ?? 'null') as Partial<SceneMark> | null;
    return raw && typeof raw.strict === 'string' && typeof raw.loose === 'string' && typeof raw.done === 'number' && typeof raw.trail === 'string' ? (raw as SceneMark) : null;
  } catch {
    return null;
  }
}

export function noteScene(began: number, scene: SceneSeen, steps: readonly string[], done: number): void {
  if (began !== epoch) return;
  const trail = steps.slice(0, done).join('\n');
  const rest = [scene.views ?? [], scene.groups ?? []];
  const next: SceneMark = {
    strict: JSON.stringify([scene.objects, rest, done, trail]),
    loose: JSON.stringify([scene.objects.map((o) => ({ ...o, revision: 0 })), rest]),
    done,
    trail,
  };
  const was = seen;
  seen = next;
  empty = scene.objects.length === 0;
  if (settle) {
    const kept = firstSight ? keptSaved() : null;
    saved = kept && kept.strict !== next.strict ? kept : next;
    overwritten = saved !== next;
    settle = false;
    keepSaved();
  } else if (saved && was) {
    const sameList = steps.length === lastSteps.length && steps.every((s, i) => s === lastSteps[i]);
    if (sameList && done === lastDone) {
      if (next.strict !== was.strict) overwritten = true;
    } else if (sameList && done > lastDone && steps.length === done) {
      if (lastDone < saved.done) overwritten = true;
    } else if (!sameList) {
      let common = 0;
      while (common < steps.length && common < lastSteps.length && steps[common] === lastSteps[common]) common++;
      if (Math.min(common, done - 1) < saved.done) overwritten = true;
    }
  }
  firstSight = false;
  lastSteps = steps;
  lastDone = done;
}

export function sceneSettled(): void {
  epoch++;
  settle = true;
}

export function sceneSaved(): void {
  saved = seen;
  overwritten = false;
  keepSaved();
}

export function unsavedWork(): boolean {
  if (settle || !saved || !seen || empty) return false;
  if (seen.strict === saved.strict) return false;
  return overwritten || seen.done !== saved.done || seen.trail !== saved.trail || seen.loose !== saved.loose;
}

export function documentTitle(name: string): void {
  const label = document.querySelector('.bar-doc');
  if (label) setText(label, name);
  const title = name ? `${name} · FormSculpt` : 'FormSculpt';
  if (document.title === title) return;
  document.title = title;
  if (inShell()) void invoke('set_window_title', { title }).catch(() => undefined);
}

let lastField: HTMLElement | null = null;

function trackFields(): void {
  document.addEventListener('focusin', (e) => {
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (textFieldFocused()) lastField = document.activeElement as HTMLElement;
    else if (!target?.closest('.fs-menu, .fs-menu-btn')) lastField = null;
  });
}

const fieldReady = (action: TextEdit): boolean => textEditReady(action) || (!!lastField?.isConnected && !!document.activeElement?.closest('.fs-menu, .fs-menu-btn'));

async function editField(action: TextEdit): Promise<void> {
  if (!textEditReady(action) && lastField?.isConnected) lastField.focus();
  if (!textEditReady(action)) return;
  if (isMacDesktop()) {
    try {
      await invoke('text_edit', { action });
      return;
    } catch {
      document.execCommand(action === 'select-all' ? 'selectAll' : action);
      return;
    }
  }
  if (action === 'paste') {
    try {
      const text = await navigator.clipboard.readText();
      if (text) document.execCommand('insertText', false, text);
    } catch {
      document.execCommand('paste');
    }
    return;
  }
  document.execCommand(action === 'select-all' ? 'selectAll' : action);
}

const EDITS: ReadonlyArray<{ id: string; label: string; keys: string[] }> = [
  { id: 'edit.cut', label: 'Cut', keys: ['Mod+X'] },
  { id: 'edit.copy', label: 'Copy', keys: ['Mod+C'] },
  { id: 'edit.paste', label: 'Paste', keys: ['Mod+V'] },
  { id: 'edit.select_all', label: 'Select All', keys: ['Mod+A'] },
];

const EXIT_TEXT = { title: 'Save Changes?', description: 'This sculpt has changes that are not saved. Save them before quitting?' };
const REPLACE_TEXT = { description: 'This sculpt has changes that are not saved. Save them first?' };

type ExitChoice = 'save' | 'discard' | 'cancel';

class ExitDialog extends Dialog {
  private readonly lead = fromHtml<HTMLElement>('<p class="fs-note"></p>');
  private readonly discard: HTMLButtonElement;
  private readonly cancel: HTMLButtonElement;
  private readonly save: HTMLButtonElement;
  private done: ((choice: ExitChoice) => void) | null = null;

  constructor() {
    super(EXIT_TEXT.title, 460);
    this.body.append(this.lead);
    this.discard = this.button('', 'ghost', () => this.finish('discard'));
    this.cancel = this.button('Cancel', 'ghost', () => this.finish('cancel'));
    this.save = this.button('Save', 'primary', () => this.finish('save'));
    this.onClose = () => this.finish('cancel');
  }

  get asking(): boolean {
    return this.done !== null;
  }

  ask(description: string = EXIT_TEXT.description): Promise<ExitChoice> {
    setText(this.lead, t(description));
    setText(this.discard, t('Don\'t Save'));
    setText(this.cancel, t('Cancel'));
    setText(this.save, t('Save'));
    return new Promise((resolve) => {
      this.done = resolve;
      this.open(this.save);
    });
  }

  private finish(choice: ExitChoice): void {
    const fn = this.done;
    this.done = null;
    fn?.(choice);
    this.close();
  }
}

export interface ShellHooks {
  open: (paths: string[]) => Promise<void>;
  save: () => Promise<unknown>;
}

const confirmExit = (cancel = false): Promise<void> => invoke<void>('confirm_exit', cancel ? { cancel: true } : {});

function savedWithin(ms: number): Promise<boolean> {
  return new Promise((resolve) => {
    const until = performance.now() + ms;
    const look = (): void => {
      if (!unsavedWork()) resolve(true);
      else if (performance.now() > until) resolve(false);
      else window.setTimeout(look, 150);
    };
    look();
  });
}

export class DesktopShell {
  readonly native: NativeMenus | null;
  private hooks: ShellHooks | null = null;
  private exitDialog: ExitDialog | null = null;
  private leaving = false;

  constructor(private readonly commands: Commands, menus: () => MenuDef[], private readonly say: (text: string) => void) {
    trackFields();
    for (const { id, label, keys } of EDITS) {
      const action = TEXT_EDITS[id];
      commands.add({ id, label, keys, group: 'Edit', enabled: () => fieldReady(action), run: () => void editField(action) });
    }
    if (isMacDesktop()) {
      commands.add({
        id: 'view.menus_in_window',
        label: 'Menus in Window',
        group: 'View',
        checked: menusInWindow,
        run: () => {
          try {
            if (menusInWindow()) localStorage.removeItem(MENUS_IN_WINDOW_KEY);
            else localStorage.setItem(MENUS_IN_WINDOW_KEY, '1');
          } catch {
            return;
          }
          this.say(t('The menus move the next time formsculpt starts'));
        },
      });
    }
    if (inShell()) {
      commands.add({ id: 'app.exit', label: 'Exit', keys: isMac ? [] : ['Mod+Q'], group: 'File', run: () => void this.exitRequested() });
    }
    this.native = wantsNativeMenus() ? new NativeMenus(commands, menus) : null;
    if (this.native) {
      const native = this.native;
      document.documentElement.classList.add('has-native-menus');
      void native.build().catch(() => undefined);
      commands.onChange(() => native.refresh());
      document.addEventListener('change', () => native.refresh());
      onLanguage(() => void native.build(true).catch(() => undefined));
    }
  }

  refresh(): void {
    this.native?.refresh();
  }

  async listen(hooks: ShellHooks): Promise<void> {
    this.hooks = hooks;
    if (!inShell()) return;
    try {
      const events = await import('@tauri-apps/api/event');
      let opening: Promise<void> = Promise.resolve();
      const open = (paths: unknown): void => {
        const list = Array.isArray(paths) ? paths.filter((p): p is string => typeof p === 'string') : [];
        if (list.length) opening = opening.then(() => hooks.open(list)).catch(() => undefined);
      };
      await events.listen<string[]>('formsculpt://open-paths', (e) => open(e.payload));
      await events.listen('formsculpt://exit-requested', () => void this.exitRequested());
      open(await invoke<string[]>('take_pending_opens'));
    } catch {
      return;
    }
  }

  async settleUnsaved(): Promise<boolean> {
    if (!unsavedWork()) return true;
    if (this.leaving || this.exitDialog?.asking) return false;
    this.exitDialog ??= new ExitDialog();
    const choice = await this.exitDialog.ask(REPLACE_TEXT.description);
    if (choice === 'discard') return true;
    if (choice !== 'save' || !this.hooks) return false;
    await this.hooks.save();
    return savedWithin(2000);
  }

  private async exitRequested(): Promise<void> {
    if (this.leaving) return;
    if (this.exitDialog?.asking) {
      await confirmExit(true).catch(() => undefined);
      return;
    }
    this.leaving = true;
    try {
      if (!unsavedWork()) {
        await confirmExit();
        return;
      }
      this.exitDialog ??= new ExitDialog();
      const choice = await this.exitDialog.ask();
      if (choice === 'discard') {
        await confirmExit();
      } else if (choice === 'save' && this.hooks) {
        await this.hooks.save();
        await confirmExit(!(await savedWithin(15000)));
      } else {
        await confirmExit(true);
      }
    } catch {
      await confirmExit(true).catch(() => undefined);
    } finally {
      this.leaving = false;
    }
  }
}

let shell: DesktopShell | null = null;

export function startDesktopShell(commands: Commands, menus: () => MenuDef[], say: (text: string) => void): DesktopShell {
  shell ??= new DesktopShell(commands, menus, say);
  return shell;
}

export const desktopShell = (): DesktopShell | null => shell;

export const askUnsaved = (): Promise<boolean> => (shell ? shell.settleUnsaved() : Promise.resolve(true));
