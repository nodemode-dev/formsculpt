const svg = (body: string): string =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;

export const ICONS = {
  mark: svg('<path d="M12 3c4.4 0 8 3.6 8 8 0 3-1.7 5.6-4.2 7l-.8 3H9l-.8-3C5.7 16.6 4 14 4 11c0-4.4 3.6-8 8-8z"/><path d="M9 11c1-1.5 2-2 3-2s2 .5 3 2"/>'),
} as const;

export type IconName = keyof typeof ICONS;
