import { commands } from './commands';
import { setAttr, setDisabled, setTitle } from './dom';
import { onLanguage, t } from './i18n';

let frame = 0;
onLanguage(() => scheduleChrome());

export function syncChrome(root: ParentNode = document): void {
  for (const node of Array.from(root.querySelectorAll<HTMLElement>('[data-cmd]'))) {
    const cmd = commands.get(node.dataset.cmd ?? '');
    if (!cmd) continue;
    const label = t(commands.label(cmd));
    const why = t(commands.why(cmd));
    const keys = commands.keyLabel(cmd);
    const on = commands.enabled(cmd);
    setTitle(node, !on && why ? why : keys ? `${label} (${keys})` : label);
    if (node instanceof HTMLButtonElement) setDisabled(node, !on);
    if (cmd.checked) setAttr(node, 'aria-pressed', cmd.checked() ? 'true' : 'false');
  }
}

export function scheduleChrome(): void {
  if (frame) return;
  frame = requestAnimationFrame(() => {
    frame = 0;
    syncChrome();
  });
}
