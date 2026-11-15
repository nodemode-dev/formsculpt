import { browserEngine } from './service';

export interface SculptSummary {
  id: string;
  title: string;
  objects: number | null;
  triangles: number | null;
  bytes: number;
  modified: string;
  opened: string | null;
  template: string | null;
  thumb: boolean;
  thumbAt: number;
}

export interface TrashedSculpt {
  entry: string;
  id: string;
  title: string;
  objects: number | null;
  deletedAt: number;
  thumb: boolean;
}

export interface TemplateSummary {
  id: string;
  name: string;
  category: string;
  tags: string[];
  description: string;
  source: string;
  thumb: boolean;
}

export interface TemplateList {
  categories: string[];
  templates: TemplateSummary[];
  default: string;
}

export interface HomeSettings {
  pinned: string[];
  recentHidden: string[];
}

async function call<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  if (browserEngine()) throw new Error('the library needs the engine service; this window is running the browser engine');
  const init: RequestInit = { method, cache: 'no-store' };
  if (body instanceof Blob) init.body = body;
  else if (body !== undefined) {
    init.body = JSON.stringify(body);
    init.headers = { 'content-type': 'application/json' };
  }
  const response = await fetch(path, init);
  const text = await response.text();
  if (!response.ok) {
    let detail = text || response.statusText;
    try {
      detail = String((JSON.parse(text) as { detail?: unknown }).detail ?? detail);
    } catch {
      detail = text || response.statusText;
    }
    throw new Error(detail);
  }
  return JSON.parse(text) as T;
}

const at = (id: string): string => `/api/sculpts/${encodeURIComponent(id)}`;

export const library = {
  list: (): Promise<{ sculpts: SculptSummary[] }> => call('/api/sculpts'),
  create: (template: string, title?: string): Promise<SculptSummary> => call('/api/sculpts', 'POST', { template, title }),
  keepScene: (title: string, ownThumb: boolean): Promise<SculptSummary> => call('/api/sculpts', 'POST', { title, fromScene: true, thumb: ownThumb ? 'client' : 'render' }),
  open: (id: string): Promise<{ ok: boolean; sculpt: SculptSummary }> => call(`${at(id)}/open`, 'POST'),
  save: (id: string, ownThumb: boolean): Promise<SculptSummary> => call(`${at(id)}/save?thumb=${ownThumb ? 'client' : 'render'}`, 'POST'),
  duplicate: (id: string): Promise<SculptSummary> => call(`${at(id)}/duplicate`, 'POST'),
  remove: (id: string): Promise<{ ok: boolean; entry: string }> => call(at(id), 'DELETE'),
  putThumb: (id: string, picture: Blob): Promise<{ ok: boolean }> => call(`${at(id)}/thumb`, 'PUT', picture),
  thumbUrl: (d: SculptSummary): string => `${at(d.id)}/thumb?v=${d.thumbAt}`,
  trash: (): Promise<{ sculpts: TrashedSculpt[] }> => call('/api/sculpt-trash'),
  restore: (entry: string): Promise<SculptSummary> => call(`/api/sculpt-trash/${encodeURIComponent(entry)}/restore`, 'POST'),
  purge: (entry: string): Promise<{ ok: boolean }> => call(`/api/sculpt-trash/${encodeURIComponent(entry)}`, 'DELETE'),
  emptyTrash: (): Promise<{ ok: boolean; removed: number }> => call('/api/sculpt-trash', 'DELETE'),
  templates: (): Promise<TemplateList> => call('/api/sculpt-templates'),
  templateThumbUrl: (id: string): string => `/api/sculpt-templates/${encodeURIComponent(id)}/thumb`,
  settings: (): Promise<Partial<HomeSettings>> => call('/api/sculpt-settings'),
  setSettings: (patch: Partial<HomeSettings>): Promise<HomeSettings> => call('/api/sculpt-settings', 'POST', patch),
};
