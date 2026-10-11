export interface Health {
  ok: boolean;
}

export interface Version {
  app: string;
  version: string;
}

export async function fetchVersion(signal?: AbortSignal): Promise<Version | null> {
  try {
    const response = await fetch('/api/version', { signal, cache: 'no-store' });
    if (!response.ok) return null;
    return (await response.json()) as Version;
  } catch {
    return null;
  }
}
