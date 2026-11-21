/**
 * The interface theme: follow the NodeMode setting (the default, as before), or dark, light, high contrast or AMOLED Black here, and
 * translucent panels (the viewport runs under the panel and rail, which turn see-through with a blur; menus, dialogs and
 * floating toolbars too). Kept in this browser and read before first paint by index.html, so nothing flashes.
 */

import { applyFamilyColours, familyBackground, familyThemeId } from './family-themes';

export const THEME_STORE = 'formsculpt.theme';
const NODEMODE_STORE = 'nodemode_ui_settings';

export type ThemeName = 'nodemode' | 'family' | 'dark' | 'light' | 'contrast' | 'amoled';
export interface ThemePrefs {
  theme: ThemeName;
  translucent: boolean;
}

const NAMES: ThemeName[] = ['nodemode', 'family', 'dark', 'light', 'contrast', 'amoled'];

export function parseTheme(v: unknown): ThemePrefs {
  const r = v && typeof v === 'object' ? (v as Record<string, unknown>) : {};
  return { theme: NAMES.includes(r.theme as ThemeName) ? (r.theme as ThemeName) : 'nodemode', translucent: r.translucent === true };
}

export function loadTheme(): ThemePrefs {
  try {
    return parseTheme(JSON.parse(localStorage.getItem(THEME_STORE) ?? 'null'));
  } catch {
    return parseTheme(null);
  }
}

export function saveTheme(p: ThemePrefs): void {
  try {
    localStorage.setItem(THEME_STORE, JSON.stringify(p));
  } catch {
    // storage blocked; the theme lasts for this session
  }
}

/** Whether NodeMode's own setting is a light theme. */
function nodemodeLight(): boolean {
  try {
    const s = JSON.parse(localStorage.getItem(NODEMODE_STORE) ?? 'null') as { theme?: string } | null;
    return s?.theme === 'light' || s?.theme === 'sand';
  } catch {
    return false;
  }
}

/** Applies a theme to the page: the body's theme classes and the page background behind it. */
export function applyTheme(p: ThemePrefs): void {
  const profile = p.theme === 'family' ? familyThemeId() : p.theme === 'amoled' ? 'amoled' : null;
  const familyLight = applyFamilyColours(profile);
  const light = p.theme === 'light' || familyLight || (p.theme === 'nodemode' && nodemodeLight());
  const body = document.body;
  body.classList.toggle('theme-light', light);
  body.classList.toggle('theme-contrast', p.theme === 'contrast');
  body.classList.toggle('panels-translucent', p.translucent);
  document.documentElement.style.background = profile ? familyBackground(profile) : light ? '#ecebe6' : p.theme === 'contrast' ? '#000000' : '';
  window.dispatchEvent(new Event(THEME_EVENT));
}

export const THEME_EVENT = 'formsculpt-theme';
