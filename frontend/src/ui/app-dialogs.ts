import { VERSION } from '../version';
import { ICONS } from './icons';
import { comboOf, formatKeys, type Commands } from './commands';
import { Dialog } from './dialog';
import { fromHtml, patchList, q, setDisabled, setText } from './dom';
import { t } from './i18n';
import { persistShortcuts } from './prefs';

export class ShortcutsDialog extends Dialog {
  private readonly search = fromHtml<HTMLInputElement>(`<input class="fs-input" type="search" placeholder="${t('Search commands')}" aria-label="${t('Search shortcuts')}" spellcheck="false" autocomplete="off">`);
  private readonly list = fromHtml('<div class="fs-list fs-keys-list" aria-label="Keyboard shortcuts"></div>');
  private readonly note = fromHtml('<div class="fs-shortcuts-note" role="status"></div>');
  private capturing: string | null = null;
  private pending: { id: string; combo: string } | null = null;
  private focusId: string | null = null;

  constructor(private readonly commands: Commands) {
    super('Keyboard Shortcuts', 640);
    this.body.append(this.search, this.note, this.list);
    this.search.addEventListener('input', () => this.render());
    this.button('Reset All', 'ghost', () => {
      this.commands.resetKeys();
      this.save();
      this.say('');
      this.render();
    });
    this.button('Close', 'primary', () => this.close());
    window.addEventListener('keydown', (e) => this.capture(e), true);
  }

  show(): void {
    this.capturing = null;
    this.pending = null;
    this.focusId = null;
    this.say('');
    this.search.value = '';
    this.render();
    this.open(this.search);
  }

  private save(): void {
    persistShortcuts(this.commands);
  }

  private say(text: string, actions: HTMLElement[] = []): void {
    const first = this.note.firstElementChild as HTMLElement | null;
    if (!text) {
      if (first) this.note.replaceChildren();
      return;
    }
    const span = first ?? fromHtml('<span></span>');
    setText(span, text);
    this.note.replaceChildren(span, ...actions);
  }

  private capture(e: KeyboardEvent): void {
    if (!this.capturing || !this.isOpen) return;
    e.preventDefault();
    e.stopPropagation();
    const combo = comboOf(e);
    if (!combo) return;
    const id = this.capturing;
    this.capturing = null;
    if (e.key === 'Escape') {
      this.say('');
      this.render();
      return;
    }
    const other = this.commands.forCombo(combo);
    if (other && other.id !== id) {
      this.pending = { id, combo };
      const re = fromHtml<HTMLButtonElement>(`<button type="button" class="fs-btn fs-btn-ghost">${t('Reassign')}</button>`);
      const no = fromHtml<HTMLButtonElement>(`<button type="button" class="fs-btn fs-btn-ghost">${t('Cancel')}</button>`);
      re.addEventListener('click', () => {
        const p = this.pending;
        this.pending = null;
        if (p) {
          this.commands.setKeys(p.id, [p.combo]);
          this.save();
        }
        this.say('');
        this.render();
      });
      no.addEventListener('click', () => {
        this.pending = null;
        this.say('');
        this.render();
      });
      this.say(`${formatKeys(combo)} ${t('is used by')} ${t(this.commands.label(other))}. `, [re, no]);
      return;
    }
    this.commands.setKeys(id, [combo]);
    this.save();
    this.say('');
    this.render();
  }

  private render(): void {
    const needle = this.search.value.trim().toLowerCase();
    const rows = this.commands
      .all()
      .map((c) => ({ id: c.id, label: t(this.commands.label(c)).replace(/\.\.\.$/, ''), keys: (c.keys ?? []).map(formatKeys).join(', ') }))
      .filter((r) => r.label && (!needle || r.label.toLowerCase().includes(needle) || r.keys.toLowerCase().includes(needle) || r.id.includes(needle)))
      .sort((a, b) => a.label.localeCompare(b.label));
    patchList(this.list, rows, {
      key: (r) => r.id,
      create: (r) => {
        const n = fromHtml(`<div class="fs-list-row fs-keys-row"><span class="fs-list-name"></span><span class="fs-list-meta"></span><button type="button" class="fs-btn fs-btn-ghost">${t('Change')}</button><button type="button" class="fs-btn fs-btn-ghost">${t('Remove')}</button></div>`);
        n.dataset.command = r.id;
        const [change, remove] = [...n.querySelectorAll('button')] as HTMLButtonElement[];
        change.addEventListener('click', () => {
          this.capturing = r.id;
          this.focusId = r.id;
          this.pending = null;
          this.say('');
          this.render();
        });
        remove.addEventListener('click', () => {
          this.focusId = r.id;
          this.commands.setKeys(r.id, []);
          this.save();
          this.render();
        });
        this.fillRow(n, r);
        return n;
      },
      update: (n, r) => this.fillRow(n, r),
    });
    const btn = this.focusId ? (this.list.querySelector(`.fs-keys-row[data-command="${CSS.escape(this.focusId)}"] button`) as HTMLButtonElement | null) : null;
    if (btn && !btn.parentElement?.contains(document.activeElement)) btn.focus();
  }

  private fillRow(n: HTMLElement, r: { id: string; label: string; keys: string }): void {
    setText(q(n, '.fs-list-name'), r.label);
    setText(q(n, '.fs-list-meta'), this.capturing === r.id ? t('Press the new keys (Esc cancels)') : r.keys || t('none'));
    const [change, remove] = [...n.querySelectorAll('button')] as HTMLButtonElement[];
    if (change.getAttribute('aria-label') !== `${t('Change')} ${r.label}`) {
      change.setAttribute('aria-label', `${t('Change')} ${r.label}`);
      remove.setAttribute('aria-label', `${t('Remove')} ${r.label}`);
    }
    setDisabled(remove, !r.keys);
  }
}

export class AboutDialog extends Dialog {
  constructor() {
    super('About formsculptor', 420);
    this.body.appendChild(fromHtml(`<div class="fs-about-title">${ICONS.mark}<span>formsculptor ${VERSION}</span></div>`));
    this.body.appendChild(fromHtml(`<p class="fs-note">${t('3D sculpting and modeling in the NodeMode design system.')}</p>`));
    this.body.appendChild(fromHtml(`<p class="fs-note">${t('Apache License 2.0. Copyright 2026 nodemode-ai.')}</p>`));
    this.button('Close', 'primary', () => this.close());
  }

  show(): void {
    this.open();
  }
}

export class PromptDialog extends Dialog {
  private readonly field = fromHtml<HTMLInputElement>('<input class="fs-input" type="text" spellcheck="false" autocomplete="off">');
  private done: ((value: string | null) => void) | null = null;

  constructor() {
    super('Name', 380);
    this.body.append(this.field);
    this.button('Cancel', 'ghost', () => this.finish(null));
    this.button('OK', 'primary', () => this.finish(this.field.value.trim() || null));
    this.field.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        this.finish(this.field.value.trim() || null);
      }
    });
    this.onClose = () => this.finish(null);
  }

  ask(title: string, value: string): Promise<string | null> {
    this.setTitle(title);
    this.field.value = value;
    return new Promise((resolve) => {
      this.done = resolve;
      this.open(this.field);
      this.field.select();
    });
  }

  private finish(value: string | null): void {
    const fn = this.done;
    this.done = null;
    fn?.(value);
    this.close();
  }
}
