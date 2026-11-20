const HEADER = 'X-Formsculpt-Window-Key';
const STORE = 'formsculpt-window-key';

let key = '';
let ready: Promise<void> | null = null;
const plain = window.fetch.bind(window);

function remembered(): string {
  try { return sessionStorage.getItem(STORE) ?? ''; } catch { return ''; }
}

function remember(value: string): void {
  try { sessionStorage.setItem(STORE, value); } catch { return; }
}

function sameOrigin(input: RequestInfo | URL): boolean {
  const raw = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
  try { return new URL(raw, location.href).origin === location.origin; } catch { return false; }
}

async function accepted(): Promise<boolean> {
  try { return (await plain('/api/mcp/window', { headers: { [HEADER]: key }, cache: 'no-store' })).ok; } catch { return false; }
}

async function claim(): Promise<void> {
  const holder = window as unknown as { __FORMSCULPT_WINDOW_KEY?: unknown };
  const given = holder.__FORMSCULPT_WINDOW_KEY;
  delete holder.__FORMSCULPT_WINDOW_KEY;
  if (typeof given === 'string' && given) { key = given; return; }
  key = remembered();
  if (key && await accepted()) return;
  key = '';
  try {
    const res = await plain('/api/window/key', { method: 'POST' });
    const body = res.ok ? await res.json() as { key?: unknown } : {};
    if (typeof body.key === 'string' && body.key) { key = body.key; remember(key); return; }
  } catch { key = ''; }
  key = remembered();
}

async function keyedFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  if (!sameOrigin(input)) return plain(input, init);
  await ready;
  if (!key) return plain(input, init);
  const headers = new Headers(init?.headers ?? (input instanceof Request ? input.headers : undefined));
  headers.set(HEADER, key);
  return plain(input, { ...init, headers });
}

export function startWindowKey(): Promise<void> {
  if (!ready) {
    ready = claim();
    window.fetch = keyedFetch;
  }
  return ready;
}

export const windowKey = (): string => key;

export const keyMessage = (): string => JSON.stringify({ type: 'key', key });
