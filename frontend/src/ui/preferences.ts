import type { Commands } from './commands';
import { Dialog } from './dialog';
import { fromHtml, q, setHidden } from './dom';
import { language, languages, onLanguage, setLanguage } from './i18n';
import { isLang } from './i18n/langs';

const THEMES = ['nodemode', 'family', 'dark', 'light', 'contrast', 'amoled'];

export class PreferencesDialog extends Dialog {
  private readonly theme: HTMLSelectElement;
  private readonly layout: HTMLSelectElement;
  private readonly lang: HTMLSelectElement;
  private readonly menus: HTMLInputElement;
  private readonly launch: HTMLInputElement;
  private readonly lastClose: HTMLInputElement;

  constructor(private readonly commands: Commands) {
    super('Preferences', 460);
    const body = fromHtml(`<div class="fs-prefs">
<label class="fs-pref-row"><span>Theme</span><select class="fs-input" data-k="theme"><option value="nodemode">Follow NodeMode</option><option value="family">Form family theme</option><option value="dark">Dark theme</option><option value="light">Light theme</option><option value="contrast">High contrast</option><option value="amoled">AMOLED Black</option></select></label>
<label class="fs-pref-row"><span>Layout</span><select class="fs-input" data-k="layout"><option value="panel">Panel layout</option><option value="tablet">Tablet Layout</option><option value="zbrush">ZUI Layout</option></select></label>
<label class="fs-pref-row"><span>Language</span><select class="fs-input" data-k="lang" data-no-i18n></select></label>
<label class="fs-home-check" data-k="menus-row"><input type="checkbox" data-k="menus"><span>Menus in Window</span></label>
<label class="fs-home-check"><input type="checkbox" data-k="launch"><span>Show Home Screen at Launch</span></label>
<label class="fs-home-check"><input type="checkbox" data-k="last-close"><span>Show Home Screen When the Last Document Closes</span></label>
</div>`);
    this.theme = q(body, '[data-k="theme"]');
    this.layout = q(body, '[data-k="layout"]');
    this.lang = q(body, '[data-k="lang"]');
    this.menus = q(body, '[data-k="menus"]');
    this.launch = q(body, '[data-k="launch"]');
    this.lastClose = q(body, '[data-k="last-close"]');
    for (const [code, name] of languages()) {
      const o = document.createElement('option');
      o.value = code;
      o.textContent = name;
      this.lang.appendChild(o);
    }
    this.body.appendChild(body);
    this.theme.addEventListener('change', () => commands.run(`view.theme_${this.theme.value}`));
    this.layout.addEventListener('change', () => this.useLayout(this.layout.value));
    this.lang.addEventListener('change', () => {
      if (isLang(this.lang.value)) void setLanguage(this.lang.value);
    });
    this.menus.addEventListener('change', () => this.flip('view.menus_in_window', this.menus));
    this.launch.addEventListener('change', () => this.flip('pref.home_launch', this.launch));
    this.lastClose.addEventListener('change', () => this.flip('pref.home_last_close', this.lastClose));
    const keys = this.button('Keyboard Shortcuts', 'ghost', () => {
      this.close();
      commands.run('app.shortcuts');
    });
    keys.classList.add('fs-btn-apart');
    this.button('Close', 'primary', () => this.close());
    onLanguage(() => {
      if (this.isOpen) this.sync();
    });
  }

  private checked(id: string): boolean {
    return !!this.commands.get(id)?.checked?.();
  }

  private flip(id: string, box: HTMLInputElement): void {
    if (this.checked(id) !== box.checked) this.commands.run(id);
    box.checked = this.checked(id);
  }

  private current(): string {
    return this.checked('view.zbrush_layout') ? 'zbrush' : this.checked('view.tablet_layout') ? 'tablet' : 'panel';
  }

  private useLayout(want: string): void {
    if (this.current() === want) return;
    if (this.checked('view.zbrush_layout')) this.commands.run('view.zbrush_layout');
    if (want === 'zbrush') this.commands.run('view.zbrush_layout');
    else if ((want === 'tablet') !== this.checked('view.tablet_layout')) this.commands.run('view.tablet_layout');
    this.layout.value = this.current();
  }

  private sync(): void {
    const theme = THEMES.find((n) => this.checked(`view.theme_${n}`)) ?? 'nodemode';
    if (this.theme.value !== theme) this.theme.value = theme;
    if (this.layout.value !== this.current()) this.layout.value = this.current();
    if (this.lang.value !== language()) this.lang.value = language();
    const menus = !!this.commands.get('view.menus_in_window');
    setHidden(q(this.body, '[data-k="menus-row"]'), !menus);
    this.menus.checked = menus && this.checked('view.menus_in_window');
    this.launch.checked = this.checked('pref.home_launch');
    this.lastClose.checked = this.checked('pref.home_last_close');
  }

  show(): void {
    this.sync();
    this.open(this.theme);
  }
}
