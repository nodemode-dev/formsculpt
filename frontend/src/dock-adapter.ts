import type { DockAdapter, DockAgentStatus, DockChannel, DockKind, DockLegacy, DockTerminal } from './dock/adapter';
import { Dock } from './dock/dock';
import { api, ApiError } from './io/api';
import { events } from './io/events';
import { keyMessage } from './io/window-key';
import { ChatPane } from './ui/chat-pane';
import { commands } from './ui/commands';
import { fromHtml, isMac } from './ui/dom';
import { t } from './ui/i18n';
import { icon } from './ui/icons';
import { THEME_EVENT } from './ui/theme';

type Program = 'antigravity';
type Kind = DockKind | Program;

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

const LEAD = "Ask Claude about your model in the line below: block out a creature, add detail, retopologize, paint or export. It works through formsculpt's tools, every change shows here and in the viewport, and each one can be undone.";
const LEGACY_HEIGHT = 'formsculpt.dock_h';

const legacy: DockLegacy = {
  keys: [LEGACY_HEIGHT],
  read: (store) => {
    const h = Number(store.getItem(LEGACY_HEIGHT));
    return Number.isFinite(h) && h >= 140 ? { h } : null;
  },
};

const isDockChord = (e: KeyboardEvent): boolean => (isMac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey) && !e.shiftKey && !e.altKey && e.code === 'KeyJ';

function socketChannel(id: string): DockChannel {
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const socket = new WebSocket(`${proto}//${location.host}/api/terminal/${encodeURIComponent(id)}`);
  socket.binaryType = 'arraybuffer';
  socket.addEventListener('open', () => socket.send(keyMessage()), { once: true });
  let data =(_chunk: string | Uint8Array): void => undefined;
  let exit = (): void => undefined;
  let closed = (): void => undefined;
  socket.addEventListener('message', (msg) => {
    if (typeof msg.data === 'string') { data(msg.data); return; }
    try { if ((JSON.parse(new TextDecoder().decode(msg.data as ArrayBuffer)) as { type?: string }).type === 'exit') exit(); } catch { return; }
  });
  socket.addEventListener('close', () => closed());
  const send = (text: string): void => {
    if (socket.readyState === WebSocket.OPEN) socket.send(text);
    else if (socket.readyState === WebSocket.CONNECTING) socket.addEventListener('open', () => socket.send(text), { once: true });
  };
  return {
    send: (chunk) => send(JSON.stringify({ type: 'input', data: chunk })),
    resize: (cols, rows) => send(JSON.stringify({ type: 'resize', cols, rows })),
    onData: (handler) => { data = handler; },
    onExit: (handler) => { exit = handler; },
    onClose: (handler) => { closed = handler; },
    close: () => socket.close(),
  };
}

const oldEngine = (err: unknown): boolean => err instanceof ApiError && err.status === 404 && /^Unknown command/.test(err.message);

