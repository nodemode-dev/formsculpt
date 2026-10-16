import './styles/fonts.css';
import './styles/tokens.css';
import './styles/shell.css';
import './styles/menu.css';
import { start } from './app';

const root = document.getElementById('app');
if (root) start(root);
