const listeners = new Set<() => void>();

export function t(english: string): string {
  return english;
}

export function language(): string {
  return 'en';
}

export function onLanguage(fn: () => void): () => void {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}
