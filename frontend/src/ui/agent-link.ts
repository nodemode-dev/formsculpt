import { el, setText } from './dom';

interface WindowState {
  epoch: number;
  changed: boolean;
  consent: { tool: string; asked: number } | null;
}

export class AgentLink {
  private epoch = -1;
  private dialog: HTMLDivElement;
  private detail: HTMLParagraphElement;
  private open = false;
  private busy = false;
  private current = 0;
  private answered = 0;
  onChange: (() => void) | null = null;
  shouldWait: () => boolean = () => false;

  constructor(overlay: HTMLElement) {
    this.dialog = el('div', 'consent');
    this.dialog.hidden = true;
    this.dialog.setAttribute('role', 'alertdialog');
    this.dialog.setAttribute('aria-modal', 'true');
    this.dialog.setAttribute('aria-labelledby', 'consent-title');
    const card = el('div', 'consent-card');
    const title = el('h2', '', 'Allow an outside agent to sculpt?');
    title.id = 'consent-title';
    this.detail = el('p', '', '');
    const row = el('div', 'consent-actions');
    const deny = el('button', 'btn', 'Decline');
    deny.type = 'button';
    const allow = el('button', 'btn primary', 'Allow');
    allow.type = 'button';
    deny.addEventListener('click', () => void this.answer(false));
    allow.addEventListener('click', () => void this.answer(true));
    row.append(deny, allow);
    card.append(title, this.detail, row);
    this.dialog.append(card);
    overlay.append(this.dialog);
  }

  start(): void {
    void this.tick();
    window.setInterval(() => void this.tick(), 900);
  }

  private async tick(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    try {
      const response = await fetch(`/api/mcp/window?since=${this.epoch}`, { cache: 'no-store' });
      if (!response.ok) return;
      const state = (await response.json()) as WindowState;
      if (state.changed && !this.shouldWait()) {
        const first = this.epoch === -1;
        this.epoch = state.epoch;
        if (!first) this.onChange?.();
      }
      this.show(state.consent);
    } catch {
      return;
    } finally {
      this.busy = false;
    }
  }

  private show(consent: WindowState['consent']): void {
    if (consent && consent.asked === this.answered) return;
    if (consent && !this.open) {
      this.current = consent.asked;
      setText(this.detail, `An agent connected over MCP wants to run ${consent.tool} on this scene. Every change it makes can be undone. Allow outside agents for this session?`);
      this.dialog.hidden = false;
      this.open = true;
    } else if (!consent && this.open) {
      this.dialog.hidden = true;
      this.open = false;
    }
  }

  private async answer(allow: boolean): Promise<void> {
    this.answered = this.current;
    this.dialog.hidden = true;
    this.open = false;
    await fetch('/api/mcp/consent', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ allow }) }).catch(() => undefined);
  }
}
