import './styles/fonts.css';
import './styles/tokens.css';
import './styles/shell.css';
import './styles/tablet.css';
import './styles/zbrush.css';
import './styles/menu.css';
import './styles/dock-tokens.css';
import './dock/dock.css';
import './styles/chat.css';
import { start } from './app';
import { startLanguage, startPageTranslator } from './ui/i18n';
import { startWindowKey } from './io/window-key';

const root = document.getElementById('app');
if (root) {
  void startWindowKey().then(startLanguage).finally(() => {
    start(root);
    startPageTranslator();
  });
}
