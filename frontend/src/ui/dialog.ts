import { fromHtml, q, setText } from './dom';
import { icon } from './icons';
import { t } from './i18n';

let openCount = 0;

export const anyDialogOpen = (): boolean => openCount > 0;

export function overlayRoot(): HTMLElement {
  return document.getElementById('overlay-root') ?? document.body;
}

export class Dialog {
  readonly scrim: HTMLElement;
  readonly body: HTMLElement;
  readonly foot: HTMLElement;
  private readonly heading: HTMLElement;
  private english = '';
  private lastFocus: HTMLElement | null = null;
  onClose: (() => void) | null = null;

  constructor(title: string, width = 440) {
    this.scrim = fromHtml<HTMLElement>(
      `<div class="fs-scrim" hidden><div class="fs-dialog" role="dialog" aria-modal="true" tabindex="-1">` +
        `<div class="fs-dialog-head"><span class="fs-dialog-title"></span>` +
        `<button class="fs-icon-btn" type="button" data-close aria-label="Close">${icon('close', 16)}</button></div>` +
        `<div class="fs-dialog-body"></div><div class="fs-dialog-foot"></div></div></div>`,
    );
    q<HTMLElement>(this.scrim, '.fs-dialog').style.setProperty('--fs-dialog-w', `${width}px`);
    this.heading = q(this.scrim, '.fs-dialog-title');
    this.body = q(this.scrim, '.fs-dialog-body');
    this.foot = q(this.scrim, '.fs-dialog-foot');
    this.english = title;
    setText(this.heading, t(title));
    q(this.scrim, '[data-close]').title = t('Close');
    q(this.scrim, '[data-close]').addEventListener('click', () => this.close());
    this.scrim.addEventListener('pointerdown', (e) => {
      if (e.target === this.scrim) this.close();
    });
    this.scrim.addEventListener('keydown', (e) => {
      e.stopPropagation();
      if (e.key === 'Escape') {
        e.preventDefault();
        this.close();
      }
    });
    overlayRoot().appendChild(this.scrim);
  }

  get isOpen(): boolean {
    return !this.scrim.hidden;
  }

  setTitle(title: string): void {
    this.english = title;
    setText(this.heading, t(title));
  }

  open(focus?: HTMLElement | null): void {
    if (this.isOpen) return;
    setText(this.heading, t(this.english));
    this.lastFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    this.scrim.hidden = false;
    openCount++;
    const target = focus ?? this.scrim.querySelector<HTMLElement>('input, select, .fs-btn-primary:not(:disabled)');
    target?.focus();
  }

  close(): void {
    if (!this.isOpen) return;
    this.scrim.hidden = true;
    openCount = Math.max(0, openCount - 1);
    this.lastFocus?.focus();
    this.lastFocus = null;
    this.onClose?.();
  }

  button(label: string, kind: '' | 'primary' | 'ghost', run: () => void): HTMLButtonElement {
    const b = fromHtml<HTMLButtonElement>(`<button type="button" class="fs-btn${kind ? ` fs-btn-${kind}` : ''}"></button>`);
    setText(b, t(label));
    b.addEventListener('click', run);
    this.foot.appendChild(b);
    return b;
  }
}

export const overlayOpen = (): boolean => anyDialogOpen() || !!overlayRoot().querySelector('.fs-menu:not([hidden])');
