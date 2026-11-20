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

const root = document.getElementById('app');
if (root) {
  void startLanguage().finally(() => {
    start(root);
    startPageTranslator();
  });
}
