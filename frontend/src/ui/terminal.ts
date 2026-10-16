import { api } from '../io/api';
import { events } from '../io/events';
import { commands } from './commands';
import { fromHtml } from './dom';
import { icon } from './icons';
import { TerminalDock } from './terminal-dock';

const HEIGHT_KEY = 'formsculptor.dock_h';
const MIN_H = 140;

export interface DockHost {
  root: HTMLElement;
  tools: HTMLElement;
  available: () => boolean;
  context: () => string;
  note: () => string;
  fail: (err: unknown) => void;
  activity: (tool: string) => void;
  select: (ids: number[]) => void;
}

function readHeight(): number | null {
  try {
    const v = Number(localStorage.getItem(HEIGHT_KEY));
    return Number.isFinite(v) && v >= MIN_H ? v : null;
  } catch {
    return null;
  }
}

function resizer(root: HTMLElement, section: HTMLElement, onDone: () => void): void {
  const handle = section.querySelector<HTMLElement>('.fl-dock-rsz');
  if (!handle) return;
  handle.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    handle.classList.add('is-dragging');
    const startY = e.clientY;
    const startH = section.getBoundingClientRect().height;
    let last = startH;
    const move = (ev: PointerEvent): void => {
      last = Math.round(Math.max(MIN_H, Math.min(window.innerHeight * 0.7, startH + (startY - ev.clientY))));
      root.style.setProperty('--dock-open-h', `${last}px`);
    };
    const up = (): void => {
      handle.classList.remove('is-dragging');
      handle.removeEventListener('pointermove', move);
      handle.removeEventListener('pointerup', up);
      handle.removeEventListener('pointercancel', up);
      try { localStorage.setItem(HEIGHT_KEY, String(last)); } catch { /* storage blocked */ }
      onDone();
    };
    handle.addEventListener('pointermove', move);
    handle.addEventListener('pointerup', up);
    handle.addEventListener('pointercancel', up);
  });
}

export function registerTerminal(host: DockHost): void {
  const section = fromHtml('<section class="fl-dock" aria-label="Terminal" hidden></section>');
  host.root.append(section);
  const saved = readHeight();
  if (saved) host.root.style.setProperty('--dock-open-h', `${saved}px`);
  let dock: TerminalDock | null = null;
  const open = (): boolean => !section.hidden;
  const setDock = (on: boolean): void => {
    if (open() === on) return;
    section.hidden = !on;
    host.root.classList.toggle('is-dock-open', on);
    if (on) requestAnimationFrame(() => dock?.fitActive());
  };
  const ensure = (): TerminalDock => {
    if (!dock) {
      dock = new TerminalDock(section, api, () => setDock(false));
      dock.onFail = host.fail;
      dock.context = host.context;
      dock.note = host.note;
      dock.onObjects = host.select;
      resizer(host.root, section, () => dock?.fitActive());
    }
    return dock;
  };
  const show = async (then?: (d: TerminalDock) => Promise<void>): Promise<void> => {
    const d = ensure();
    const first = !open();
    setDock(true);
    if (then) await then(d);
    else if (first) await d.ensure();
    else d.focus();
  };

  host.tools.append(
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="agent.claude" aria-label="Claude Code">${icon('agent', 17)}</button>`),
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="agent.chat" aria-label="Chat with Claude">${icon('agentChat', 17)}</button>`),
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="view.terminal" aria-label="Terminal">${icon('terminal', 17)}</button>`),
  );

  const why = (): boolean => host.available();
  commands.add(
    {
      id: 'view.terminal', label: () => (open() ? 'Hide Terminal' : 'Show Terminal'), keys: ['Mod+J'], group: 'Window', enabled: why, checked: open,
      description: 'Shows or hides the terminal dock under the viewport; programs keep running while it is hidden.',
      run: () => { if (open()) setDock(false); else void show(); },
    },
    {
      id: 'view.newTerminal', label: 'New Terminal', group: 'Window', enabled: why,
      description: 'Opens a new shell tab in the terminal dock.',
      run: () => void show((d) => d.open()),
    },
    {
      id: 'agent.claude', label: 'Claude Code', keys: ['Shift+Mod+J'], group: 'Window', enabled: why,
      description: 'Starts Claude Code in the terminal dock, connected to formsculptor\'s tools, or brings its running tab forward.',
      run: () => void show((d) => d.claude()),
    },
    {
      id: 'agent.chat', label: 'Chat with Claude', keys: ['Alt+Mod+J'], group: 'Window', enabled: why,
      description: 'Opens the Chat tab, where Claude answers in plain language and works through formsculptor\'s tools.',
      run: () => void show((d) => d.showChat()),
    },
  );

  if (host.available()) {
    events.on('agent.activity', (e) => host.activity(String(e.tool ?? '')));
    events.start();
  }
}
