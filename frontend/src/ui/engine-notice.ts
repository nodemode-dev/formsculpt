import { askUnsaved } from './desktop-shell';
import { fromHtml, q, setDisabled } from './dom';
import { icon } from './icons';
import { t } from './i18n';

const NOTICE = {
  description: 'formsculpt is running on the browser engine because the engine service did not answer. Sculpting works here; the library, Recover, plugins, hand-offs and agents need the service.',
  title: 'The engine service still does not answer.',
};
const LEAD = NOTICE.description;
const STILL = NOTICE.title;

async function serviceAnswers(): Promise<boolean> {
  try {
    return (await fetch('/api/version', { cache: 'no-store' })).ok;
  } catch {
    return false;
  }
}

export function showEngineNotice(): void {
  const node = fromHtml(`<div class="fs-toast fs-engine-notice" role="status"><p class="fs-engine-notice-text"></p><div class="fs-engine-notice-actions"><button class="fs-btn fs-btn-sm" type="button" data-a="retry"><span>Retry</span></button><button class="fs-icon-btn" type="button" data-a="dismiss" aria-label="Close" title="Close">${icon('close', 14)}</button></div></div>`);
  const text = q(node, '.fs-engine-notice-text');
  const retry = q<HTMLButtonElement>(node, '[data-a="retry"]');
  text.textContent = t(LEAD);
  q(node, '[data-a="dismiss"]').addEventListener('click', () => node.remove());
  retry.addEventListener('click', () => {
    setDisabled(retry, true);
    void (async () => {
      if (!(await serviceAnswers())) {
        text.textContent = `${t(LEAD)} ${t(STILL)}`;
      } else if (await askUnsaved()) {
        location.reload();
        return;
      }
      setDisabled(retry, false);
    })();
  });
  document.body.appendChild(node);
}
