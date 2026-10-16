const svg = (body: string): string =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const ICONS = {
  mark: svg('<path d="M12 3c4.4 0 8 3.6 8 8 0 3-1.7 5.6-4.2 7l-.8 3H9l-.8-3C5.7 16.6 4 14 4 11c0-4.4 3.6-8 8-8z"/><path d="M9 11c1-1.5 2-2 3-2s2 .5 3 2"/>'),
} as const;

export type IconName = keyof typeof ICONS;

const GLYPHS: Record<string, string> = {
  check: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  chevronRight: '<path d="M9 6l6 6-6 6"/>',
  close: '<path d="M6 6l12 12M18 6L6 18"/>',
  search: '<circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/>',
  undo: '<path d="M9 7L4.5 11.5 9 16"/><path d="M5 11.5h9a5 5 0 0 1 0 10h-3"/>',
  redo: '<path d="M15 7l4.5 4.5L15 16"/><path d="M19 11.5h-9a5 5 0 0 0 0 10h3"/>',
  open: '<path d="M3.5 7.5a2 2 0 0 1 2-2H10l2 2.5h6.5a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  save: '<path d="M5 4h11l3.5 3.5V19a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1z"/><path d="M8 4v5h7V4M8 20v-6h8v6"/>',
  frame: '<path d="M4 9V5h4M16 5h4v4M20 15v4h-4M8 19H4v-4"/><circle cx="12" cy="12" r="2.5"/>',
  cube: '<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5"/>',
  brush: '<path d="M14.5 4.5l5 5L10 19l-5 .5.5-5z"/><path d="M12.5 6.5l5 5"/>',
  mask: '<circle cx="12" cy="12" r="8"/><path d="M12 4a8 8 0 0 1 0 16z" fill="currentColor"/>',
  trash: '<path d="M5 7h14M10 7V4.5h4V7M7 7l1 13h8l1-13"/>',
  copy: '<rect x="8.5" y="8.5" width="11" height="11" rx="1.5"/><path d="M15.5 8.5V5.5a1 1 0 0 0-1-1h-9a1 1 0 0 0-1 1v9a1 1 0 0 0 1 1h3"/>',
  eye: '<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="2.8"/>',
  keyboard: '<rect x="3" y="6.5" width="18" height="11" rx="1.5"/><path d="M7 10.5h.01M10.5 10.5h.01M14 10.5h.01M17.5 10.5h.01M8 14h8"/>',
  command: '<path d="M9 9V6.5A2.5 2.5 0 1 0 6.5 9H9zm0 0h6m-6 0v6m6-6V6.5A2.5 2.5 0 1 1 17.5 9H15zm0 0v6m-6 0H6.5A2.5 2.5 0 1 0 9 17.5V15zm6 0h2.5a2.5 2.5 0 1 1-2.5 2.5V15z"/>',
  info: '<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8v.01"/>',
  plus: '<path d="M12 6v12M6 12h12"/>',
  refresh: '<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3"/><path d="M19.5 4.5v4h-4"/>',
  send: '<path d="M4 12l16-7.5-6 16-2.5-6.5z"/><path d="M11.5 14l8.5-9.5"/>',
  agent: '<rect x="4" y="7.5" width="16" height="12" rx="4"/><path d="M12 7.5V4.5"/><circle cx="12" cy="3.6" r="1.1"/><circle cx="9.3" cy="13.3" r="1.35" fill="currentColor" stroke="none"/><circle cx="14.7" cy="13.3" r="1.35" fill="currentColor" stroke="none"/><path d="M2 12.2v2.8M22 12.2v2.8"/>',
  agentChat: '<path d="M6 4.5h12a3.5 3.5 0 0 1 3.5 3.5v6a3.5 3.5 0 0 1-3.5 3.5h-6.5L7 21v-3.5H6A3.5 3.5 0 0 1 2.5 14V8A3.5 3.5 0 0 1 6 4.5z"/><circle cx="9.2" cy="11" r="1.3" fill="currentColor" stroke="none"/><circle cx="14.8" cy="11" r="1.3" fill="currentColor" stroke="none"/>',
  terminal: '<rect x="3" y="4.5" width="18" height="15" rx="2"/><path d="M7 9.5l3 2.5-3 2.5M12.5 15h4.5"/>',
};

export function icon(name: string, size = 16): string {
  const body = GLYPHS[name] ?? '';
  return `<svg width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
}
