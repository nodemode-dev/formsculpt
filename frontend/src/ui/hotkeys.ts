/**
 * The engine's command catalogue as the window reads it, and the bridge from the old key form's actions
 * ('formsculpt.keys') to command ids, used once when those overrides move to the shortcut preferences.
 */

export interface CatalogueCommand {
  id: string;
  name: string;
  group?: string;
  label: string;
  description?: string;
  keys: string[];
  annotations?: { readOnlyHint?: boolean; destructiveHint?: boolean };
  inputSchema?: { properties?: Record<string, { type?: string; enum?: Array<string | number>; minimum?: number; maximum?: number; description?: string }>; required?: string[] };
}

const DIRECT: Record<string, string> = { undo: 'history.undo', redo: 'history.redo', delete_object: 'object.delete' };

/** The command id an action of the old key form stands for (view:front, file:save, brush:clay, cmd:name[:step]); null when none. */
export function legacyTarget(action: string, catalogue: CatalogueCommand[]): string | null {
  const [kind, name, step] = action.split(':');
  if (kind === 'view' || kind === 'file') return `${kind}.${name}`;
  if (kind === 'brush') return `tool.brush.${name}`;
  if (kind !== 'cmd') return null;
  if (name === 'multires_set_level') return step === 'down' ? 'multires.lower' : 'multires.higher';
  return DIRECT[name] ?? catalogue.find((c) => c.name === name)?.id ?? null;
}