function adapter(host: DockHost, dock: () => Dock<Program>): DockAdapter<Program> {
  let pane: ChatPane | null = null;
  return {
    words: { app: 'formsculpt', noun: 'model', lead: LEAD, example: 'block out a dragon head', start: './formsculpt' },
    placement: {
      mode: 'own',
      storage: { key: 'formsculpt.dock', legacy },
      sizes: { minHeight: 140, maxHeightShare: 0.7, defaultHeight: 280, minWidth: 320, maxWidthShare: 0.5, defaultWidth: 440 },
      canDockRight: () => false,
    },
    isDockChord,
    chordLabel: () => `${isMac ? 'Cmd' : 'Ctrl'}+J`,
    tabs: () => ({ builtin: host.available() ? ['chat', 'shell', 'claude', 'codex', 'cursor', 'antigravity'] : [] }),
    classes: { iconButton: 'cv-tt-btn', button: 'cv-btn', primaryButton: 'cv-btn', input: 'cv-input', select: 'cv-input', note: 'cv-note' },
    t: (_key, english) => t(english),
    onTheme: (handler) => { window.addEventListener(THEME_EVENT, handler); },
    errors: {
      message: (err) => (err instanceof ApiError ? err.message : null),
      kind: (err) => {
        if (err instanceof TypeError) return 'unreachable';
        if (!(err instanceof ApiError)) return null;
        if (err.status === 403) return 'refused';
        if (oldEngine(err)) return 'oldEngine';
        if (err.status === 424) return 'notInstalled';
        if (err.status === 409) return 'busy';
        return err.status >= 502 && err.status <= 504 ? 'unreachable' : null;
      },
    },
    terminal: {
      list: async () => (await api.cmd<{ terminals: DockTerminal<Program>[] }>('terminal.list')).terminals,
      open: (kind, o) => {
        if (kind === 'shell') return api.cmd<DockTerminal<Program>>('terminal.open', { cols: o.cols, rows: o.rows });
        if (kind === 'claude') return api.cmd<DockTerminal<Program>>('claudecode.open', { cols: o.cols, rows: o.rows, ...(o.prompt ? { prompt: o.prompt } : {}), ...(o.resume ? { resume: o.resume } : {}) });
        return api.cmd<DockTerminal<Program>>('terminal.agent', { kind, cols: o.cols, rows: o.rows });
      },
      restart: async (id) => { await api.cmd('terminal.restart', { id }); },
      close: async (id) => { await api.cmd('terminal.close', { id }); },
      channel: (id) => socketChannel(id),
    },
    agentStatus: async () => {
      try {
        return await api.cmd<DockAgentStatus<Program>>('terminal.agents');
      } catch (err) {
        if (!oldEngine(err)) return null;
        const old = await api.cmd<{ installed: boolean; version: string | null }>('claudecode.status').catch(() => null);
        return old ? { claude: { installed: old.installed, version: old.version, install: '', signedIn: null } } : null;
      }
    },
    chat: {
      pane: (box) => {
        const chat = new ChatPane(api);
        chat.onFail = host.fail;
        chat.onObjects = host.select;
        chat.onHandoff = (session) => void dock().claude(undefined, session);
        chat.root.hidden = false;
        box.append(chat.root);
        pane = chat;
        return {
          send: (text) => {
            if (chat.isBusy()) return false;
            void chat.send(text, host.context(), host.note());
            return true;
          },
          shown: () => void chat.load(),
          busy: () => chat.isBusy(),
        };
      },
      ask: {
        send: (text: string, tab: Kind | null) => {
          if (tab === 'chat' && pane) {
            if (pane.isBusy()) return false;
            void pane.send(text, host.context(), host.note());
            return true;
          }
          const extra = host.context();
          return dock().claude(extra ? `${text} (${extra})` : text);
        },
      },
    },
  };
}

export function registerDock(host: DockHost): Dock<Program> {
  let made: Dock<Program> | null = null;
  const dock = new Dock<Program>(adapter(host, () => made as Dock<Program>));
  made = dock;
  dock.mount(host.root);

  host.tools.append(
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="agent.claude" aria-label="Claude Code">${icon('agent', 17)}</button>`),
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="agent.chat" aria-label="Chat with Claude">${icon('agentChat', 17)}</button>`),
    fromHtml(`<button type="button" class="cv-tt-btn" data-cmd="view.terminal" aria-label="Terminal">${icon('terminal', 17)}</button>`),
  );

  const why = (): boolean => host.available();
  commands.add(
    {
      id: 'view.terminal', label: () => (dock.isOpen() ? 'Hide Terminal' : 'Show Terminal'), keys: ['Mod+J'], group: 'Window', enabled: why, checked: () => dock.isOpen(),
      description: 'Shows or hides the terminal dock under the viewport; programs keep running while it is hidden.',
      run: () => dock.toggle(),
    },
    {
      id: 'view.newTerminal', label: 'New Terminal', group: 'Window', enabled: why,
      description: 'Opens a new shell tab in the terminal dock.',
      run: () => void dock.newTerminal(),
    },
    {
      id: 'agent.claude', label: 'Claude Code', keys: ['Shift+Mod+J'], group: 'Window', enabled: why,
      description: 'Starts Claude Code in the terminal dock, connected to formsculpt\'s tools, or brings its running tab forward.',
      run: () => void dock.show('claude'),
    },
    {
      id: 'agent.chat', label: 'Chat with Claude', keys: ['Alt+Mod+J'], group: 'Window', enabled: why,
      description: 'Opens the Chat tab, where Claude answers in plain language and works through formsculpt\'s tools.',
      run: () => void dock.show('chat'),
    },
  );

  if (host.available()) {
    events.on('agent.activity', (e) => host.activity(String(e.tool ?? '')));
    events.start();
  }
  return dock;
}
