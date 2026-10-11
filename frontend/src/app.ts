import { VERSION } from './version';
import { ICONS } from './ui/icons';
import { el, setAttr, setText } from './ui/dom';
import { fetchVersion } from './engine/service';

interface Shell {
  engineDot: HTMLElement;
  engineText: HTMLElement;
  engineVersion: HTMLElement;
}

function buildShell(root: HTMLElement): Shell {
  const bar = el('header', 'appbar');
  const brand = el('div', 'brand');
  brand.innerHTML = ICONS.mark;
  brand.append(el('span', '', 'formsculptor'), el('span', 'ver', `v${VERSION}`));
  bar.append(brand, el('div', 'spacer'));

  const rail = el('nav', 'rail');
  rail.setAttribute('aria-label', 'Tools');

  const stage = el('main', 'stage');
  stage.append(el('p', 'stage-note', 'The sculpting viewport arrives in 0.2.0.'));

  const panel = el('aside', 'panel');
  panel.append(el('h2', '', 'Engine'));
  const kv = el('dl', 'kv');
  const engineVersion = el('dd', '', '…');
  kv.append(el('dt', '', 'App'), el('dd', '', VERSION), el('dt', '', 'Service'), engineVersion);
  panel.append(kv);

  const status = el('footer', 'status');
  const state = el('span', 'engine-state');
  const engineDot = el('i', 'dot');
  const engineText = el('span', '', 'connecting');
  state.append(engineDot, engineText);
  status.append(state);

  root.append(bar, rail, stage, panel, status);
  return { engineDot, engineText, engineVersion };
}

async function poll(shell: Shell): Promise<void> {
  const version = await fetchVersion();
  if (version) {
    setAttr(shell.engineDot, 'data-state', 'ok');
    setText(shell.engineText, 'service connected');
    setText(shell.engineVersion, version.version);
  } else {
    setAttr(shell.engineDot, 'data-state', 'down');
    setText(shell.engineText, 'service unreachable');
    setText(shell.engineVersion, 'offline');
  }
}

export function start(root: HTMLElement): void {
  const shell = buildShell(root);
  document.body.classList.add('app-ready');
  void poll(shell);
  window.setInterval(() => void poll(shell), 5000);
}
