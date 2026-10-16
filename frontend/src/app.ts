import { VERSION } from './version';
import { ICONS } from './ui/icons';
import { el, saveBlob, setAttr, setText } from './ui/dom';
import { browserEngine, dispatch, fetchBooleanPreview, fetchCatalogue, fetchFile, fetchMesh, fetchVersion, postBody, postJson, uploadFile, useBrowserEngine, type ObjectSummary, type SceneSummary } from './engine/service';
import { WasmEngine } from './engine/wasm';
import { dataUrl, HostLink } from './host';
import { inDesktop, saveNative } from './ui/desktop';

interface PrintReport {
  printable: boolean;
  size_mm: [number, number, number];
  volume_mm3: number;
  wall_mm: { min: number | null };
  problems: string[];
}
import { Viewport, type Ray } from './viewport/viewport';
import { hexColor, type LightSpec } from './viewport/gl-renderer';
import type { ViewName } from './viewport/camera';
import { add, cross, distance3, dot, normalize, scale, sub, type Vec3 } from './viewport/math';
import { LATHES, SKELETONS } from './ui/presets';
import { StrokeClient, type BrushState } from './engine/stroke-client';
import { AgentLink } from './ui/agent-link';
import { legacyTarget, type CatalogueCommand } from './ui/hotkeys';
import { commands, comboOf, normalizeCombo, type Command, type CommandParam } from './ui/commands';
import { NEEDS_OBJECT, PANEL_CMDS } from './ui/command-map';
import { applyShortcuts, persistShortcuts, takeLegacyKeys } from './ui/prefs';
import { overlayOpen } from './ui/dialog';
import { AboutDialog, PromptDialog, ShortcutsDialog } from './ui/app-dialogs';
import { CommandPalette } from './ui/command-palette';
import { MenuBar, PopupMenu } from './ui/menu';
import { menuDefs, OBJECT_ROW_MENU, PIN_MENU, tidy, VIEWPORT_MENU } from './ui/menus';
import { scheduleChrome } from './ui/chrome';
import { language, languages, setLanguage, t } from './ui/i18n';
import { isLang } from './ui/i18n/langs';
import { registerTerminal } from './ui/terminal';
import { Gizmo, type GizmoDrag } from './ui/gizmo';
import { PanelLayout } from './ui/layout';
import { SpaceMouse } from './ui/spacemouse';
import { ENV_PRESETS, imageEnvironment, parseHdr, presetEnvironment, type EnvImage } from './viewport/environment';

/** A plugin as the service lists it: its fields come from the PARAMS it declares, and CLICK makes it a surface tool. */
interface PluginInfo {
  name: string;
  title: string;
  description: string;
  click?: boolean;
  params?: Record<string, { type: string; label?: string; default?: number | string | boolean; min?: number; max?: number; step?: number; options?: string[] }>;
}

interface Shell {
  engineDot: HTMLElement;
  engineText: HTMLElement;
  engineVersion: HTMLElement;
  canvas: HTMLCanvasElement;
  addButtons: HTMLButtonElement[];
  objectList: HTMLUListElement;
  statVerts: HTMLElement;
  statTris: HTMLElement;
  statLevel: HTMLElement;
  statRenderer: HTMLElement;
  statFrame: HTMLElement;
  message: HTMLElement;
  brushButtons: HTMLButtonElement[];
  radius: HTMLInputElement;
  strength: HTMLInputElement;
  invert: HTMLInputElement;
  symX: HTMLInputElement;
  radial: HTMLInputElement;
  dyntopo: HTMLInputElement;
  detail: HTMLInputElement;
  detailMode: HTMLSelectElement;
  paintColor: HTMLInputElement;
  paintRough: HTMLInputElement;
  paintMetal: HTMLInputElement;
  paintOpacityCh: HTMLInputElement;
  paintEmissive: HTMLInputElement;
  paintMaterial: HTMLInputElement;
  shading: HTMLSelectElement;
  paintButtons: HTMLButtonElement[];
  bakeSize: HTMLSelectElement;
  fileButtons: HTMLButtonElement[];
  fileInput: HTMLInputElement;
  exportFormat: HTMLSelectElement;
  nodemode: HTMLInputElement;
  objectButtons: HTMLButtonElement[];
  deformAxis: HTMLSelectElement;
  deformAngle: HTMLInputElement;
  layerSelect: HTMLSelectElement;
  layerStrength: HTMLInputElement;
  layerButtons: HTMLButtonElement[];
  topoButtons: HTMLButtonElement[];
  resolution: HTMLInputElement;
  quadTarget: HTMLInputElement;
  actionButtons: HTMLButtonElement[];
  skeleton: HTMLSelectElement;
  lathe: HTMLSelectElement;
  tubeRadius: HTMLInputElement;
  buildButtons: HTMLButtonElement[];
  tubeLine: SVGPolylineElement;
  lazy: HTMLInputElement;
  fov: HTMLInputElement;
  showGroups: HTMLInputElement;
  trackball: HTMLInputElement;
  insertKind: HTMLSelectElement;
  insertSize: HTMLInputElement;
  arrayCount: HTMLInputElement;
  folderName: HTMLInputElement;
  reference: HTMLImageElement;
  referenceInput: HTMLInputElement;
  referenceOpacity: HTMLInputElement;
  viewButtons: HTMLButtonElement[];
  historyList: HTMLUListElement;
  alpha: HTMLSelectElement;
  alphaAngle: HTMLInputElement;
  strokeMode: HTMLSelectElement;
  alphaInput: HTMLInputElement;
  alphaLoad: HTMLButtonElement;
  matcapInput: HTMLInputElement;
  shadeActive: HTMLInputElement;
  smoothPick: HTMLSelectElement;
  uvSeams: HTMLSelectElement;
  uvView: HTMLDivElement;
  uvImage: HTMLImageElement;
  uvEmpty: HTMLSpanElement;
  uvButtons: HTMLButtonElement[];
  layout: PanelLayout;
  panelLeft: HTMLInputElement;
  uiSize: HTMLSelectElement;
  menubar: HTMLElement;
  falloff: HTMLSelectElement;
  falloffCurve: SVGSVGElement;
  falloffLine: SVGPolylineElement;
  falloffDots: SVGPathElement;
  pressureGamma: HTMLInputElement;
  pressureStrength: HTMLInputElement;
  pressureSize: HTMLInputElement;
  curveButtons: HTMLButtonElement[];
  curveInterp: HTMLSelectElement;
  curveClosed: HTMLInputElement;
  curveRound: HTMLInputElement;
  radiusMode: HTMLSelectElement;
  radiusEnd: HTMLInputElement;
  pointRadius: HTMLInputElement;
  pointTwist: HTMLInputElement;
  curveOffset: HTMLInputElement;
  curveGrid: HTMLInputElement;
  curveChain: HTMLInputElement;
  setPick: HTMLSelectElement;
  tubeProfile: HTMLSelectElement;
  curveIntensity: HTMLSelectElement;
  profileEditor: SVGSVGElement;
  profileLine: SVGPolygonElement;
  profileDots: SVGPathElement;
  profileButtons: HTMLButtonElement[];
  tubeCap: HTMLSelectElement;
  tubeHole: HTMLInputElement;
  tubeSpiral: HTMLInputElement;
  pathButtons: HTMLButtonElement[];
  backdropMode: HTMLSelectElement;
  backdropTop: HTMLInputElement;
  backdropBottom: HTMLInputElement;
  backdropMid: HTMLInputElement;
  backdropVignette: HTMLInputElement;
  floorGrid: HTMLInputElement;
  curveLine: SVGPolylineElement;
  curveHandles: SVGPathElement;
  paintLayer: HTMLSelectElement;
  paintBlend: HTMLSelectElement;
  paintOpacity: HTMLInputElement;
  paintVisible: HTMLInputElement;
  paintLayerButtons: HTMLButtonElement[];
  envSelect: HTMLSelectElement;
  envRotate: HTMLInputElement;
  exposure: HTMLInputElement;
  fixedLights: HTMLInputElement;
  tone: HTMLSelectElement;
  envBackground: HTMLInputElement;
  envBlur: HTMLInputElement;
  envInput: HTMLInputElement;
  envLoad: HTMLButtonElement;
  postSsao: HTMLInputElement;
  postBloom: HTMLInputElement;
  postDof: HTMLInputElement;
  postVignette: HTMLInputElement;
  postGrain: HTMLInputElement;
  gizmoAxes: SVGPathElement[];
  gizmoUniform: SVGPathElement;
  latticeLines: SVGPathElement;
  latticePoints: SVGPathElement;
  referencePlace: HTMLSelectElement;
  arrayLinked: HTMLInputElement;
  vdmPick: HTMLSelectElement;
  textInput: HTMLInputElement;
  fuseBlend: HTMLInputElement;
  liveBoolean: HTMLSelectElement;
  pluginPick: HTMLSelectElement;
  pluginParams: HTMLElement;
  subsurface: HTMLInputElement;
  transmission: HTMLInputElement;
  pluginButtons: HTMLButtonElement[];
  projectOpacity: HTMLInputElement;
  projectInput: HTMLInputElement;
  lightPick: HTMLSelectElement;
  lightAdd: HTMLButtonElement;
  lightRemove: HTMLButtonElement;
  keyAzimuth: HTMLInputElement;
  keyElevation: HTMLInputElement;
  keyIntensity: HTMLInputElement;
  keyColor: HTMLInputElement;
  keyFollows: HTMLInputElement;
  shadows: HTMLInputElement;
  groundShadow: HTMLInputElement;
  primSize: HTMLInputElement;
  primDetail: HTMLInputElement;
  primHeight: HTMLInputElement;
  primApply: HTMLButtonElement;
  noisePattern: HTMLSelectElement;
  noiseSize: HTMLInputElement;
  noiseHeight: HTMLInputElement;
  noiseOctaves: HTMLInputElement;
  noiseGain: HTMLInputElement;
  noiseStretch: HTMLSelectElement;
}

const SVG = 'http://www.w3.org/2000/svg';

function choices(label: string, entries: [string, string][]): HTMLSelectElement {
  const select = el('select');
  select.setAttribute('aria-label', label);
  for (const [value, text] of entries) {
    const option = el('option', '', text);
    option.value = value;
    select.append(option);
  }
  return select;
}

const BRUSHES: [string, string][] = [
  ['orbit', 'Orbit'],
  ['draw', 'Draw'],
  ['clay', 'Clay'],
  ['inflate', 'Inflate'],
  ['pinch', 'Pinch'],
  ['crease', 'Crease'],
  ['flatten', 'Flatten'],
  ['smooth', 'Smooth'],
  ['grab', 'Grab'],
  ['mask', 'Mask'],
  ['paint', 'Paint'],
  ['tube', 'Tube'],
  ['relax', 'Relax'],
  ['planar', 'Planar'],
  ['trim', 'Trim'],
  ['snake_hook', 'Snake hook'],
  ['noise', 'Noise'],
  ['smudge', 'Smudge'],
  ['stamp', 'Drag stamp'],
  ['dragdot', 'Drag dot'],
  ['curve', 'Curve stroke'],
  ['silhouette', 'Silhouette'],
  ['gizmo', 'Transform'],
  ['lattice', 'Lattice'],
  ['mask_box', 'Box mask'],
  ['mask_lasso', 'Lasso mask'],
  ['mask_topo', 'Topo mask'],
  ['mask_group', 'Mask group'],
  ['hide_group', 'Hide group'],
  ['insert', 'Insert'],
  ['plugin', 'Plugin tool'],
  ['vdm', 'VDM stamp'],
  ['cut', 'Lasso cut'],
  ['trim_shape', 'Lasso trim'],
  ['insert_brush', 'Insert brush'],
  ['group_brush', 'Group brush'],
  ['retopo', 'Retopo quad'],
  ['edge_loop', 'Edge loop'],
  ['bevel_edge', 'Bevel edge'],
  ['clip', 'Clip'],
  ['zsphere', 'Add joint'],
  ['pose_joint', 'Pose joint'],
  ['groom_comb', 'Comb fibers'],
  ['groom_cut', 'Cut fibers'],
];

/** Window-side tools that draw or click instead of streaming a stroke to the engine. */
const SHAPE_TOOLS = new Set(['mask_box', 'mask_lasso', 'stamp', 'dragdot', 'curve', 'gizmo', 'lattice', 'silhouette', 'cut', 'trim_shape', 'insert_brush', 'group_brush']);

/** A #rrggbb colour as linear 0..1 light, normalised so its brightest channel is 1 (strength is set apart). */
function hexToLinear(hex: string): [number, number, number] {
  const n = parseInt(hex.replace('#', ''), 16) || 0xffffff;
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => Math.pow(v / 255, 2.2));
  const m = Math.max(1e-3, ...c);
  return [c[0] / m, c[1] / m, c[2] / m];
}

/** Control points per axis of the window's lattice tool. */
const LATTICE_DIV = 3;

/** Named falloffs as the engine computes them, to draw a preset in the curve editor. */
const FALLOFFS: Record<string, (t: number) => number> = {
  smooth: (t) => 1 - t * t * (3 - 2 * t),
  sphere: (t) => Math.sqrt(Math.max(0, 1 - t * t)),
  linear: (t) => 1 - t,
  sharp: (t) => (1 - t) * (1 - t),
};
const FALLOFF_POINTS = 5;

const ALPHAS: [string, string][] = [
  ['none', 'None'],
  ['square', 'Square'],
  ['noise', 'Noise'],
  ['cracks', 'Cracks'],
  ['rings', 'Rings'],
  ['star', 'Star'],
  ['stripes', 'Stripes'],
  ['dots', 'Dots'],
];
const CLICK_TOOLS = new Set(['mask_topo', 'mask_group', 'hide_group', 'insert', 'vdm', 'plugin', 'retopo', 'edge_loop', 'bevel_edge', 'clip', 'zsphere', 'pose_joint', 'groom_comb', 'groom_cut']);

const TOPOLOGY: [string, string, Record<string, unknown>][] = [
  ['subdivide', 'Subdivide', { levels: 1 }],
  ['remesh_voxel', 'Voxel remesh', {}],
  ['decimate', 'Decimate 50%', { ratio: 0.5 }],
  ['fill_holes', 'Fill holes', {}],
  ['quad_remesh', 'Quad remesh', {}],
  ['delete_faces', 'Delete masked faces', { which: 'masked' }],
  ['bridge', 'Bridge borders', { segments: 4 }],
];

const MULTIRES: [string, string, Record<string, unknown>][] = [
  ['multires_divide', 'Divide', {}],
  ['multires_set_level', 'Lower', { step: 'down' }],
  ['multires_set_level', 'Higher', { step: 'up' }],
  ['multires_delete_lower', 'Delete lower', {}],
  ['multires_delete_higher', 'Delete higher', {}],
  ['multires_reconstruct', 'Rebuild lower', {}],
];

const FILE_ACTIONS: [string, string][] = [
  ['open', 'Open'],
  ['save', 'Save'],
  ['export', 'Export'],
  ['print', 'Print check'],
  ['repair', 'Repair'],
  ['recover', 'Recover'],
  ['nodemode', 'Send to NodeMode'],
];

const NODEMODE_KEY = 'formsculptor.nodemode';
const BACKDROP_KEY = 'formsculptor.backdrop';
const PROFILES_KEY = 'formsculptor.profiles';

const OBJECT_ACTIONS: [string, string][] = [
  ['duplicate', 'Duplicate'],
  ['split', 'Split parts'],
  ['merge', 'Merge all'],
  ['union', 'Union'],
  ['difference', 'Subtract'],
  ['intersection', 'Intersect'],
  ['bend', 'Bend'],
  ['twist', 'Twist'],
  ['taper', 'Taper'],
  ['project', 'Project from'],
  ['align', 'Align'],
  ['distribute', 'Distribute'],
  ['folder', 'Put in folder'],
  ['isolate_folder', 'Isolate folder'],
  ['bevel', 'Bevel edges'],
  ['bridge', 'Bridge to app'],
  ['cloth', 'Drape cloth'],
  ['pose', 'Pose'],
  ['fibers', 'Grow fibers'],
  ['render', 'Path render'],
  ['substance', 'To Substance'],
  ['substance_sync', 'From Substance'],
];

/** Edit buttons whose engine call differs from their name. */
const ACTION_CALLS: Record<string, [string, Record<string, unknown>]> = {
  mask_grow: ['mask_grow', { steps: 1 }],
  mask_shrink: ['mask_grow', { steps: -1 }],
  mask_cavity: ['mask_cavity', { mode: 'cavity' }],
  mask_peaks: ['mask_cavity', { mode: 'peaks' }],
  group_mask: ['group_from_mask', {}],
  group_auto: ['group_auto', { by: 'angle', angle: 30 }],
  group_by_paint: ['group_auto', { by: 'paint' }],
  group_by_uv: ['group_auto', { by: 'uv' }],
  hide_masked: ['hide_faces', { action: 'hide', by: 'mask' }],
  invert_hidden: ['hide_faces', { action: 'invert' }],
  show_faces: ['hide_faces', { action: 'show_all' }],
};

const LAYER_ACTIONS: [string, string][] = [
  ['layer_add', 'Add layer'],
  ['layer_apply', 'Apply'],
  ['layer_delete', 'Delete'],
  ['surface_noise', 'Noise layer'],
];

const ACTIONS: [string, string][] = [
  ['undo', 'Undo'],
  ['redo', 'Redo'],
  ['mask_invert', 'Invert mask'],
  ['mask_blur', 'Blur mask'],
  ['mask_sharpen', 'Sharpen mask'],
  ['mask_grow', 'Grow mask'],
  ['mask_shrink', 'Shrink mask'],
  ['mask_cavity', 'Mask cavity'],
  ['mask_peaks', 'Mask peaks'],
  ['mask_color', 'Mask colour'],
  ['group_mask', 'Group from mask'],
  ['group_auto', 'Auto groups'],
  ['group_by_paint', 'Groups by paint'],
  ['group_by_uv', 'Groups by UV'],
  ['hide_masked', 'Hide masked'],
  ['invert_hidden', 'Invert hidden'],
  ['show_faces', 'Show faces'],
  ['extract_mask', 'Extract mask'],
  ['extrude_mask', 'Extrude mask'],
  ['inset_mask', 'Inset mask'],
  ['mask_clear', 'Clear mask'],
];

function field(label: string, input: HTMLInputElement | HTMLSelectElement): HTMLLabelElement {
  const row = el('label', 'field');
  row.append(el('span', '', label), input);
  return row;
}

function input(type: string, attrs: Record<string, string>): HTMLInputElement {
  const node = el('input');
  node.type = type;
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, v);
  return node;
}

const PRIMITIVES: [string, string][] = [
  ['sphere', 'Sphere'],
  ['cube', 'Cube'],
  ['cylinder', 'Cylinder'],
  ['cone', 'Cone'],
  ['torus', 'Torus'],
  ['plane', 'Plane'],
];

function buildShell(root: HTMLElement): Shell {
  const bar = el('header', 'appbar');
  const brand = el('div', 'brand');
  brand.innerHTML = ICONS.mark;
  brand.append(el('span', '', 'formsculptor'), el('span', 'ver', `v${VERSION}`));
  const menubar = el('div', 'fs-menubar');
  bar.append(brand, menubar, el('div', 'spacer'), el('div', 'appbar-tools'));

  const rail = el('nav', 'rail');
  rail.setAttribute('aria-label', 'Tools');

  const stage = el('main', 'stage');
  const canvas = el('canvas', 'viewport');
  canvas.setAttribute('aria-label', 'Viewport');
  canvas.tabIndex = 0;
  stage.append(canvas);
  // A reference image over the model (see-through, so the model shows under it), sized to fit and never reflowing
  // anything: it is absolutely placed, ignores the pointer and stays hidden until an image is chosen.
  const reference = el('img', 'stage-reference');
  reference.alt = '';
  reference.hidden = true;
  stage.append(reference);
  const overlay = document.createElementNS(SVG, 'svg');
  overlay.setAttribute('class', 'stage-overlay');
  overlay.setAttribute('aria-hidden', 'true');
  const tubeLine = document.createElementNS(SVG, 'polyline');
  // A curve stroke waiting to be applied: its line and its handles, redrawn from 3D points as the view moves.
  const curveLine = document.createElementNS(SVG, 'polyline');
  const curveHandles = document.createElementNS(SVG, 'path');
  // The transform gizmo (an axis path each, and a centre square) and the lattice cage (edges and points).
  const svgPath = (cls: string): SVGPathElement => {
    const p = document.createElementNS(SVG, 'path');
    p.setAttribute('class', cls);
    return p;
  };
  const gizmoAxes = ['x', 'y', 'z'].map((a) => svgPath(`gizmo-axis gizmo-${a}`));
  const gizmoUniform = svgPath('gizmo-uniform');
  const latticeLines = svgPath('lattice-lines');
  const latticePoints = svgPath('lattice-points');
  overlay.append(tubeLine, curveLine, curveHandles, ...gizmoAxes, gizmoUniform, latticeLines, latticePoints);
  stage.append(overlay);

  const panel = el('aside', 'panel');
  panel.append(el('h2', '', 'File'));
  const fileGrid = el('div', 'add-grid two');
  const fileButtons = FILE_ACTIONS.map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    if (action === 'nodemode') b.classList.add('wide');
    fileGrid.append(b);
    return b;
  });
  panel.append(fileGrid);
  const fileInput = input('file', { accept: '.fsc,.obj,.stl,.ply,.glb,.gltf,.3mf,.fbx,.usda,.usd', 'aria-label': 'Choose a file' });
  fileInput.hidden = true;
  panel.append(fileInput);
  const exportFormat = el('select');
  exportFormat.setAttribute('aria-label', 'Export format');
  for (const f of ['glb', 'obj', 'fbx', 'usda', 'stl', 'ply', '3mf']) {
    const option = el('option', '', f.toUpperCase());
    option.value = f;
    exportFormat.append(option);
  }
  let remembered = '';
  try {
    remembered = localStorage.getItem(NODEMODE_KEY) ?? '';
  } catch {
    // storage blocked; the default address stands
  }
  const nodemode = input('url', { value: remembered || 'http://localhost:8000', 'aria-label': 'NodeMode address', spellcheck: 'false' });
  const fileFields = el('div', 'fields');
  fileFields.append(field('Export format', exportFormat), field('NodeMode address', nodemode));
  panel.append(fileFields);
  panel.append(el('h2', 'gap', 'Add'));
  const grid = el('div', 'add-grid');
  const addButtons = PRIMITIVES.map(([kind, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.kind = kind;
    grid.append(b);
    return b;
  });
  panel.append(grid);
  // A new primitive stays editable until something else changes it: these fields show its settings and Apply
  // remakes it; they are disabled for any other object.
  const primSize = input('number', { min: '0.01', max: '1000', step: '0.1', value: '1', 'aria-label': 'Primitive size' });
  const primDetail = input('number', { min: '0', max: '2048', step: '1', value: '5', 'aria-label': 'Primitive detail' });
  const primHeight = input('number', { min: '0.01', max: '1000', step: '0.1', value: '2', 'aria-label': 'Primitive height' });
  for (const c of [primSize, primDetail, primHeight]) c.disabled = true;
  const primFields = el('div', 'fields after-fields');
  primFields.append(field('Primitive size', primSize), field('Primitive detail', primDetail), field('Primitive height', primHeight));
  panel.append(primFields);
  const primGrid = el('div', 'add-grid two after-fields');
  const primApply = el('button', 'btn', 'Apply to primitive');
  primApply.type = 'button';
  primApply.disabled = true;
  primGrid.append(primApply);
  panel.append(primGrid);

  panel.append(el('h2', 'gap', 'Build'));
  const skeleton = choices('Skeleton', Object.entries(SKELETONS).map(([k, v]) => [k, v.label]));
  const lathe = choices('Lathe profile', Object.entries(LATHES).map(([k, v]) => [k, v.label]));
  const tubeRadius = input('number', { min: '0.005', max: '1', step: '0.01', value: '0.05', 'aria-label': 'Tube radius' });
  const insertKind = choices('Insert shape', [['sphere', 'Sphere'], ['cube', 'Cube'], ['cylinder', 'Cylinder'], ['cone', 'Cone'], ['torus', 'Torus'], ['previous', 'Previous object']]);
  const insertSize = input('number', { min: '0.01', max: '10', step: '0.05', value: '0.2', 'aria-label': 'Insert size' });
  const arrayCount = input('number', { min: '2', max: '200', value: '6', 'aria-label': 'Array copies' });
  const arrayLinked = input('checkbox', { 'aria-label': 'Linked copies' });
  const vdmPick = choices('Stamp map', [['horn', 'Horn'], ['hook', 'Hook'], ['mushroom', 'Mushroom']]);
  const textInput = input('text', { maxlength: '160', value: 'FORM', 'aria-label': 'Text', spellcheck: 'false' });
  const buildFields = el('div', 'fields');
  buildFields.append(field('Skeleton', skeleton), field('Lathe profile', lathe), field('Tube radius', tubeRadius), field('Insert shape', insertKind), field('Insert size', insertSize), field('Array copies', arrayCount), field('Linked copies', arrayLinked), field('Stamp map', vdmPick), field('Text', textInput));
  panel.append(buildFields);
  const buildGrid = el('div', 'add-grid two');
  const buildButtons = (
    [
      ['skeleton', 'Build skeleton'],
      ['lathe', 'Lathe'],
      ['array_line', 'Array in line'],
      ['array_radial', 'Array radial'],
      ['array_mirror', 'Mirror copy'],
      ['array_curve', 'Array on curve'],
      ['shadow_box', 'Build shadow box'],
      ['clear_outlines', 'Clear outlines'],
      ['extrude_outline', 'Extrude outline'],
      ['vdm_capture', 'Capture VDM'],
      ['text', 'Build text'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    buildGrid.append(b);
    return b;
  });
  buildGrid.classList.add('after-fields');
  panel.append(buildGrid);

  panel.append(el('h2', 'gap', 'Brush'));
  const brushGrid = el('div', 'add-grid');
  const brushButtons = BRUSHES.map(([kind, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.brush = kind;
    b.setAttribute('aria-pressed', kind === 'draw' ? 'true' : 'false');
    brushGrid.append(b);
    return b;
  });
  panel.append(brushGrid);
  const radius = input('range', { min: '4', max: '300', value: '60', 'aria-label': 'Radius' });
  const strength = input('range', { min: '0', max: '100', value: '50', 'aria-label': 'Strength' });
  const invert = input('checkbox', { 'aria-label': 'Invert' });
  const symX = input('checkbox', { 'aria-label': 'Mirror X' });
  const radial = input('number', { min: '1', max: '150', value: '1', 'aria-label': 'Radial' });
  const dyntopo = input('checkbox', { 'aria-label': 'Dynamic topology' });
  const detail = input('range', { min: '5', max: '100', value: '25', 'aria-label': 'Detail size' });
  const detailMode = el('select');
  detailMode.setAttribute('aria-label', 'Detail mode');
  for (const [value, label] of [['brush', 'Brush'], ['constant', 'Constant'], ['screen', 'Screen']]) {
    const option = el('option', '', label);
    option.value = value;
    detailMode.append(option);
  }
  const lazy = input('range', { min: '0', max: '95', value: '0', 'aria-label': 'Lazy mouse' });
  const alpha = choices('Alpha', ALPHAS);
  const alphaAngle = input('number', { min: '-360', max: '360', step: '5', value: '0', 'aria-label': 'Alpha angle' });
  const strokeMode = choices('Stroke', [['dots', 'Dots (upright)'], ['roll', 'Roll (along stroke)'], ['spray', 'Spray (scattered)']]);
  const alphaInput = input('file', { accept: 'image/png', 'aria-label': 'Choose an alpha image' });
  alphaInput.hidden = true;
  panel.append(alphaInput);
  const falloff = choices('Falloff', [['smooth', 'Smooth'], ['sphere', 'Sphere'], ['linear', 'Linear'], ['sharp', 'Sharp'], ['custom', 'Drawn curve']]);
  const pressureGamma = input('number', { min: '0.2', max: '5', step: '0.1', value: '1', 'aria-label': 'Pressure curve' });
  const pressureStrength = input('checkbox', { 'aria-label': 'Pressure sets strength' });
  pressureStrength.checked = true;
  const pressureSize = input('checkbox', { 'aria-label': 'Pressure sets size' });
  const fields = el('div', 'fields');
  fields.append(field('Radius', radius), field('Strength', strength), field('Lazy mouse', lazy), field('Invert (Ctrl)', invert), field('Mirror X', symX), field('Radial', radial), field('Dyntopo', dyntopo), field('Detail size', detail), field('Detail mode', detailMode), field('Alpha', alpha), field('Alpha angle', alphaAngle), field('Stroke', strokeMode), field('Pressure curve', pressureGamma), field('Pressure strength', pressureStrength), field('Pressure size', pressureSize), field('Falloff', falloff));
  panel.append(fields);
  // The falloff curve: five weights from the centre (left) to the rim (right), dragged up and down; always shown, so
  // choosing a preset or drawing never changes the panel's height.
  const falloffCurve = document.createElementNS(SVG, 'svg');
  falloffCurve.setAttribute('class', 'falloff-editor');
  falloffCurve.setAttribute('viewBox', '0 0 200 64');
  falloffCurve.setAttribute('preserveAspectRatio', 'none');
  falloffCurve.setAttribute('role', 'img');
  falloffCurve.setAttribute('aria-label', 'Falloff curve, drag the points');
  const falloffLine = document.createElementNS(SVG, 'polyline');
  const falloffDots = document.createElementNS(SVG, 'path');
  falloffCurve.append(falloffLine, falloffDots);
  panel.append(falloffCurve);
  const alphaGrid = el('div', 'add-grid after-fields');
  const curveButtons = (
    [
      ['alpha', 'Load alpha'],
      ['apply_curve', 'Apply curve'],
      ['clear_curve', 'Clear curve'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    alphaGrid.append(b);
    return b;
  });
  const alphaLoad = curveButtons[0];
  panel.append(alphaGrid);
  const curveInterp = choices('Path shape', [['catmull_rom', 'Smooth'], ['bspline', 'Softer'], ['linear', 'Straight']]);
  const curveClosed = input('checkbox', { 'aria-label': 'Closed path' });
  const curveRound = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Corner round' });
  const tubeProfile = choices('Tube profile', [['circle', 'Circle'], ['square', 'Square'], ['triangle', 'Triangle'], ['star', 'Star'], ['flat', 'Flat strap'], ['custom', 'Drawn below']]);
  const tubeCap = choices('Tube ends', [['flat', 'Flat'], ['round', 'Round'], ['none', 'Open']]);
  const tubeHole = input('number', { min: '0', max: '1', step: '0.005', value: '0', 'aria-label': 'Pipe wall' });
  const tubeSpiral = input('number', { min: '0', max: '64', step: '0.5', value: '0', 'aria-label': 'Spiral turns' });
  const pathFields = el('div', 'fields after-fields');
  const radiusMode = choices('Radius along path', [['uniform', 'Uniform'], ['start_end', 'Start to end'], ['per_point', 'Per point']]);
  const radiusEnd = input('range', { min: '5', max: '300', value: '100', 'aria-label': 'End radius %' });
  const pointRadius = input('range', { min: '5', max: '400', value: '100', 'aria-label': 'Point radius %' });
  const pointTwist = input('number', { min: '-720', max: '720', step: '5', value: '0', 'aria-label': 'Point twist' });
  const curveOffset = input('number', { min: '-1', max: '1', step: '0.005', value: '0', 'aria-label': 'Surface offset' });
  const curveGrid = input('number', { min: '0', max: '10', step: '0.05', value: '0', 'aria-label': 'Grid snap' });
  const curveChain = input('checkbox', { 'aria-label': 'Drag as chain' });
  const curveIntensity = choices('Intensity along curve', [['even', 'Even'], ['fade_in', 'Fade in'], ['fade_out', 'Fade out'], ['fade_both', 'Fade both ends'], ['pulse', 'Pulse']]);
  pathFields.append(field('Path shape', curveInterp), field('Closed path', curveClosed), field('Corner round', curveRound), field('Surface offset', curveOffset), field('Grid snap', curveGrid), field('Drag as chain', curveChain), field('Intensity along curve', curveIntensity), field('Tube profile', tubeProfile), field('Radius along path', radiusMode), field('End radius %', radiusEnd), field('Point radius %', pointRadius), field('Point twist', pointTwist), field('Tube ends', tubeCap), field('Pipe wall', tubeHole), field('Spiral turns', tubeSpiral));
  panel.append(pathFields);
  // The tube profile editor: a closed outline in units of the tube radius, always shown at a fixed size.
  const profileEditor = document.createElementNS(SVG, 'svg');
  profileEditor.setAttribute('class', 'profile-editor');
  profileEditor.setAttribute('viewBox', '0 0 200 200');
  profileEditor.setAttribute('role', 'img');
  profileEditor.setAttribute('aria-label', 'Tube profile, drag the points');
  const profileAxes = document.createElementNS(SVG, 'path');
  profileAxes.setAttribute('class', 'profile-axes');
  profileAxes.setAttribute('d', 'M100,8V192M8,100H192');
  const profileLine = document.createElementNS(SVG, 'polygon');
  const profileDots = document.createElementNS(SVG, 'path');
  profileEditor.append(profileAxes, profileLine, profileDots);
  panel.append(profileEditor);
  const profileGrid = el('div', 'add-grid two after-fields');
  const profileButtons = (
    [
      ['profile_save', 'Save profile'],
      ['profile_delete', 'Delete saved profile'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    profileGrid.append(b);
    return b;
  });
  panel.append(profileGrid);
  panel.append(el('p', 'panel-hint', 'Profile: drag a point, click an edge to add one, Alt-click a point for a corner or a smooth point, Shift-click to remove it.'));
  const pathGrid = el('div', 'add-grid two after-fields');
  const pathButtons = (
    [
      ['toggle_corner', 'Sharp or smooth'],
      ['all_sharp', 'All corners sharp'],
      ['all_smooth', 'All points smooth'],
      ['tube_curve', 'Tube from curve'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    pathGrid.append(b);
    return b;
  });
  panel.append(pathGrid);
  const makeGrid = el('div', 'add-grid two after-fields');
  for (const [cmd, label] of [['curve.edit_last_stroke', 'Last stroke as curve'], ['curve.from_mask_edge', 'Curve from mask edge'], ['curve.from_group_border', 'Curve from group border']]) {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.cmd = cmd;
    makeGrid.append(b);
  }
  panel.append(makeGrid);
  panel.append(el('p', 'panel-hint', 'Alt-click a curve handle to make it a corner; Corner round rounds the chosen or every sharp corner.'));
  panel.append(el('p', 'panel-hint', 'Click on the object without dragging to add a curve point.'));

  panel.append(el('h2', 'gap', 'Paint'));
  const paintColor = input('color', { value: '#c8402f', 'aria-label': 'Paint colour' });
  const paintRough = input('range', { min: '0', max: '100', value: '50', 'aria-label': 'Paint roughness' });
  const paintMetal = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Paint metalness' });
  const paintMaterial = input('checkbox', { 'aria-label': 'Paint roughness and metalness' });
  const paintOpacityCh = input('range', { min: '0', max: '100', value: '100', 'aria-label': 'Paint opacity channel' });
  const paintEmissive = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Paint emissive' });
  const shading = el('select');
  shading.setAttribute('aria-label', 'Shading');
  for (const [value, label] of [['clay', 'Clay matcap'], ['wax', 'Red wax'], ['pearl', 'Pearl'], ['bronze', 'Bronze'], ['gray', 'Gray'], ['pbr', 'Lit (PBR)']]) {
    const option = el('option', '', label);
    option.value = value;
    shading.append(option);
  }
  const paintFields = el('div', 'fields');
  const bakeSize = el('select');
  bakeSize.setAttribute('aria-label', 'Bake size');
  for (const size of ['512', '1024', '2048', '4096']) {
    const option = el('option', '', `${size} px`);
    option.value = size;
    option.selected = size === '1024';
    bakeSize.append(option);
  }
  const projectOpacity = input('range', { min: '5', max: '100', value: '100', 'aria-label': 'Projection opacity' });
  const projectInput = input('file', { accept: 'image/png,image/jpeg,image/webp', 'aria-label': 'Choose an image to paint' });
  projectInput.hidden = true;
  panel.append(projectInput);
  const subsurface = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Subsurface' });
  const transmission = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Transmission' });
  paintFields.append(field('Colour', paintColor), field('Roughness', paintRough), field('Metalness', paintMetal), field('Opacity', paintOpacityCh), field('Emissive', paintEmissive), field('Paint material too', paintMaterial), field('Shading', shading), field('Subsurface', subsurface), field('Transmission', transmission), field('Bake size', bakeSize), field('Image opacity', projectOpacity));
  panel.append(paintFields);
  const paintGrid = el('div', 'add-grid two');
  const paintButtons = (
    [
      ['paint_fill', 'Fill object'],
      ['paint_clear', 'Clear paint'],
      ['uv_unwrap', 'Unwrap UVs'],
      ['bake_maps', 'Bake maps'],
      ['maps', 'Download maps'],
      ['project_view', 'Project image'],
      ['project_tile', 'Tile image'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    paintGrid.append(b);
    return b;
  });
  panel.append(paintGrid);
  const paintLayer = choices('Paint layer', [['', 'Base']]);
  const paintBlend = choices('Paint layer blend', [['normal', 'Normal'], ['multiply', 'Multiply'], ['screen', 'Screen'], ['overlay', 'Overlay'], ['add', 'Add']]);
  const paintOpacity = input('range', { min: '0', max: '100', value: '100', 'aria-label': 'Paint layer opacity' });
  const paintVisible = input('checkbox', { 'aria-label': 'Paint layer visible' });
  paintVisible.checked = true;
  for (const c of [paintBlend, paintOpacity, paintVisible]) c.disabled = true;
  const paintLayerFields = el('div', 'fields after-fields');
  paintLayerFields.append(field('Paint into', paintLayer), field('Blend', paintBlend), field('Opacity', paintOpacity), field('Visible', paintVisible));
  panel.append(paintLayerFields);
  const paintLayerGrid = el('div', 'add-grid two after-fields');
  const paintLayerButtons = (
    [
      ['paint_layer_add', 'New paint layer'],
      ['paint_layer_delete', 'Delete paint layer'],
      ['paint_layer_merge', 'Merge down'],
      ['paint_layer_flatten', 'Flatten paint'],
      ['up', 'Move up'],
      ['down', 'Move down'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    if (action !== 'paint_layer_add') b.disabled = true;
    paintLayerGrid.append(b);
    return b;
  });
  panel.append(paintLayerGrid);

  panel.append(el('h2', 'gap', 'UV'));
  const uvSeams = choices('UV seams', [['direction', 'By facing'], ['groups', 'Along face groups']]);
  const uvFields = el('div', 'fields');
  uvFields.append(field('Seams', uvSeams));
  panel.append(uvFields);
  // The layout picture keeps its square whether or not there is a layout, so nothing below it moves.
  const uvView = el('div', 'uv-view');
  const uvImage = el('img');
  uvImage.alt = 'UV layout of the active object';
  uvImage.draggable = false;
  uvImage.hidden = true;
  const uvEmpty = el('span', '', 'No UVs yet; Unwrap to make them');
  uvView.append(uvImage, uvEmpty);
  panel.append(uvView);
  const uvGrid = el('div', 'add-grid three after-fields');
  const uvButtons = (
    [
      ['unwrap', 'Unwrap'],
      ['pack', 'Pack'],
      ['rotate', 'Turn 90'],
      ['grow', 'Scale up'],
      ['shrink', 'Scale down'],
      ['refresh', 'Redraw'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    uvGrid.append(b);
    return b;
  });
  panel.append(uvGrid);
  panel.append(el('p', 'panel-hint', 'Click an island to choose it; drag to move it.'));

  panel.append(el('h2', 'gap', 'Edit'));
  const actionGrid = el('div', 'add-grid two');
  const actionButtons = ACTIONS.map(([name, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = name;
    actionGrid.append(b);
    return b;
  });
  panel.append(actionGrid);

  panel.append(el('h2', 'gap', 'Topology'));
  const topoGrid = el('div', 'add-grid two');
  const topoButtons = TOPOLOGY.map(([name, label, args]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = name;
    b.dataset.args = JSON.stringify(args);
    topoGrid.append(b);
    return b;
  });
  panel.append(topoGrid);
  const resolution = input('number', { min: '8', max: '320', value: '128', 'aria-label': 'Voxel resolution' });
  const topoFields = el('div', 'fields');
  const quadTarget = input('number', { min: '100', max: '400000', value: '4000', 'aria-label': 'Target quads' });
  topoFields.append(field('Voxel resolution', resolution), field('Target quads', quadTarget));
  panel.append(topoFields);

  panel.append(el('h2', 'gap', 'Multiresolution'));
  const levelGrid = el('div', 'add-grid two');
  for (const [name, label, args] of MULTIRES) {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = name;
    b.dataset.args = JSON.stringify(args);
    levelGrid.append(b);
    topoButtons.push(b);
  }
  panel.append(levelGrid);

  panel.append(el('h2', 'gap', 'Object'));
  const objectGrid = el('div', 'add-grid three');
  const objectButtons = OBJECT_ACTIONS.map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    objectGrid.append(b);
    return b;
  });
  panel.append(objectGrid);
  const deformAxis = el('select');
  deformAxis.setAttribute('aria-label', 'Deform axis');
  for (const a of ['x', 'y', 'z']) {
    const option = el('option', '', a.toUpperCase());
    option.value = a;
    option.selected = a === 'y';
    deformAxis.append(option);
  }
  const deformAngle = input('number', { min: '-720', max: '720', value: '45', 'aria-label': 'Deform angle' });
  const objectFields = el('div', 'fields');
  const folderName = input('text', { maxlength: '80', value: 'Folder 1', 'aria-label': 'Folder name', spellcheck: 'false' });
  const fuseBlend = input('number', { min: '0', max: '10', step: '0.02', value: '0', 'aria-label': 'Boolean blend' });
  const liveBoolean = choices('Live boolean', [['', 'Off'], ['union', 'Union'], ['difference', 'Difference'], ['intersection', 'Intersection']]);
  objectFields.append(field('Deform axis', deformAxis), field('Angle or taper %', deformAngle), field('Folder name', folderName), field('Boolean blend', fuseBlend), field('Live boolean', liveBoolean));
  panel.append(objectFields);

  panel.append(el('h2', 'gap', 'Layers'));
  const layerSelect = el('select');
  layerSelect.setAttribute('aria-label', 'Layer');
  const baseOption = el('option', '', 'Base mesh');
  baseOption.value = '';
  layerSelect.append(baseOption);
  const layerStrength = input('range', { min: '-100', max: '200', value: '100', 'aria-label': 'Layer strength' });
  layerStrength.disabled = true;
  const layerFields = el('div', 'fields');
  // The noise editor for the Noise layer button: pattern, feature size and height as percentages of the object's
  // size, octaves, gain and a stretch; the alpha pattern uses the Brush section's alpha.
  const noisePattern = choices('Noise pattern', [['value', 'Value'], ['ridged', 'Ridged'], ['turbulence', 'Turbulence'], ['cells', 'Cells'], ['alpha', 'Alpha (Brush alpha)']]);
  const noiseSize = input('number', { min: '0.2', max: '50', step: '0.2', value: '2', 'aria-label': 'Noise size' });
  const noiseHeight = input('number', { min: '-10', max: '10', step: '0.1', value: '0.4', 'aria-label': 'Noise height' });
  const noiseOctaves = input('number', { min: '1', max: '8', value: '3', 'aria-label': 'Noise octaves' });
  const noiseGain = input('range', { min: '10', max: '90', value: '50', 'aria-label': 'Noise gain' });
  const noiseStretch = choices('Noise stretch', [['none', 'None'], ['x', 'Along x'], ['y', 'Along y'], ['z', 'Along z']]);
  layerFields.append(field('Record into', layerSelect), field('Strength', layerStrength), field('Noise pattern', noisePattern), field('Noise size %', noiseSize), field('Noise height %', noiseHeight), field('Octaves', noiseOctaves), field('Gain', noiseGain), field('Stretch', noiseStretch));
  panel.append(layerFields);
  const layerGrid = el('div', 'add-grid three after-fields');
  const layerButtons = LAYER_ACTIONS.map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    layerGrid.append(b);
    return b;
  });
  panel.append(layerGrid);

  panel.append(el('h2', 'gap', 'Selection Sets'));
  const setPick = el('select');
  setPick.setAttribute('aria-label', 'Selection set');
  const noSet = el('option', '', 'None saved');
  noSet.value = '';
  setPick.append(noSet);
  const setFields = el('div', 'fields');
  setFields.append(field('Set', setPick));
  panel.append(setFields);
  const setGrid = el('div', 'add-grid three after-fields');
  for (const [cmd, label] of [['selection.save', 'Save'], ['selection.restore', 'Restore'], ['selection.delete', 'Delete']]) {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.cmd = cmd;
    setGrid.append(b);
  }
  panel.append(setGrid);

  panel.append(el('h2', 'gap', 'View'));
  const fov = input('range', { min: '15', max: '100', value: '40', 'aria-label': 'Field of view' });
  const showGroups = input('checkbox', { 'aria-label': 'Show face groups' });
  const trackball = input('checkbox', { 'aria-label': 'Trackball rotation' });
  const referenceOpacity = input('range', { min: '5', max: '100', value: '45', 'aria-label': 'Reference opacity' });
  const referenceInput = input('file', { accept: 'image/png,image/jpeg,image/webp', 'aria-label': 'Choose a reference image' });
  referenceInput.hidden = true;
  panel.append(referenceInput);
  const viewFields = el('div', 'fields');
  const shadeActive = input('checkbox', { 'aria-label': 'Shade the active object only' });
  const matcapInput = input('file', { accept: 'image/png,image/jpeg,image/webp', 'aria-label': 'Choose a matcap image' });
  matcapInput.hidden = true;
  panel.append(matcapInput);
  const referencePlace = choices('Reference place', [['screen', 'Over the view'], ['front', 'Front plane'], ['side', 'Side plane'], ['top', 'Top plane']]);
  const panelLeft = input('checkbox', { 'aria-label': 'Panel on the left' });
  const uiSize = choices('Interface size', [['small', 'Small'], ['normal', 'Normal'], ['large', 'Large']]);
  const backdropMode = choices('Background', [['gradient', 'Gradient'], ['color', 'Flat colour']]);
  const backdropTop = input('color', { value: '#3c3c41', 'aria-label': 'Background top' });
  const backdropBottom = input('color', { value: '#17171a', 'aria-label': 'Background bottom' });
  const backdropMid = input('range', { min: '5', max: '95', value: '55', 'aria-label': 'Gradient midpoint' });
  const backdropVignette = input('range', { min: '0', max: '100', value: '35', 'aria-label': 'Background vignette' });
  const floorGrid = input('checkbox', { 'aria-label': 'Floor grid' });
  floorGrid.checked = true;
  const langPick = choices('Language', languages().map(([code, name]) => [code, name] as [string, string]));
  langPick.dataset.noI18n = '';
  langPick.value = language();
  langPick.addEventListener('change', () => { if (isLang(langPick.value)) void setLanguage(langPick.value); });
  const smoothPick = choices('Smooth preview', [['0', 'Off'], ['1', '1 level'], ['2', '2 levels'], ['3', '3 levels']]);
  viewFields.append(field('Field of view', fov), field('Face groups', showGroups), field('Trackball', trackball), field('Reference opacity', referenceOpacity), field('Reference place', referencePlace), field('Shade active only', shadeActive), field('Smooth preview', smoothPick), field('Panel on the left', panelLeft), field('Interface size', uiSize), field('Language', langPick), field('Background', backdropMode), field('Background top', backdropTop), field('Background bottom', backdropBottom), field('Gradient midpoint', backdropMid), field('Background vignette', backdropVignette), field('Floor grid', floorGrid));
  panel.append(viewFields);
  const viewGrid = el('div', 'add-grid three after-fields');
  const viewButtons = (
    [
      ['hide', 'Hide'],
      ['isolate', 'Isolate'],
      ['show_all', 'Show all'],
      ['reference', 'Reference'],
      ['clear_reference', 'Clear ref'],
      ['turntable', 'Turntable'],
      ['anim_key', 'Add key'],
      ['anim_record', 'Record animation'],
      ['anim_clear', 'Clear keys'],
      ['history_movie', 'History movie'],
      ['spacemouse', '3D mouse'],
      ['matcap', 'Load matcap'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    viewGrid.append(b);
    return b;
  });
  // Layout only, so it is wired at the end of buildShell rather than through the view actions.
  const clearRail = el('button', 'btn', 'Clear rail');
  clearRail.type = 'button';
  viewGrid.append(clearRail);
  panel.append(viewGrid);
  panel.append(el('p', 'panel-hint', 'Right-click any button to pin it to the tool rail.'));

  panel.append(el('h2', 'gap', 'Lighting'));
  const envSelect = choices('Environment', [['off', 'Off (fixed lights)'], ...ENV_PRESETS]);
  const envRotate = input('range', { min: '-180', max: '180', value: '0', 'aria-label': 'Environment rotation' });
  const exposure = input('range', { min: '10', max: '400', value: '100', 'aria-label': 'Exposure' });
  const fixedLights = input('range', { min: '0', max: '200', value: '35', 'aria-label': 'Fixed lights' });
  const tone = choices('Tone mapping', [['reinhard', 'Reinhard'], ['aces', 'ACES filmic'], ['none', 'None']]);
  const envBackground = input('checkbox', { 'aria-label': 'Show environment' });
  const envBlur = input('range', { min: '0', max: '100', value: '35', 'aria-label': 'Background blur' });
  const envInput = input('file', { accept: '.hdr,image/png,image/jpeg,image/webp', 'aria-label': 'Choose an environment image' });
  envInput.hidden = true;
  panel.append(envInput);
  const keyAzimuth = input('range', { min: '-180', max: '180', value: '-37', 'aria-label': 'Key light direction' });
  const keyElevation = input('range', { min: '-10', max: '90', value: '41', 'aria-label': 'Key light height' });
  const keyIntensity = input('range', { min: '0', max: '300', value: '100', 'aria-label': 'Key light strength' });
  const keyColor = input('color', { value: '#fff8ef', 'aria-label': 'Key light colour' });
  const keyFollows = input('checkbox', { 'aria-label': 'Key light follows the view' });
  keyFollows.checked = true;
  const lightPick = choices('Light', [['0', 'Key light (casts shadows)']]);
  const shadows = input('checkbox', { 'aria-label': 'Shadows' });
  const groundShadow = input('checkbox', { 'aria-label': 'Ground shadow' });
  const postSsao = input('checkbox', { 'aria-label': 'Ambient occlusion' });
  const postBloom = input('checkbox', { 'aria-label': 'Bloom' });
  const postDof = input('checkbox', { 'aria-label': 'Depth of field' });
  const postVignette = input('checkbox', { 'aria-label': 'Vignette' });
  const postGrain = input('checkbox', { 'aria-label': 'Film grain' });
  const lightFields = el('div', 'fields');
  lightFields.append(field('Environment', envSelect), field('Rotate', envRotate), field('Exposure', exposure), field('Fixed lights', fixedLights), field('Tone mapping', tone), field('Show environment', envBackground), field('Background blur', envBlur), field('Light', lightPick), field('Light direction', keyAzimuth), field('Light height', keyElevation), field('Light strength', keyIntensity), field('Light colour', keyColor), field('Follows the view', keyFollows), field('Shadows', shadows), field('Ground shadow', groundShadow), field('Ambient occlusion', postSsao), field('Bloom', postBloom), field('Depth of field', postDof), field('Vignette', postVignette), field('Film grain', postGrain));
  panel.append(lightFields);
  const lightGrid = el('div', 'add-grid two after-fields');
  const envLoad = el('button', 'btn', 'Load HDRI');
  envLoad.type = 'button';
  const lightAdd = el('button', 'btn', 'Add light');
  lightAdd.type = 'button';
  const lightRemove = el('button', 'btn', 'Remove light');
  lightRemove.type = 'button';
  lightRemove.disabled = true;
  lightGrid.classList.remove('two');
  lightGrid.append(envLoad, lightAdd, lightRemove);
  panel.append(lightGrid);

  panel.append(el('h2', 'gap', 'Keys'));
  const keyGrid = el('div', 'add-grid two');
  const keysButton = el('button', 'btn', 'Keyboard shortcuts');
  keysButton.type = 'button';
  keysButton.dataset.cmd = 'app.shortcuts';
  keyGrid.append(keysButton);
  panel.append(keyGrid);

  panel.append(el('h2', 'gap', 'Plugins'));
  const pluginPick = choices('Plugin', [['', 'No plugins found']]);
  const pluginFields = el('div', 'fields');
  pluginFields.append(field('Plugin', pluginPick));
  panel.append(pluginFields);
  // The chosen plugin's own fields, drawn from the PARAMS it declares.
  const pluginParams = el('div', 'fields after-fields');
  panel.append(pluginParams);
  const pluginGrid = el('div', 'add-grid two after-fields');
  const pluginButtons = (
    [
      ['run', 'Run plugin'],
      ['reload', 'Reload list'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    pluginGrid.append(b);
    return b;
  });
  panel.append(pluginGrid);

  const message = el('p', 'panel-note', '');
  panel.append(message);

  panel.append(el('h2', 'gap', 'Active mesh'));
  const stats = el('dl', 'kv');
  const statVerts = el('dd', '', '0');
  const statTris = el('dd', '', '0');
  const statLevel = el('dd', '', 'none');
  stats.append(el('dt', '', 'Vertices'), statVerts, el('dt', '', 'Triangles'), statTris, el('dt', '', 'Level'), statLevel);
  panel.append(stats);

  panel.append(el('h2', 'gap', 'Engine'));
  const kv = el('dl', 'kv');
  const engineVersion = el('dd', '', '…');
  const statRenderer = el('dd', '', '…');
  const statFrame = el('dd', '', '…');
  kv.append(el('dt', '', 'App'), el('dd', '', VERSION), el('dt', '', 'Service'), engineVersion, el('dt', '', 'Renderer'), statRenderer, el('dt', '', 'Frame'), statFrame);
  panel.append(kv);

  panel.append(el('h2', 'gap', 'History'));
  const historyList = el('ul', 'history');
  historyList.setAttribute('aria-label', 'Undo history');
  panel.append(historyList);

  // The Scene list stays last: it grows and shrinks with the objects, and nothing below it may move.
  panel.append(el('h2', 'gap', 'Scene'));
  const objectList = el('ul', 'objects');
  objectList.setAttribute('aria-label', 'Objects');
  panel.append(objectList);

  const status = el('footer', 'status');
  const state = el('span', 'engine-state');
  const engineDot = el('i', 'dot');
  const engineText = el('span', '', 'connecting');
  state.append(engineDot, engineText);
  status.append(state);

  root.append(bar, rail, stage, panel, status);
  // Folds, panel side and size are layout only, so the window keeps them here rather than in the engine.
  const layout = new PanelLayout(root, panel);
  layout.apply();
  panelLeft.checked = layout.panelLeft;
  uiSize.value = layout.size;
  panelLeft.addEventListener('change', () => layout.setSide(panelLeft.checked));
  uiSize.addEventListener('change', () => layout.setSize(uiSize.value));
  const tag = (list: HTMLButtonElement[], group: string, by: (b: HTMLButtonElement) => string | undefined): void => {
    for (const b of list) {
      const spec = PANEL_CMDS[group][by(b) ?? ''];
      if (spec) b.dataset.cmd = spec.id;
    }
  };
  const action = (b: HTMLButtonElement): string | undefined => b.dataset.action;
  tag(fileButtons, 'file', action);
  tag(addButtons, 'add', (b) => b.dataset.kind);
  tag(buildButtons, 'build', action);
  tag(curveButtons, 'curve', action);
  tag(pathButtons, 'curve', action);
  tag(profileButtons, 'curve', action);
  tag(paintButtons, 'paint', action);
  tag(paintLayerButtons, 'paintLayer', action);
  tag(uvButtons, 'uv', action);
  tag(actionButtons, 'action', action);
  tag(topoButtons, 'topo', (b) => b.textContent ?? '');
  tag(objectButtons, 'object', action);
  tag(layerButtons, 'layer', action);
  tag(viewButtons, 'view', action);
  tag(pluginButtons, 'plugin', action);
  for (const b of brushButtons) b.dataset.cmd = `tool.brush.${b.dataset.brush ?? ''}`;
  envLoad.dataset.cmd = 'light.load_hdri';
  lightAdd.dataset.cmd = 'light.add';
  lightRemove.dataset.cmd = 'light.remove';
  primApply.dataset.cmd = 'add.apply_primitive';
  clearRail.dataset.cmd = 'view.clear_rail';
  layout.attachRail(rail, (text) => setText(message, text));
  return { engineDot, engineText, engineVersion, canvas, addButtons, objectList, statVerts, statTris, statLevel, statRenderer, statFrame, message, brushButtons, radius, strength, invert, symX, radial, actionButtons, dyntopo, detail, detailMode, topoButtons, resolution, quadTarget, paintColor, paintRough, paintMetal, paintOpacityCh, paintEmissive, paintMaterial, shading, paintButtons, bakeSize, fileButtons, fileInput, exportFormat, nodemode, objectButtons, deformAxis, deformAngle, layerSelect, layerStrength, layerButtons, skeleton, lathe, tubeRadius, buildButtons, tubeLine, lazy, fov, viewButtons, historyList, showGroups, trackball, insertKind, insertSize, arrayCount, reference, referenceInput, referenceOpacity, folderName, alpha, alphaAngle, strokeMode, alphaInput, alphaLoad, matcapInput, shadeActive, smoothPick, uvSeams, uvView, uvImage, uvEmpty, uvButtons, layout, panelLeft, uiSize, menubar, falloff, falloffCurve, falloffLine, falloffDots, pressureGamma, pressureStrength, pressureSize, curveButtons, curveInterp, curveClosed, curveRound, radiusMode, radiusEnd, pointRadius, pointTwist, curveOffset, curveGrid, curveChain, setPick, curveIntensity, tubeProfile, profileEditor, profileLine, profileDots, profileButtons, tubeCap, tubeHole, tubeSpiral, pathButtons, backdropMode, backdropTop, backdropBottom, backdropMid, backdropVignette, floorGrid, curveLine, curveHandles, paintLayer, paintBlend, paintOpacity, paintVisible, paintLayerButtons, envSelect, envRotate, exposure, fixedLights, tone, envBackground, envBlur, envInput, envLoad, postSsao, postBloom, postDof, postVignette, postGrain, gizmoAxes, gizmoUniform, latticeLines, latticePoints, primSize, primDetail, primHeight, primApply, noisePattern, noiseSize, noiseHeight, noiseOctaves, noiseGain, noiseStretch, keyAzimuth, keyElevation, keyIntensity, keyColor, keyFollows, shadows, groundShadow, lightPick, lightAdd, lightRemove, projectOpacity, projectInput, referencePlace, pluginPick, pluginParams, subsurface, transmission, pluginButtons, arrayLinked, vdmPick, textInput, fuseBlend, liveBoolean };
}

class SceneView {
  private rows = new Map<number, HTMLLIElement>();
  private scene: SceneSummary = { objects: [], active: null };

  constructor(private shell: Shell, private viewport: Viewport) {
    shell.objectList.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (row?.dataset.id) void this.run('select_object', { id: Number(row.dataset.id) });
    });
    shell.addButtons.forEach((b) => this.on(b, () => void this.add(b.dataset.kind ?? 'sphere')));
    shell.brushButtons.forEach((b) =>
      this.on(b, () => {
        this.brush = b.dataset.brush ?? 'draw';
        if (!SHAPE_TOOLS.has(this.brush) && !CLICK_TOOLS.has(this.brush) && this.brush !== 'orbit' && this.brush !== 'tube') this.lastBrush = this.brush;
        this.drawTools();
        scheduleChrome();
      }),
    );
    shell.actionButtons.forEach((b) =>
      this.on(b, () => {
        const action = b.dataset.action ?? 'undo';
        const mapped = ACTION_CALLS[action];
        if (mapped) void this.run(mapped[0], mapped[1]);
        else if (action === 'mask_color') void this.run('mask_by_color', { color: shell.paintColor.value, tolerance: 0.2 });
        else void this.run(action);
      }),
    );
    // The reference image sits over the view, or on an image plane in the scene (front, side or top) behind the model.
    const applyReference = (): void => {
      const opacity = Number(shell.referenceOpacity.value) / 100;
      shell.reference.style.opacity = String(opacity);
      const place = shell.referencePlace.value;
      const has = shell.reference.src.startsWith('blob:');
      shell.reference.hidden = !has || place !== 'screen';
      this.viewport.renderer.setReferencePlane(has && place !== 'screen' ? place : null, opacity);
      this.viewport.invalidate();
    };
    applyReference();
    shell.referenceOpacity.addEventListener('input', applyReference);
    shell.referencePlace.addEventListener('change', applyReference);
    shell.smoothPick.addEventListener('change', () => this.setSmoothPreview(Number(shell.smoothPick.value)));
    this.wireUvView();
    shell.liveBoolean.addEventListener('change', () => this.setLive(shell.liveBoolean.value));
    shell.referenceInput.addEventListener('change', () => {
      const file = shell.referenceInput.files?.[0];
      shell.referenceInput.value = '';
      if (!file) return;
      if (shell.reference.src.startsWith('blob:')) URL.revokeObjectURL(shell.reference.src);
      shell.reference.src = URL.createObjectURL(file);
      void createImageBitmap(file).then((bitmap) => {
        this.viewport.renderer.setReferenceImage(bitmap);
        bitmap.close();
        applyReference();
      });
      this.note(`Reference: ${file.name}`);
    });
    shell.viewButtons.forEach((b) =>
      this.on(b, () => {
        const action = b.dataset.action ?? 'show_all';
        if (action === 'reference') {
          shell.referenceInput.click();
          return;
        }
        if (action === 'clear_reference') {
          if (shell.reference.src.startsWith('blob:')) URL.revokeObjectURL(shell.reference.src);
          shell.reference.hidden = true;
          shell.reference.removeAttribute('src');
          this.viewport.renderer.setReferenceImage(null);
          this.viewport.invalidate();
          return;
        }
        if (action === 'matcap') {
          shell.matcapInput.click();
          return;
        }
        if (action === 'turntable') {
          this.note('Recording a turntable');
          void this.download(`/api/engine/turntable?frames=48&size=384&shading=${encodeURIComponent(shell.shading.value === 'pbr' ? 'pbr' : 'clay')}`, 'turntable.png');
          return;
        }
        if (action === 'spacemouse') {
          void this.connectSpaceMouse();
          return;
        }
        if (action === 'history_movie') {
          this.note('Recording the undo history');
          void this.download(`/api/engine/history_movie?fps=4&size=384&turn=90&shading=${encodeURIComponent(shell.shading.value === 'pbr' ? 'pbr' : 'clay')}`, 'history.png');
          return;
        }
        if (action === 'anim_key' || action === 'anim_record' || action === 'anim_clear') {
          this.animationAction(action);
          return;
        }
        const id = this.scene.active;
        if (action !== 'show_all' && id === null) return;
        void this.run('set_visibility', action === 'show_all' ? { action } : { action, ids: [id] });
      }),
    );
    shell.fov.addEventListener('input', () => this.viewport.setFov(Number(shell.fov.value)));
    shell.showGroups.addEventListener('change', () => {
      this.viewport.renderer.setShowGroups(shell.showGroups.checked);
      this.viewport.invalidate();
    });
    shell.trackball.addEventListener('change', () => this.viewport.camera.setTrackball(shell.trackball.checked));
    this.wireBackdrop();
    shell.canvas.addEventListener('dblclick', (e) => void this.pivot(e));
    shell.historyList.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (row?.dataset.done !== undefined) void this.run('history_go_to', { done: Number(row.dataset.done) });
    });
    // Under the smooth preview the drawn vertices are not the sculpted ones, so the stroke shows when it ends.
    this.strokes.onUpdate = (u) => {
      if (!this.smoothPreview) this.viewport.patchMesh(u);
    };
    this.strokes.onEnd = (r) => {
      // Under the preview the revision stays behind, so the refresh fetches the smoothed result.
      if (r && !this.smoothPreview) this.viewport.renderer.setRevision(r.id, r.revision);
      void this.refresh();
    };
    this.strokes.onError = (m) => this.note(m);
    this.strokes.onTopology = (id) => this.refetch(id);
    shell.topoButtons.forEach((b) =>
      this.on(b, () => {
        const args = JSON.parse(b.dataset.args ?? '{}') as Record<string, unknown>;
        if (b.dataset.action === 'remesh_voxel') args.resolution = Math.max(8, Math.min(320, Number(shell.resolution.value) || 128));
        if (b.dataset.action === 'quad_remesh') args.faces = Math.max(100, Math.min(400000, Number(shell.quadTarget.value) || 4000));
        void this.run(b.dataset.action ?? 'mesh_check', args);
      }),
    );
    shell.paintButtons.forEach((b) =>
      this.on(b, () => {
        const id = this.scene.active;
        if (id === null) return;
        const action = b.dataset.action;
        if (action === 'paint_clear') void this.run(action, { id });
        else if (action === 'uv_unwrap') void this.unwrapUvs(id);
        else if (action === 'bake_maps') void this.bake(id);
        else if (action === 'maps') void this.download(`/api/engine/maps/${id}`, 'maps.zip');
        else if (action === 'project_view' || action === 'project_tile') {
          this.projectMode = action === 'project_view' ? 'view' : 'triplanar';
          shell.projectInput.click();
        }
        else void this.run('paint_fill', { id, ...this.paintArgs(true) });
      }),
    );
    shell.shading.addEventListener('change', () => this.applyShading());
    shell.envSelect.addEventListener('change', () => void this.chooseEnvironment());
    for (const c of [shell.envRotate, shell.exposure, shell.fixedLights, shell.envBlur]) c.addEventListener('input', () => this.applyLighting());
    for (const c of [shell.shadows, shell.groundShadow]) c.addEventListener('change', () => this.applyLighting());
    // The light fields edit the light chosen under Light: the key, or an added one.
    const editLight = (): void => {
      const spec = this.lightSpecs[Number(shell.lightPick.value)] ?? this.lightSpecs[0];
      spec.azimuth = Number(shell.keyAzimuth.value);
      spec.elevation = Number(shell.keyElevation.value);
      spec.intensity = Number(shell.keyIntensity.value) / 100;
      spec.color = hexToLinear(shell.keyColor.value);
      spec.hex = shell.keyColor.value;
      spec.followsView = shell.keyFollows.checked;
      this.applyLighting();
    };
    for (const c of [shell.keyAzimuth, shell.keyElevation, shell.keyIntensity, shell.keyColor]) c.addEventListener('input', editLight);
    shell.keyFollows.addEventListener('change', editLight);
    shell.lightPick.addEventListener('change', () => this.showLight());
    this.on(shell.lightAdd, () => {
      if (this.lightSpecs.length >= 4) {
        this.note('Up to four lights: the key and three more');
        return;
      }
      // A new light starts opposite the key, dimmer and cool, so it reads as a fill or rim.
      const key = this.lightSpecs[0];
      this.lightSpecs.push({ azimuth: key.azimuth > 0 ? key.azimuth - 180 : key.azimuth + 180, elevation: 25, intensity: 0.4, color: hexToLinear('#dfe8ff'), hex: '#dfe8ff', followsView: key.followsView });
      this.patchLightList(this.lightSpecs.length - 1);
      this.applyLighting();
    });
    this.on(shell.lightRemove, () => {
      const i = Number(shell.lightPick.value);
      if (i <= 0) return;
      this.lightSpecs.splice(i, 1);
      this.patchLightList(Math.min(i, this.lightSpecs.length - 1));
      this.applyLighting();
    });
    for (const c of [shell.tone, shell.envBackground, shell.postSsao, shell.postBloom, shell.postDof, shell.postVignette, shell.postGrain]) c.addEventListener('change', () => this.applyLighting());
    this.on(shell.envLoad, () => shell.envInput.click());
    this.on(shell.primApply, () => void this.applyPrimitive());
    shell.projectInput.addEventListener('change', () => void this.projectImage());
    shell.pluginButtons.forEach((b) => this.on(b, () => void (b.dataset.action === 'run' ? this.runPlugin() : this.loadPlugins())));
    shell.pluginPick.addEventListener('change', () => this.drawPluginParams());
    // Subsurface and transmission belong to the object (saved with it); they show in lit (PBR) shading.
    for (const [control, key] of [[shell.subsurface, 'subsurface'], [shell.transmission, 'transmission']] as const) {
      control.addEventListener('change', () => {
        const id = this.scene.active;
        if (id === null) return;
        if (shell.shading.value !== 'pbr') {
          shell.shading.value = 'pbr';
          shell.shading.dispatchEvent(new Event('change'));
        }
        void this.run('set_material', { id, [key]: Number(control.value) / 100 });
      });
    }
    // A tube or lathe that has not been changed since it was made follows these fields.
    const editable = (kind: string): number | null => {
      const o = this.scene.objects.find((x) => x.id === this.scene.active);
      return o && o.editable_build === kind ? o.id : null;
    };
    this.wireProfile(() => {
      const id = editable('tube');
      if (id !== null) void this.run('edit_build', { id, profile: this.tubeArgs().profile });
    });
    shell.tubeRadius.addEventListener('change', () => {
      const id = editable('tube');
      if (id !== null) void this.run('edit_build', { id, radius: Math.max(0.005, Math.min(1, Number(shell.tubeRadius.value) || 0.05)) });
    });
    for (const control of [shell.tubeProfile, shell.tubeCap, shell.tubeHole, shell.tubeSpiral]) {
      control.addEventListener('change', () => {
        const id = editable('tube');
        if (id === null) return;
        const { profile, cap, hole, spiral } = this.tubeArgs();
        void this.run('edit_build', { id, profile, cap, hole, spiral });
      });
    }
    shell.lathe.addEventListener('change', () => {
      const id = editable('lathe');
      const preset = LATHES[shell.lathe.value];
      if (id !== null && preset) void this.run('edit_build', { id, profile: preset.profile });
    });
    void this.loadPlugins();
    shell.envInput.addEventListener('change', () => void this.loadEnvironment());
    shell.paintLayerButtons.forEach((b) => this.on(b, () => this.paintLayerAction(b.dataset.action ?? '')));
    shell.paintLayer.addEventListener('change', () => {
      const id = this.scene.active;
      if (id !== null) void this.run('paint_layer_select', { id, index: shell.paintLayer.value === '' ? null : Number(shell.paintLayer.value) });
    });
    const layerSetting = (args: Record<string, unknown>): void => {
      const o = this.scene.objects.find((x) => x.id === this.scene.active);
      if (o && o.active_paint_layer !== null && o.active_paint_layer !== undefined) void this.run('paint_layer_set', { id: o.id, index: o.active_paint_layer, ...args });
    };
    shell.paintBlend.addEventListener('change', () => layerSetting({ blend: shell.paintBlend.value }));
    shell.paintOpacity.addEventListener('change', () => layerSetting({ opacity: Number(shell.paintOpacity.value) / 100 }));
    shell.paintVisible.addEventListener('change', () => layerSetting({ visible: shell.paintVisible.checked }));
    shell.curveButtons.forEach((b) =>
      this.on(b, () => {
        if (b.dataset.action === 'alpha') shell.alphaInput.click();
        else if (b.dataset.action === 'apply_curve') void this.applyCurve();
        else this.clearCurve();
      }),
    );
    shell.pathButtons.forEach((b) =>
      this.on(b, () => {
        const a = b.dataset.action;
        if (a === 'tube_curve') void this.tubeFromCurve();
        else this.setCorners(a === 'all_sharp' ? 'sharp' : a === 'all_smooth' ? 'smooth' : 'toggle');
      }),
    );
    shell.curveRound.addEventListener('input', () => this.setCorners('round'));
    shell.pointRadius.addEventListener('change', () => this.setPointShape());
    shell.pointTwist.addEventListener('change', () => this.setPointShape());
    shell.curveInterp.addEventListener('change', () => void this.resample());
    shell.curveClosed.addEventListener('change', () => void this.resample());
    shell.falloff.addEventListener('change', () => {
      const preset = FALLOFFS[shell.falloff.value];
      if (preset) this.falloffWeights = this.falloffWeights.map((_, i) => preset(i / (FALLOFF_POINTS - 1)));
      this.drawFalloff();
    });
    const dragFalloff = (e: PointerEvent): void => {
      if (e.type === 'pointermove' && !shell.falloffCurve.hasPointerCapture(e.pointerId)) return;
      const box = shell.falloffCurve.getBoundingClientRect();
      const fx = (e.clientX - box.left) / Math.max(1, box.width);
      const fy = (e.clientY - box.top) / Math.max(1, box.height);
      if (e.type === 'pointerdown') {
        shell.falloffCurve.setPointerCapture(e.pointerId);
        this.falloffDrag = Math.max(0, Math.min(FALLOFF_POINTS - 1, Math.round(((fx * 200 - 4) / 192) * (FALLOFF_POINTS - 1))));
      }
      if (this.falloffDrag === null) return;
      this.falloffWeights[this.falloffDrag] = Math.max(0, Math.min(1, (60 - fy * 64) / 56));
      shell.falloff.value = 'custom';
      this.drawFalloff();
    };
    shell.falloffCurve.addEventListener('pointerdown', dragFalloff);
    shell.falloffCurve.addEventListener('pointermove', dragFalloff);
    shell.falloffCurve.addEventListener('pointerup', () => (this.falloffDrag = null));
    this.drawFalloff();
    shell.alphaInput.addEventListener('change', () => void this.loadAlpha());
    shell.matcapInput.addEventListener('change', () => void this.loadMatcap());
    void this.loadCatalogue();
    shell.fileButtons.forEach((b) => this.on(b, () => void this.fileAction(b.dataset.action ?? '')));
    if (this.host.embedded) {
      const send = shell.fileButtons.find((b) => b.dataset.action === 'nodemode');
      if (send) setText(send, 'Send to canvas');
      const row = shell.nodemode.closest('label');
      if (row) row.hidden = true;
    }
    shell.nodemode.addEventListener('change', () => {
      try {
        localStorage.setItem(NODEMODE_KEY, shell.nodemode.value.trim());
      } catch {
        // storage blocked; the address lasts for this session
      }
    });
    shell.fileInput.addEventListener('change', () => void this.uploadChosen());
    shell.objectButtons.forEach((b) => this.on(b, () => this.objectAction(b.dataset.action ?? '')));
    shell.layerButtons.forEach((b) => this.on(b, () => this.layerAction(b.dataset.action ?? '')));
    shell.layerSelect.addEventListener('change', () => {
      const id = this.scene.active;
      if (id === null) return;
      const v = shell.layerSelect.value;
      void this.run('layer_select', { id, index: v === '' ? null : Number(v) });
    });
    shell.layerStrength.addEventListener('change', () => {
      const o = this.scene.objects.find((x) => x.id === this.scene.active);
      if (!o || o.active_layer === null) return;
      void this.run('layer_set_strength', { id: o.id, index: o.active_layer, strength: Number(shell.layerStrength.value) / 100 });
    });
    shell.buildButtons.forEach((b) => this.on(b, () => void this.build(b.dataset.action ?? '')));
    const trace = (e: PointerEvent): void => {
      if (this.shape) {
        const p: [number, number] = [e.offsetX, e.offsetY];
        if (this.shape.kind === 'gizmo' || this.shape.kind === 'lattice') {
          this.handleTool(this.shape, p);
          return;
        }
        if (this.shape.kind === 'curve' && this.shape.pts.length === 0 && this.curve) {
          // Pressing on a handle of the waiting curve moves that handle instead of drawing a new curve.
          const near = this.curveScreen().findIndex((q) => q !== null && Math.hypot(q[0] - p[0], q[1] - p[1]) < 10);
          if (near >= 0) this.shape.edit = near;
          this.curveAlt = e.altKey;
        }
        if (this.shape.kind === 'dragdot' || (this.shape.kind === 'curve' && this.shape.edit !== undefined)) this.shape.pts = [p];
        else if (this.shape.kind === 'mask_box' || this.shape.kind === 'stamp') this.shape.pts = [this.shape.pts[0] ?? p, p];
        else this.shape.pts.push(p);
        const pts = this.shape.pts;
        if (this.shape.kind === 'dragdot') {
          // Drag dot: one dab of the brush's size that follows the pointer and lands where it is let go.
          const r = Number(shell.radius.value);
          shell.tubeLine.setAttribute('points', Array.from({ length: 33 }, (_, i) => `${(p[0] + r * Math.cos((i / 32) * Math.PI * 2)).toFixed(0)},${(p[1] + r * Math.sin((i / 32) * Math.PI * 2)).toFixed(0)}`).join(' '));
          return;
        }
        if (this.shape.kind === 'curve') {
          if (this.shape.edit !== undefined) this.drawCurve(this.shape.edit, p);
          else shell.tubeLine.setAttribute('points', pts.map((q) => `${q[0].toFixed(0)},${q[1].toFixed(0)}`).join(' '));
          return;
        }
        if (this.shape.kind === 'stamp') {
          // The stamp's size and turn: a circle around the first point through the pointer, and its radius line.
          const [c, q] = [pts[0], pts[pts.length - 1]];
          const r = Math.hypot(q[0] - c[0], q[1] - c[1]);
          const a0 = Math.atan2(q[1] - c[1], q[0] - c[0]);
          const ring = Array.from({ length: 33 }, (_, i) => `${(c[0] + r * Math.cos(a0 + (i / 32) * Math.PI * 2)).toFixed(0)},${(c[1] + r * Math.sin(a0 + (i / 32) * Math.PI * 2)).toFixed(0)}`);
          shell.tubeLine.setAttribute('points', [`${c[0].toFixed(0)},${c[1].toFixed(0)}`, ...ring].join(' '));
          return;
        }
        const outline = this.shape.kind === 'mask_box' && pts.length === 2 ? [pts[0], [pts[1][0], pts[0][1]], pts[1], [pts[0][0], pts[1][1]], pts[0]] : [...pts, pts[0]];
        shell.tubeLine.setAttribute('points', outline.map((q) => `${q[0].toFixed(0)},${q[1].toFixed(0)}`).join(' '));
        return;
      }
      if (!this.tube) return;
      this.tube.screen.push(`${e.offsetX.toFixed(0)},${e.offsetY.toFixed(0)}`);
      shell.tubeLine.setAttribute('points', this.tube.screen.join(' '));
    };
    shell.canvas.addEventListener('pointerdown', trace);
    shell.canvas.addEventListener('pointermove', trace);
    this.strokes.warm();
    viewport.strokes = {
      enabled: () => this.brush === 'tube' || this.brush === 'silhouette' || (this.brush !== 'orbit' && this.scene.active !== null),
      begin: (ray, pressure, wpp, inv, tilt) => {
        if (this.brush === 'tube') this.tubeBegin(ray, wpp);
        else if (SHAPE_TOOLS.has(this.brush)) this.shape = { kind: this.brush, pts: [], subtract: inv, wpp };
        else if (CLICK_TOOLS.has(this.brush)) void this.clickTool(ray, wpp, inv);
        else if (this.scene.active !== null) {
          this.lastStroke = this.brush === 'curve' ? null : { rays: [{ o: ray.origin, d: ray.dir }], wpp, brush: this.brush };
          this.strokes.begin(this.scene.active, this.brushState(), wpp, inv, ray.origin, ray.dir, pressure, tilt);
        }
      },
      move: (ray, pressure, tilt) => {
        if (this.tube) this.tubeMove(ray);
        else if (!this.shape && !CLICK_TOOLS.has(this.brush)) {
          if (this.lastStroke && this.lastStroke.rays.length < 4000) this.lastStroke.rays.push({ o: ray.origin, d: ray.dir });
          this.strokes.move(ray.origin, ray.dir, pressure, tilt);
        }
      },
      end: () => {
        if (this.tube) void this.tubeEnd();
        else if (this.shape) void this.shapeEnd();
        else if (!CLICK_TOOLS.has(this.brush)) this.strokes.end();
      },
      // A press that misses the gizmo's handles or the lattice's points turns the view instead.
      missed: () => (this.shape?.miss ? true : this.tube || this.shape || CLICK_TOOLS.has(this.brush) ? false : this.strokes.missed()),
    };
  }

  private brush = 'draw';
  private strokes = new StrokeClient();
  private host = new HostLink(() => undefined, () => undefined);
  private tube: { points: Vec3[]; step: number; anchor: Vec3; normal: Vec3; screen: string[] } | null = null;

  private shape: { kind: string; pts: [number, number][]; subtract: boolean; wpp: number; edit?: number; miss?: boolean } | null = null;
  private gizmoDrag: GizmoDrag | null = null;
  /** A lattice point being dragged: its index (i + j * n + k * n * n) and world offset so far. */
  private latticeDrag: { index: number; offset: Vec3 } | null = null;
  /** The brush a curve stroke applies: the last sculpting or painting brush chosen before the Curve tool. */
  private lastBrush = 'draw';
  /** The rays of the last brush stroke, so it can be undone and brought back as an editable curve. */
  private lastStroke: { rays: { o: Vec3; d: Vec3 }[]; wpp: number; brush: string } | null = null;
  /** A curve stroke waiting to be applied: handles on the surface, and the world size of a pixel when it was drawn. */
  private curve: { handles: Vec3[]; wpp: number; sharp: boolean[]; round: number[]; radius: number[]; twist: number[]; samples: Vec3[] | null } | null = null;
  /** The handle last clicked (Corner round sets its fillet), and whether Alt was held when a handle was pressed. */
  private curvePick: number | null = null;
  private curveAlt = false;
  private resampling = false;
  private resampleAgain = false;
  private catalogue: CatalogueCommand[] = [];
  private handlers = new Map<string, () => void>();
  private contextEvent: MouseEvent | null = null;
  private pinTarget = '';
  private readonly prompt = new PromptDialog();

  private on(button: HTMLButtonElement, fn: () => void): void {
    const id = button.dataset.cmd;
    if (id) this.handlers.set(id, fn);
  }

  stroking(): boolean {
    return this.strokes.active() || this.tube !== null || this.shape !== null;
  }

  /** The backdrop controls: the theme's gradient until the person changes it, then their choice, kept in this browser. */
  private wireBackdrop(): void {
    const s = this.shell;
    const b = this.viewport.renderer.backdrop;
    const toHex = (c: [number, number, number]): string => `#${c.map((v) => Math.round(v * 255).toString(16).padStart(2, '0')).join('')}`;
    let saved: Partial<{ mode: string; top: string; bottom: string; mid: number; vignette: number; grid: boolean }> = {};
    try {
      saved = JSON.parse(localStorage.getItem(BACKDROP_KEY) ?? '{}') as typeof saved;
    } catch {
      saved = {};
    }
    s.backdropMode.value = saved.mode === 'color' ? 'color' : 'gradient';
    s.backdropTop.value = saved.top ?? toHex(b.top);
    s.backdropBottom.value = saved.bottom ?? toHex(b.bottom);
    s.backdropMid.value = String(saved.mid ?? Math.round(b.mid * 100));
    s.backdropVignette.value = String(saved.vignette ?? Math.round(b.vignette * 100));
    s.floorGrid.checked = saved.grid ?? b.grid;
    const apply = (persist: boolean): void => {
      b.mode = s.backdropMode.value === 'color' ? 'color' : 'gradient';
      b.top = hexColor(s.backdropTop.value) ?? b.top;
      b.bottom = hexColor(s.backdropBottom.value) ?? b.bottom;
      b.mid = Number(s.backdropMid.value) / 100;
      b.vignette = Number(s.backdropVignette.value) / 100;
      b.grid = s.floorGrid.checked;
      this.viewport.renderer.setClearColor(s.backdropBottom.value);
      this.viewport.invalidate();
      scheduleChrome();
      if (!persist) return;
      try {
        localStorage.setItem(BACKDROP_KEY, JSON.stringify({ mode: b.mode, top: s.backdropTop.value, bottom: s.backdropBottom.value, mid: Number(s.backdropMid.value), vignette: Number(s.backdropVignette.value), grid: b.grid }));
      } catch {
        // storage blocked; the choice lasts for this session
      }
    };
    for (const c of [s.backdropTop, s.backdropBottom, s.backdropMid, s.backdropVignette]) c.addEventListener('input', () => apply(true));
    for (const c of [s.backdropMode, s.floorGrid]) c.addEventListener('change', () => apply(true));
    apply(false);
  }

  /** What an agent asked from the dock should know about the window: the active object and the scene's size. */
  agentContext(): string {
    const active = this.scene.objects.find((o) => o.id === this.scene.active);
    const count = this.scene.objects.length;
    if (!active) return count ? `formsculptor window: ${count} objects, none active` : 'formsculptor window: empty scene';
    return `formsculptor window: active object id ${active.id} named ${JSON.stringify(active.name)}, ${count} objects in the scene`;
  }

  agentNote(): string {
    const active = this.scene.objects.find((o) => o.id === this.scene.active);
    return active ? `with ${active.name} active` : '';
  }

  async selectObject(id: number): Promise<void> {
    if (!this.scene.objects.some((o) => o.id === id)) return;
    await this.run('select_object', { id });
  }

  agentNotice(tool: string): void {
    if (tool) this.note(`Agent · ${tool.replace(/_/g, ' ')}`);
  }

  private async shapeEnd(): Promise<void> {
    const shape = this.shape;
    this.shape = null;
    this.shell.tubeLine.setAttribute('points', '');
    const id = this.scene.active;
    if (shape?.kind === 'stamp') {
      await this.stampEnd(shape.pts, shape.wpp, shape.subtract);
      return;
    }
    if (shape?.kind === 'dragdot') {
      const q = shape.pts[shape.pts.length - 1];
      if (q) await this.stampEnd([q, q], shape.wpp, shape.subtract);
      return;
    }
    if (shape?.kind === 'curve') {
      await this.curveEnd(shape.pts, shape.wpp, shape.edit);
      return;
    }
    if (shape?.kind === 'gizmo') {
      await this.gizmoEnd();
      return;
    }
    if (shape?.kind === 'insert_brush' || shape?.kind === 'group_brush') {
      await this.strokeTool(shape.pts, shape.wpp, shape.kind);
      return;
    }
    if (shape?.kind === 'cut' || shape?.kind === 'trim_shape') {
      await this.cutEnd(shape.pts, shape.kind === 'trim_shape');
      return;
    }
    if (shape?.kind === 'silhouette') {
      this.silhouetteEnd(shape.pts);
      return;
    }
    if (shape?.kind === 'lattice') {
      await this.latticeEnd();
      return;
    }
    const box = shape?.kind === 'mask_box';
    if (!shape || id === null || shape.pts.length < (box ? 2 : 3)) {
      this.note(box ? 'Drag a box over the area to mask' : 'Draw around the area to mask');
      return;
    }
    const w = Math.max(1, this.shell.canvas.clientWidth);
    const h = Math.max(1, this.shell.canvas.clientHeight);
    const points = shape.pts.map(([x, y]) => [x / w, y / h]);
    const viewProj = Array.from(this.viewport.camera.viewProjection());
    await this.run('mask_shape', { id, shape: box ? 'box' : 'lasso', points, view_proj: viewProj, op: shape.subtract ? 'subtract' : 'add' });
  }

  /** Drag stamp: one dab of the alpha where the drag began, as wide as the drag and turned to face it. */
  private async stampEnd(pts: [number, number][], wpp: number, invert: boolean): Promise<void> {
    const id = this.scene.active;
    if (id === null || pts.length === 0) return;
    const [c, q] = [pts[0], pts[pts.length - 1]];
    const rect = this.shell.canvas.getBoundingClientRect();
    const ray = this.viewport.rayAtClient(rect.left + c[0], rect.top + c[1]);
    if (!ray) return;
    const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
    if (!hit.ok || !hit.result.hit || hit.result.id !== id || !hit.result.point) {
      this.note('Start the stamp on the active object');
      return;
    }
    const dragPx = Math.hypot(q[0] - c[0], q[1] - c[1]);
    const px = dragPx >= 3 ? dragPx : Number(this.shell.radius.value);
    // Dragging straight up keeps the alpha upright; any other direction turns it to face the drag.
    const turn = dragPx >= 3 ? (Math.atan2(q[0] - c[0], c[1] - q[1]) * 180) / Math.PI : 0;
    const args = this.brushArgs(Math.max(1e-4, px * wpp), invert);
    await this.run('sculpt_stroke', { ...args, id, points: [hit.result.point], alpha: this.shell.alpha.value, alpha_angle: (Number(this.shell.alphaAngle.value) || 0) + turn, alpha_up: this.viewport.camera.screenUp() });
  }

  /** The engine's brush arguments for a stroke the window sends whole (stamps, drag dots, curves): the last
   * sculpting brush with every Brush section setting, as the live stroke socket would send them. */
  private brushArgs(radius: number, ctrl: boolean): Record<string, unknown> {
    const s = this.brushState();
    return {
      ...(s.alpha ?? {}),
      ...(this.lastBrush === 'paint' ? this.paintArgs(this.shell.paintMaterial.checked) : {}),
      brush: this.lastBrush,
      radius,
      strength: s.strength,
      invert: s.invert !== ctrl,
      falloff: s.falloff,
      stroke: s.stroke,
      ...(s.falloffCurve ? { falloff_curve: s.falloffCurve } : {}),
      symmetry: s.symmetry,
      pressure_gamma: s.pressure.gamma,
      pressure_strength: s.pressure.strength,
      pressure_size: s.pressure.size,
    };
  }

  /** Where each curve handle falls on the canvas now, or null behind the camera. */
  private curveScreen(): ([number, number] | null)[] {
    const c = this.curve;
    if (!c) return [];
    const m = this.viewport.camera.viewProjection();
    const w = this.shell.canvas.clientWidth;
    const h = this.shell.canvas.clientHeight;
    return c.handles.map(([x, y, z]) => {
      const cw = m[3] * x + m[7] * y + m[11] * z + m[15];
      if (cw <= 1e-6) return null;
      const cx = (m[0] * x + m[4] * y + m[8] * z + m[12]) / cw;
      const cy = (m[1] * x + m[5] * y + m[9] * z + m[13]) / cw;
      return [(cx * 0.5 + 0.5) * w, (0.5 - cy * 0.5) * h];
    });
  }

  /** Points along a Catmull-Rom spline through the handles, `per` per span (screen or world). */
  private static spline<T extends number[]>(pts: T[], per: number): T[] {
    if (pts.length < 2) return pts.slice();
    const out: T[] = [];
    for (let i = 0; i < pts.length - 1; i++) {
      const p0 = pts[Math.max(0, i - 1)];
      const p1 = pts[i];
      const p2 = pts[i + 1];
      const p3 = pts[Math.min(pts.length - 1, i + 2)];
      for (let k = 0; k < per; k++) {
        const t = k / per;
        const t2 = t * t;
        const t3 = t2 * t;
        out.push(p1.map((_, d) => 0.5 * (2 * p1[d] + (p2[d] - p0[d]) * t + (2 * p0[d] - 5 * p1[d] + 4 * p2[d] - p3[d]) * t2 + (3 * p1[d] - p0[d] - 3 * p2[d] + p3[d]) * t3)) as T);
      }
    }
    out.push(pts[pts.length - 1].slice() as T);
    return out;
  }

  /** Redraws the waiting curve over the view; while a handle is dragged it is drawn at the pointer. */
  drawCurve(moving?: number, at?: [number, number]): void {
    const screen = this.curveScreen();
    if (moving !== undefined && at) screen[moving] = at;
    const shown = screen.filter((q): q is [number, number] => q !== null);
    if (shown.length < 2) {
      setAttr(this.shell.curveLine, 'points', '');
      setAttr(this.shell.curveHandles, 'd', '');
      return;
    }
    const c = this.curve;
    const project = this.projector();
    const sampled = moving === undefined && c?.samples ? c.samples.map(project).filter((q): q is [number, number] => q !== null) : null;
    const line = sampled && sampled.length > 1 ? sampled : SceneView.spline(shown, 8);
    setAttr(this.shell.curveLine, 'points', line.map((q) => `${q[0].toFixed(1)},${q[1].toFixed(1)}`).join(' '));
    const marks = screen.map((q, i) => {
      if (!q) return '';
      const r = i === this.curvePick ? 6 : 4;
      const y = q[1].toFixed(1);
      return c?.sharp[i] ? `M${(q[0] - r).toFixed(1)},${(q[1] - r).toFixed(1)}h${2 * r}v${2 * r}h${-2 * r}z` : `M${(q[0] - r).toFixed(1)},${y}a${r},${r} 0 1,0 ${2 * r},0a${r},${r} 0 1,0 ${-2 * r},0`;
    });
    setAttr(this.shell.curveHandles, 'd', marks.join(''));
  }

  /** The waiting curve's handles as the engine's curve points: corners where marked, with their fillet radius. */
  private curvePoints(): Record<string, unknown>[] {
    const c = this.curve;
    if (!c) return [];
    const base = Math.max(0.005, Math.min(1, Number(this.shell.tubeRadius.value) || 0.05));
    return c.handles.map((p, i) => ({ p, sharp: c.sharp[i], round: c.sharp[i] ? c.round[i] : 0, radius: base * c.radius[i], twist: c.twist[i] }));
  }

  private curveOptions(): { interp: string; closed: boolean } {
    return { interp: this.shell.curveInterp.value, closed: this.shell.curveClosed.checked };
  }

  /** Asks the engine to sample the waiting curve, so the line drawn is the line a stroke or tube follows. */
  private async resample(): Promise<void> {
    if (this.resampling) {
      this.resampleAgain = true;
      return;
    }
    this.resampling = true;
    try {
      do {
        this.resampleAgain = false;
        const c = this.curve;
        if (!c) break;
        const reply = await dispatch<{ positions: Vec3[] }>('sample_curve', { points: this.curvePoints(), ...this.curveOptions(), per_span: 12 });
        if (this.curve !== c) continue;
        const pts = reply.ok ? reply.result.positions : null;
        c.samples = pts && this.shell.curveClosed.checked && pts.length > 2 ? [...pts, pts[0]] : pts;
        this.drawCurve();
      } while (this.resampleAgain);
    } finally {
      this.resampling = false;
    }
  }

  /** Corner round as a world radius: a share of the curve's mean span, so the slider means the same on any size. */
  private roundWorld(): number {
    const c = this.curve;
    if (!c || c.handles.length < 2) return 0;
    let total = 0;
    for (let i = 1; i < c.handles.length; i++) total += distance3(c.handles[i - 1], c.handles[i]);
    return ((total / (c.handles.length - 1)) * Number(this.shell.curveRound.value)) / 200;
  }

  private setCorners(which: 'toggle' | 'sharp' | 'smooth' | 'round'): void {
    const c = this.curve;
    if (!c) {
      this.note('Draw a curve with the Curve stroke tool first');
      return;
    }
    const round = this.roundWorld();
    const pick = this.curvePick;
    if (which === 'toggle') {
      if (pick === null) {
        this.note('Click a curve handle first, or Alt-click it');
        return;
      }
      c.sharp[pick] = !c.sharp[pick];
      c.round[pick] = c.sharp[pick] ? round : 0;
    } else if (which === 'round') {
      for (let i = 0; i < c.handles.length; i++) if (c.sharp[i] && (pick === null || !c.sharp[pick] || i === pick)) c.round[i] = round;
    } else {
      c.sharp.fill(which === 'sharp');
      c.round.fill(which === 'sharp' ? round : 0);
    }
    this.drawCurve();
    void this.resample();
  }

  /** Builds a tube along the waiting curve with its corners and the tube settings. */
  private async tubeFromCurve(): Promise<void> {
    if (!this.curve) {
      this.note('Draw a curve with the Curve stroke tool first');
      return;
    }
    await this.run('curve_tube', { points: this.curvePoints(), ...this.curveOptions(), ...this.tubeArgs() });
  }

  private tubeArgs(): Record<string, unknown> {
    const s = this.shell;
    const radius = Math.max(0.005, Math.min(1, Number(s.tubeRadius.value) || 0.05));
    return {
      radius,
      radius_mode: s.radiusMode.value,
      radius_end: radius * Math.max(0.05, Number(s.radiusEnd.value) / 100),
      profile: s.tubeProfile.value === 'custom' || s.tubeProfile.value.startsWith('saved:') ? this.profilePts.map((q) => ({ p: q.p, sharp: q.sharp })) : s.tubeProfile.value,
      cap: s.tubeCap.value,
      hole: Math.max(0, Number(s.tubeHole.value) || 0),
      spiral: Math.max(0, Number(s.tubeSpiral.value) || 0),
    };
  }

  /** Outlines for the shadow box, in world units: front (x, y), side (z, y), top (x, z). */
  private silhouettes: { front?: number[][]; side?: number[][]; top?: number[][] } = {};

  /** Timeline keys: the camera's turn and height and the active object's layer strengths, one key every 1.5 seconds. */
  private animKeys: Record<string, unknown>[] = [];

  private animationAction(action: string): void {
    if (action === 'anim_clear') {
      this.animKeys = [];
      this.note('Animation keys cleared');
      return;
    }
    if (action === 'anim_key') {
      const cam = this.viewport.camera;
      const key: Record<string, unknown> = { time: this.animKeys.length * 1.5, yaw: (cam.yaw * 180) / Math.PI, pitch: (cam.pitch * 180) / Math.PI };
      const active = this.scene.objects.find((o) => o.id === this.scene.active);
      if (active && active.layers.length > 0) key.layers = active.layers.map((l, layer) => ({ id: active.id, layer, strength: l.strength }));
      // Unwind the turn so the camera goes the short way round from the last key.
      const last = this.animKeys[this.animKeys.length - 1];
      if (last && typeof last.yaw === 'number') {
        let y = key.yaw as number;
        while (y - last.yaw > 180) y -= 360;
        while (y - last.yaw < -180) y += 360;
        key.yaw = y;
      }
      this.animKeys.push(key);
      this.note(`Key ${this.animKeys.length} at ${key.time as number} s; turn the view or change layer strengths, then add another`);
      return;
    }
    if (this.animKeys.length < 2) {
      this.note('Add at least two keys first');
      return;
    }
    this.note('Recording the animation');
    const shading = this.shell.shading.value === 'pbr' ? 'pbr' : 'clay';
    void this.download(`/api/engine/animation?fps=24&size=384&shading=${shading}&keys=${encodeURIComponent(JSON.stringify(this.animKeys))}`, 'animation.png');
  }

  /** Insert brush or group brush: the stroke's points picked on the surface, then one command for the whole stroke. */
  private async strokeTool(pts: [number, number][], wpp: number, kind: string): Promise<void> {
    const id = this.scene.active;
    if (pts.length < 2) return;
    const rect = this.shell.canvas.getBoundingClientRect();
    const hits: { point: Vec3; normal: Vec3 }[] = [];
    let lastAt: [number, number] | null = null;
    for (const p of pts) {
      if (lastAt && Math.hypot(p[0] - lastAt[0], p[1] - lastAt[1]) < 6) continue;
      lastAt = p;
      const ray = this.viewport.rayAtClient(rect.left + p[0], rect.top + p[1]);
      if (!ray) continue;
      const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3; normal?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
      if (hit.ok && hit.result.hit && hit.result.point && hit.result.normal && (kind === 'insert_brush' || hit.result.id === id)) hits.push({ point: hit.result.point, normal: hit.result.normal });
      if (hits.length >= 256) break;
    }
    if (hits.length === 0) {
      this.note(kind === 'insert_brush' ? 'Draw the stroke over a surface' : 'Draw over the active object');
      return;
    }
    if (kind === 'group_brush') {
      if (id === null) return;
      await this.run('group_paint', { id, points: hits.map((h) => h.point), radius: Math.max(1e-4, Number(this.shell.radius.value) * wpp) });
      return;
    }
    const size = Math.max(0.01, Math.min(10, Number(this.shell.insertSize.value) || 0.2));
    const args: Record<string, unknown> = { path: hits.map((h) => [...h.point, ...h.normal]), size };
    if (this.shell.insertKind.value === 'previous') {
      if (this.previous === null) {
        this.note('Select the object to insert first, then the one to insert onto');
        return;
      }
      args.source_id = this.previous;
    } else args.kind = this.shell.insertKind.value;
    await this.run('insert_along', args);
  }

  /** Lasso cut or trim: the outline, taken onto the plane through the orbit centre facing the view, cut straight through the active object along the view. */
  private async cutEnd(pts: [number, number][], keepInside: boolean): Promise<void> {
    const id = this.scene.active;
    if (id === null || pts.length < 3) {
      this.note(keepInside ? 'Draw around the part to keep' : 'Draw around the part to cut away');
      return;
    }
    const cam = this.viewport.camera;
    const forward = normalize(sub(cam.target, cam.eye()));
    const rect = this.shell.canvas.getBoundingClientRect();
    const step = Math.max(1, Math.floor(pts.length / 512));
    const outline: Vec3[] = [];
    for (let i = 0; i < pts.length; i += step) {
      const ray = this.viewport.rayAtClient(rect.left + pts[i][0], rect.top + pts[i][1]);
      if (!ray) continue;
      const along = dot(ray.dir, forward);
      if (Math.abs(along) < 1e-6) continue;
      outline.push(add(ray.origin, scale(ray.dir, dot(sub(cam.target, ray.origin), forward) / along)));
    }
    const resolution = Math.max(32, Math.min(320, Number(this.shell.resolution.value) || 128));
    await this.run('cut_shape', { id, outline, direction: forward, keep: keepInside ? 'inside' : 'outside', resolution });
  }

  /** Stores a drawn outline as the front, side or top silhouette, by the axis the view looks along most. */
  private silhouetteEnd(pts: [number, number][]): void {
    if (pts.length < 3) {
      this.note('Draw around the shape you want seen from this view');
      return;
    }
    const cam = this.viewport.camera;
    const f = sub(cam.target, cam.eye());
    const a = [0, 1, 2].reduce((best, i) => (Math.abs(f[i]) > Math.abs(f[best]) ? i : best), 0);
    const rect = this.shell.canvas.getBoundingClientRect();
    const step = Math.max(1, Math.floor(pts.length / 256));
    const world: Vec3[] = [];
    for (let i = 0; i < pts.length; i += step) {
      const ray = this.viewport.rayAtClient(rect.left + pts[i][0], rect.top + pts[i][1]);
      if (!ray || Math.abs(ray.dir[a]) < 1e-6) continue;
      const t = (cam.target[a] - ray.origin[a]) / ray.dir[a];
      world.push(add(ray.origin, scale(ray.dir, t)));
    }
    const which = a === 2 ? 'front' : a === 0 ? 'side' : 'top';
    this.silhouettes[which] = world.map((p) => (a === 2 ? [p[0], p[1]] : a === 0 ? [p[2], p[1]] : [p[0], p[2]]));
    const have = (['front', 'side', 'top'] as const).filter((k) => this.silhouettes[k]);
    this.note(`${which[0].toUpperCase()}${which.slice(1)} outline kept (${have.join(', ')}); ${have.length < 2 ? 'draw one more from another view' : 'Build shadow box when ready'}`);
  }

  /** Projects world points to canvas pixels with the current camera (null behind it). */
  private projector(): (p: Vec3) => [number, number] | null {
    const m = this.viewport.camera.viewProjection();
    const w = this.shell.canvas.clientWidth;
    const h = this.shell.canvas.clientHeight;
    return ([x, y, z]) => {
      const cw = m[3] * x + m[7] * y + m[11] * z + m[15];
      if (cw <= 1e-6) return null;
      return [((m[0] * x + m[4] * y + m[8] * z + m[12]) / cw * 0.5 + 0.5) * w, (0.5 - ((m[1] * x + m[5] * y + m[9] * z + m[13]) / cw) * 0.5) * h];
    };
  }

  private activeObject(): ObjectSummary | undefined {
    return this.scene.objects.find((o) => o.id === this.scene.active && o.visible);
  }

  /** The gizmo's centre (the bounds centre, as transform_object turns and scales about it) and arrow length. */
  private gizmoFrame(o: ObjectSummary): { center: Vec3; len: number } {
    const { min, max } = o.bounds;
    const center: Vec3 = [(min[0] + max[0]) / 2, (min[1] + max[1]) / 2, (min[2] + max[2]) / 2];
    return { center, len: this.viewport.camera.worldPerPixel(this.shell.canvas.clientHeight) * 90 };
  }

  /** The lattice's control points around an object, as lattice_deform places them (bounds grown by 2 percent). */
  private latticePoints(o: ObjectSummary): Vec3[] {
    const { min, max } = o.bounds;
    const pad = Math.max(max[0] - min[0], max[1] - min[1], max[2] - min[2], 1e-6) * 0.02;
    const lo = min.map((v) => v - pad);
    const hi = max.map((v) => v + pad);
    const n = LATTICE_DIV;
    const out: Vec3[] = [];
    for (let k = 0; k < n; k++)
      for (let j = 0; j < n; j++)
        for (let i = 0; i < n; i++) out.push([lo[0] + ((hi[0] - lo[0]) * i) / (n - 1), lo[1] + ((hi[1] - lo[1]) * j) / (n - 1), lo[2] + ((hi[2] - lo[2]) * k) / (n - 1)]);
    return out;
  }

  /** Presses and drags with the Transform and Lattice tools. */
  private handleTool(shape: NonNullable<SceneView['shape']>, p: [number, number]): void {
    const o = this.activeObject();
    if (!o) {
      shape.miss = true;
      return;
    }
    const project = this.projector();
    const first = shape.pts.length === 0;
    shape.pts.push(p);
    if (shape.kind === 'gizmo') {
      const { center, len } = this.gizmoFrame(o);
      if (first) {
        const handle = Gizmo.hit(p, center, len, project);
        if (!handle) {
          shape.miss = true;
          return;
        }
        this.gizmoDrag = { handle, start: p, translate: [0, 0, 0], degrees: 0, scale: [1, 1, 1] };
        return;
      }
      const drag = this.gizmoDrag;
      if (!drag) return;
      const eye = this.viewport.camera.eye();
      Gizmo.update(drag, p, center, len, project, normalize(sub(eye, center)));
      this.viewport.renderer.setPreview(o.id, Gizmo.matrix(drag, center));
      this.viewport.invalidate();
      return;
    }
    const points = this.latticePoints(o);
    if (first) {
      const index = points.findIndex((q) => {
        const s = project(q);
        return s !== null && Math.hypot(s[0] - p[0], s[1] - p[1]) <= 9;
      });
      if (index < 0) {
        shape.miss = true;
        return;
      }
      this.latticeDrag = { index, offset: [0, 0, 0] };
      return;
    }
    const drag = this.latticeDrag;
    if (!drag) return;
    // The point moves in the plane facing the camera, at the scale of the view at the object.
    const cam = this.viewport.camera;
    const up = cam.screenUp();
    const forward = normalize(sub(cam.target, cam.eye()));
    const right = normalize([forward[1] * up[2] - forward[2] * up[1], forward[2] * up[0] - forward[0] * up[2], forward[0] * up[1] - forward[1] * up[0]]);
    const wpp = cam.worldPerPixel(this.shell.canvas.clientHeight);
    const start = shape.pts[0];
    const [dx, dy] = [p[0] - start[0], p[1] - start[1]];
    drag.offset = add(scale(right, dx * wpp), scale(up, -dy * wpp));
    this.drawTools();
  }

  private async gizmoEnd(): Promise<void> {
    const drag = this.gizmoDrag;
    this.gizmoDrag = null;
    const o = this.activeObject();
    if (!drag || !o) return;
    const moved = Math.hypot(...drag.translate) > 1e-7 || Math.abs(drag.degrees) > 1e-4 || drag.scale.some((s) => Math.abs(s - 1) > 1e-5);
    if (!moved) {
      this.viewport.renderer.setPreview(o.id, null);
      this.viewport.invalidate();
      return;
    }
    const rotate: Vec3 = [0, 0, 0];
    if (drag.handle.kind === 'rotate') rotate[drag.handle.axis] = drag.degrees;
    const reply = await dispatch<unknown>('transform_object', { id: o.id, translate: drag.translate, rotate, scale: drag.scale });
    if (!reply.ok) {
      // The preview goes on failure; on success it stays until the new geometry replaces it, so nothing jumps.
      this.viewport.renderer.setPreview(o.id, null);
      this.viewport.invalidate();
    }
    this.note(reply.ok ? '' : reply.error.message);
    await this.refresh();
  }

  private async latticeEnd(): Promise<void> {
    const drag = this.latticeDrag;
    this.latticeDrag = null;
    const o = this.activeObject();
    this.drawTools();
    if (!drag || !o || Math.hypot(...drag.offset) < 1e-9) return;
    const n = LATTICE_DIV;
    const point = [drag.index % n, Math.floor(drag.index / n) % n, Math.floor(drag.index / (n * n))];
    await this.run('lattice_deform', { id: o.id, divisions: [n, n, n], moves: [{ point, offset: drag.offset }] });
  }

  /** Redraws the gizmo or the lattice cage for the tool in hand (nothing for other tools). */
  drawTools(): void {
    const s = this.shell;
    const o = this.activeObject();
    const project = this.projector();
    if (this.brush === 'gizmo' && o) {
      const { center, len } = this.gizmoFrame(o);
      const t = this.gizmoDrag?.translate ?? [0, 0, 0];
      const { axes, uniform } = Gizmo.paths(add(center, t), len, project);
      s.gizmoAxes.forEach((p, i) => p.setAttribute('d', axes[i]));
      s.gizmoUniform.setAttribute('d', uniform);
    } else {
      s.gizmoAxes.forEach((p) => p.setAttribute('d', ''));
      s.gizmoUniform.setAttribute('d', '');
    }
    if (this.brush === 'lattice' && o) {
      const pts = this.latticePoints(o);
      const drag = this.latticeDrag;
      if (drag) pts[drag.index] = add(pts[drag.index], drag.offset);
      const scr = pts.map(project);
      const n = LATTICE_DIV;
      let lines = '';
      const seg = (a: number, b: number): void => {
        const [p, q] = [scr[a], scr[b]];
        if (p && q) lines += `M${p[0].toFixed(1)},${p[1].toFixed(1)}L${q[0].toFixed(1)},${q[1].toFixed(1)}`;
      };
      for (let k = 0; k < n; k++)
        for (let j = 0; j < n; j++)
          for (let i = 0; i < n; i++) {
            const at = i + j * n + k * n * n;
            if (i + 1 < n) seg(at, at + 1);
            if (j + 1 < n) seg(at, at + n);
            if (k + 1 < n) seg(at, at + n * n);
          }
      s.latticeLines.setAttribute('d', lines);
      s.latticePoints.setAttribute('d', scr.map((q) => (q ? `M${(q[0] - 4).toFixed(1)},${(q[1] - 4).toFixed(1)}h8v8h-8z` : '')).join(''));
    } else {
      s.latticeLines.setAttribute('d', '');
      s.latticePoints.setAttribute('d', '');
    }
  }

  /** Ends a Curve tool drag: a new curve becomes handles picked on the active object; a dragged handle is re-picked. */
  private async curveEnd(pts: [number, number][], wpp: number, edit?: number): Promise<void> {
    this.shell.tubeLine.setAttribute('points', '');
    const id = this.scene.active;
    if (id === null) return;
    const rect = this.shell.canvas.getBoundingClientRect();
    const pick = async (q: [number, number]): Promise<Vec3 | null> => {
      const ray = this.viewport.rayAtClient(rect.left + q[0], rect.top + q[1]);
      if (!ray) return null;
      const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3; normal?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
      if (!(hit.ok && hit.result.hit && hit.result.id === id && hit.result.point)) return null;
      return this.snapHandle(hit.result.point, hit.result.normal);
    };
    if (edit !== undefined && this.curve) {
      const q = pts[pts.length - 1];
      const at = this.curveScreen()[edit];
      if (!q || (at && Math.hypot(q[0] - at[0], q[1] - at[1]) < 3)) {
        this.curvePick = edit;
        this.shell.pointRadius.value = String(Math.round(this.curve.radius[edit] * 100));
        this.shell.pointTwist.value = String(this.curve.twist[edit]);
        if (this.curveAlt) this.setCorners('toggle');
        else this.drawCurve();
        this.note(`Handle ${edit + 1}: ${this.curve.sharp[edit] ? 'a corner' : 'smooth'}; Alt-click toggles, Corner round rounds it`);
        return;
      }
      const point = await pick(q);
      if (!point) this.note('A handle must stay on the active object');
      else if (this.shell.curveChain.checked) await this.chainDrag(edit, point);
      else this.curve.handles[edit] = point;
      this.drawCurve();
      void this.resample();
      return;
    }
    // Handles about every 40 pixels along the drawn path (2 to 32 of them).
    let length = 0;
    for (let i = 1; i < pts.length; i++) length += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
    if (pts.length < 2 || length < 8) {
      const at = pts[pts.length - 1] ? await pick(pts[pts.length - 1]) : null;
      if (!at) {
        this.note('Drag across the object to draw a curve');
        return;
      }
      if (this.curve) {
        const c = this.curve;
        c.handles.push(at);
        c.sharp.push(false);
        c.round.push(0);
        c.radius.push(1);
        c.twist.push(0);
      } else this.curve = { handles: [at], wpp, sharp: [false], round: [0], radius: [1], twist: [0], samples: null };
      this.curvePick = this.curve.handles.length - 1;
      this.drawCurve();
      if (this.curve.handles.length > 1) void this.resample();
      this.note(`Point ${this.curve.handles.length} added; keep clicking, or drag a handle`);
      return;
    }
    const count = Math.max(2, Math.min(32, Math.round(length / 40) + 1));
    const samples: [number, number][] = [];
    let walked = 0;
    let j = 1;
    for (let k = 0; k < count; k++) {
      const want = (length * k) / (count - 1);
      while (j < pts.length - 1 && walked + Math.hypot(pts[j][0] - pts[j - 1][0], pts[j][1] - pts[j - 1][1]) < want) {
        walked += Math.hypot(pts[j][0] - pts[j - 1][0], pts[j][1] - pts[j - 1][1]);
        j++;
      }
      const seg = Math.hypot(pts[j][0] - pts[j - 1][0], pts[j][1] - pts[j - 1][1]) || 1;
      const t = Math.min(1, Math.max(0, (want - walked) / seg));
      samples.push([pts[j - 1][0] + (pts[j][0] - pts[j - 1][0]) * t, pts[j - 1][1] + (pts[j][1] - pts[j - 1][1]) * t]);
    }
    const handles = (await Promise.all(samples.map(pick))).filter((p): p is Vec3 => p !== null);
    if (handles.length < 2) {
      this.note('Draw the curve over the active object');
      return;
    }
    this.curve = { handles, wpp, sharp: handles.map(() => false), round: handles.map(() => 0), radius: handles.map(() => 1), twist: handles.map(() => 0), samples: null };
    this.curvePick = null;
    this.drawCurve();
    void this.resample();
    this.note(`Curve ready: ${handles.length} handles; drag one to bend it, Alt-click one for a corner, then Apply curve or Tube from curve`);
  }

  /** A picked surface point lifted along its normal by Surface offset, then rounded to Grid snap's step when it is above 0. */
  private snapHandle(point: Vec3, normal?: Vec3): Vec3 {
    const offset = Number(this.shell.curveOffset.value) || 0;
    const step = Math.max(0, Number(this.shell.curveGrid.value) || 0);
    let p: Vec3 = normal && offset ? [point[0] + normal[0] * offset, point[1] + normal[1] * offset, point[2] + normal[2] * offset] : point;
    if (step > 0) p = [Math.round(p[0] / step) * step, Math.round(p[1] / step) * step, Math.round(p[2] / step) * step];
    return p;
  }

  /** Drag as chain: the handles keep their spacing and follow the dragged one (the first handle stays put), solved by the engine's curve_ik. */
  private async chainDrag(index: number, target: Vec3): Promise<void> {
    const c = this.curve;
    if (!c) return;
    const reply = await dispatch<{ points: { p: Vec3 }[]; reached: boolean }>('curve_ik', { points: this.curvePoints(), index, target, pin_start: index !== 0 });
    if (this.curve !== c) return;
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    reply.result.points.forEach((pt, i) => (c.handles[i] = pt.p));
    if (!reply.result.reached) this.note('The chain is too short to reach there; it stretched as far as it could');
  }

  /** Waits on a curve made outside the Curve tool's drag: switches to the Curve tool, keeping the brush it will apply. */
  private adoptCurve(handles: Vec3[], wpp: number, closed: boolean, brush?: string): void {
    this.shell.brushButtons.find((b) => b.dataset.brush === 'curve')?.click();
    if (brush) this.lastBrush = brush;
    this.shell.curveClosed.checked = closed;
    this.curve = { handles, wpp, sharp: handles.map(() => false), round: handles.map(() => 0), radius: handles.map(() => 1), twist: handles.map(() => 0), samples: null };
    this.curvePick = null;
    this.drawCurve();
    void this.resample();
  }

  /** Undoes the last brush stroke and brings its path back as curve handles on the restored surface, to move and apply again. */
  private async editLastStroke(): Promise<void> {
    const s = this.lastStroke;
    const id = this.scene.active;
    if (!s || id === null) {
      this.note(t('Make a brush stroke first'));
      return;
    }
    await this.run('undo');
    const want = Math.max(2, Math.min(24, Math.round(s.rays.length / 6) + 2));
    const rays = Array.from({ length: want }, (_, k) => s.rays[Math.round((k * (s.rays.length - 1)) / (want - 1))]);
    const hits = await Promise.all(rays.map((r) => dispatch<{ hit: boolean; id?: number; point?: Vec3 }>('pick', { origin: r.o, direction: r.d })));
    const handles = hits.flatMap((h) => (h.ok && h.result.hit && h.result.id === id && h.result.point ? [h.result.point] : []));
    if (handles.length < 2) {
      this.note(t('The stroke left too few points on the object to make a curve'));
      return;
    }
    this.lastStroke = null;
    this.adoptCurve(handles, s.wpp, false, s.brush);
    this.note(t('The stroke is undone and waits as a curve: move its handles, then Apply curve'));
  }

  /** A curve along the mask's edge or a face group's border (curve_from_border), lifted by Surface offset. */
  private async curveFromBorder(source: 'mask' | 'group'): Promise<void> {
    const id = this.scene.active;
    if (id === null) return;
    const args: Record<string, unknown> = { id, source, points: 16, offset: Number(this.shell.curveOffset.value) || 0 };
    if (source === 'group') {
      const answer = await this.prompt.ask(t('Face group number'), '1');
      if (answer === null) return;
      args.group = Math.max(0, Math.round(Number(answer) || 0));
    }
    const reply = await dispatch<{ points: { p: Vec3 }[]; closed: boolean; loops: number }>('curve_from_border', args);
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    const wpp = this.viewport.camera.worldPerPixel(this.shell.canvas.clientHeight);
    this.adoptCurve(reply.result.points.map((x) => x.p), wpp, reply.result.closed);
    this.note(`Curve from the border: ${reply.result.points.length} handles, ${reply.result.closed ? 'closed' : 'open'} (${reply.result.loops} border${reply.result.loops === 1 ? '' : 's'} found, the longest taken)`);
  }

  /** The drawn tube profile, in units of the tube radius; starts as a rounded-off square with smooth sides. */
  private profilePts: { p: [number, number]; sharp: boolean }[] = [
    { p: [0.8, 0.8], sharp: true },
    { p: [-0.8, 0.8], sharp: true },
    { p: [-0.8, -0.8], sharp: true },
    { p: [0.8, -0.8], sharp: true },
  ];
  private profileDrag: number | null = null;

  private drawProfile(): void {
    const xy = (q: [number, number]): [number, number] => [100 + q[0] * 76, 100 - q[1] * 76];
    const pts = this.profilePts.map((q) => xy(q.p));
    setAttr(this.shell.profileLine, 'points', pts.map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`).join(' '));
    setAttr(
      this.shell.profileDots,
      'd',
      pts.map(([x, y], i) => (this.profilePts[i].sharp ? `M${x - 4},${y - 4}h8v8h-8z` : `M${x - 4},${y}a4,4 0 1,0 8,0a4,4 0 1,0 -8,0`)).join(''),
    );
  }

  private savedProfiles(): Record<string, { p: [number, number]; sharp: boolean }[]> {
    try {
      return JSON.parse(localStorage.getItem(PROFILES_KEY) ?? '{}') as Record<string, { p: [number, number]; sharp: boolean }[]>;
    } catch {
      return {};
    }
  }

  private storeProfiles(all: Record<string, { p: [number, number]; sharp: boolean }[]>): void {
    try {
      localStorage.setItem(PROFILES_KEY, JSON.stringify(all));
    } catch {
      this.note(t('Profiles cannot be saved in this browser; this one lasts for the session'));
    }
  }

  /** Saved profiles appear in Tube profile after the presets, as saved:Name options. */
  private listProfiles(): void {
    const pick = this.shell.tubeProfile;
    const names = Object.keys(this.savedProfiles()).sort();
    for (const o of Array.from(pick.options)) if (o.value.startsWith('saved:') && !names.includes(o.value.slice(6))) o.remove();
    for (const name of names) {
      if (Array.from(pick.options).some((o) => o.value === `saved:${name}`)) continue;
      const o = el('option', '', name);
      o.value = `saved:${name}`;
      pick.append(o);
    }
  }

  private wireProfile(changed: () => void): void {
    const s = this.shell;
    const at = (e: PointerEvent): [number, number] => {
      const box = s.profileEditor.getBoundingClientRect();
      const x = ((e.clientX - box.left) / Math.max(1, box.width)) * 200;
      const y = ((e.clientY - box.top) / Math.max(1, box.height)) * 200;
      return [Math.max(-1.2, Math.min(1.2, (x - 100) / 76)), Math.max(-1.2, Math.min(1.2, (100 - y) / 76))];
    };
    const touched = (): void => {
      if (!s.tubeProfile.value.startsWith('saved:')) s.tubeProfile.value = 'custom';
      this.drawProfile();
    };
    s.profileEditor.addEventListener('pointerdown', (e) => {
      const q = at(e);
      const pts = this.profilePts;
      const near = pts.findIndex((v) => Math.hypot(v.p[0] - q[0], v.p[1] - q[1]) < 0.09);
      if (near >= 0 && e.shiftKey) {
        if (pts.length > 3) pts.splice(near, 1);
        else this.note(t('A profile needs at least three points'));
        touched();
        changed();
        return;
      }
      if (near >= 0 && e.altKey) {
        pts[near].sharp = !pts[near].sharp;
        touched();
        changed();
        return;
      }
      let index = near;
      if (index < 0) {
        let best = 0;
        let bestD = Infinity;
        for (let i = 0; i < pts.length; i++) {
          const [a, b] = [pts[i].p, pts[(i + 1) % pts.length].p];
          const dx = b[0] - a[0];
          const dy = b[1] - a[1];
          const t0 = Math.max(0, Math.min(1, ((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / Math.max(1e-9, dx * dx + dy * dy)));
          const d = Math.hypot(a[0] + dx * t0 - q[0], a[1] + dy * t0 - q[1]);
          if (d < bestD) [best, bestD] = [i, d];
        }
        pts.splice(best + 1, 0, { p: q, sharp: false });
        index = best + 1;
      }
      this.profileDrag = index;
      s.profileEditor.setPointerCapture(e.pointerId);
      touched();
    });
    s.profileEditor.addEventListener('pointermove', (e) => {
      if (this.profileDrag === null || !s.profileEditor.hasPointerCapture(e.pointerId)) return;
      this.profilePts[this.profileDrag].p = at(e);
      this.drawProfile();
    });
    s.profileEditor.addEventListener('pointerup', () => {
      if (this.profileDrag === null) return;
      this.profileDrag = null;
      changed();
    });
    s.tubeProfile.addEventListener('change', () => {
      const v = s.tubeProfile.value;
      if (!v.startsWith('saved:')) return;
      const saved = this.savedProfiles()[v.slice(6)];
      if (saved && saved.length >= 3) this.profilePts = saved.map((q) => ({ p: [q.p[0], q.p[1]], sharp: q.sharp }));
      this.drawProfile();
    });
    for (const b of s.profileButtons) {
      this.on(b, () => void this.profileAction(b.dataset.action ?? ''));
    }
    this.listProfiles();
    this.drawProfile();
  }

  private async profileAction(action: string): Promise<void> {
    const all = this.savedProfiles();
    const current = this.shell.tubeProfile.value.startsWith('saved:') ? this.shell.tubeProfile.value.slice(6) : '';
    if (action === 'profile_delete') {
      if (!current) {
        this.note(t('Choose a saved profile in Tube profile first'));
        return;
      }
      delete all[current];
      this.storeProfiles(all);
      this.shell.tubeProfile.value = 'custom';
      this.listProfiles();
      return;
    }
    const name = await this.prompt.ask(t('Save profile'), current || `Profile ${Object.keys(all).length + 1}`);
    if (!name) return;
    all[name] = this.profilePts.map((q) => ({ p: q.p, sharp: q.sharp }));
    this.storeProfiles(all);
    this.listProfiles();
    this.shell.tubeProfile.value = `saved:${name}`;
  }

  /** Point radius % and Point twist write to the chosen handle. */
  private setPointShape(): void {
    const c = this.curve;
    const pick = this.curvePick;
    if (!c || pick === null) {
      this.note('Click a curve handle first');
      return;
    }
    c.radius[pick] = Math.max(0.05, Number(this.shell.pointRadius.value) / 100);
    c.twist[pick] = Number(this.shell.pointTwist.value) || 0;
    this.shell.radiusMode.value = 'per_point';
    this.note(`Handle ${pick + 1}: radius ${Math.round(c.radius[pick] * 100)}%, twist ${c.twist[pick]} degrees; Radius along path is Per point`);
  }

  /** Runs the last brush along the waiting curve as one stroke (the curve stays, so it can be applied again). */
  private async applyCurve(): Promise<void> {
    const c = this.curve;
    const id = this.scene.active;
    if (!c || id === null) {
      this.note('Draw a curve with the Curve stroke tool first');
      return;
    }
    const radius = Math.max(1e-4, Number(this.shell.radius.value) * c.wpp);
    let span = 0;
    for (let i = 1; i < c.handles.length; i++) span = Math.max(span, distance3(c.handles[i - 1], c.handles[i]));
    const per = Math.max(1, Math.min(64, Math.ceil(span / (radius * 0.1))));
    const sampled = await dispatch<{ positions: Vec3[] }>('sample_curve', { points: this.curvePoints(), ...this.curveOptions(), spacing: radius * 0.1 });
    let points = sampled.ok && sampled.result.positions.length > 1 ? sampled.result.positions : SceneView.spline(c.handles, per);
    if (sampled.ok && this.shell.curveClosed.checked && points.length > 2) points = [...points, points[0]];
    if (points.length > 4096) points = points.filter((_, i) => i % Math.ceil(points.length / 4096) === 0);
    const shape = this.shell.curveIntensity.value;
    const last = Math.max(1, points.length - 1);
    const weigh = (i: number): number => {
      const u = i / last;
      if (shape === 'fade_in') return 0.1 + 0.9 * u;
      if (shape === 'fade_out') return 1 - 0.9 * u;
      if (shape === 'fade_both') return 0.1 + 0.9 * Math.sin(Math.PI * u);
      if (shape === 'pulse') return 0.55 + 0.45 * Math.cos(u * Math.PI * 8);
      return 1;
    };
    const weighted = shape === 'even' ? points : points.map((p, i) => [p[0], p[1], p[2], weigh(i)]);
    await this.run('sculpt_stroke', { ...this.brushArgs(radius, false), ...(shape === 'even' ? {} : { pressure_strength: true }), id, points: weighted });
  }

  private clearCurve(): void {
    this.curve = null;
    this.curvePick = null;
    this.drawCurve();
  }

  /** Draws the falloff curve editor's line and points from the five weights. */
  private drawFalloff(): void {
    const w = this.falloffWeights;
    const preset = FALLOFFS[this.shell.falloff.value];
    const xy = (i: number, v: number): [number, number] => [4 + (i / (FALLOFF_POINTS - 1)) * 192, 60 - v * 56];
    const line = Array.from({ length: 41 }, (_, k) => {
      const x = (k / 40) * (FALLOFF_POINTS - 1);
      const i = Math.min(FALLOFF_POINTS - 2, Math.floor(x));
      const v = preset ? preset(k / 40) : w[i] + (w[i + 1] - w[i]) * (x - i);
      return xy(x, v);
    });
    this.shell.falloffLine.setAttribute('points', line.map((q) => `${q[0].toFixed(1)},${q[1].toFixed(1)}`).join(' '));
    this.shell.falloffDots.setAttribute('d', w.map((v, i) => xy(i, v)).map(([x, y]) => `M${x - 3},${y - 3}h6v6h-6z`).join(''));
  }

  private falloffDrag: number | null = null;
  private falloffWeights: number[] = Array.from({ length: FALLOFF_POINTS }, (_, i) => FALLOFFS.smooth(i / (FALLOFF_POINTS - 1)));

  /** The key light (first) and up to three added lights, as the Light fields edit them. */
  private lightSpecs: (LightSpec & { hex: string })[] = [{ azimuth: -37, elevation: 41, intensity: 1, color: hexToLinear('#fff8ef'), hex: '#fff8ef', followsView: true }];

  /** Rebuilds the Light choices (reusing options) and selects `pick`. */
  private patchLightList(pick: number): void {
    const select = this.shell.lightPick;
    while (select.options.length > this.lightSpecs.length) select.options[select.options.length - 1].remove();
    for (let i = select.options.length; i < this.lightSpecs.length; i++) {
      const option = el('option', '', `Light ${i + 1}`);
      option.value = String(i);
      select.append(option);
    }
    select.value = String(pick);
    this.showLight();
  }

  /** Shows the chosen light's settings in the Light fields. */
  private showLight(): void {
    const s = this.shell;
    const i = Number(s.lightPick.value);
    const spec = this.lightSpecs[i] ?? this.lightSpecs[0];
    s.keyAzimuth.value = String(Math.round(spec.azimuth));
    s.keyElevation.value = String(Math.round(spec.elevation));
    s.keyIntensity.value = String(Math.round(spec.intensity * 100));
    s.keyColor.value = spec.hex;
    s.keyFollows.checked = spec.followsView;
    scheduleChrome();
  }

  /** Environments loaded from files this session, by their select value. */
  private environments = new Map<string, EnvImage>();

  /** Lights with the chosen environment; an environment needs lit (PBR) shading, so choosing one switches to it. */
  private async chooseEnvironment(): Promise<void> {
    const name = this.shell.envSelect.value;
    if (name === 'off') this.viewport.renderer.setEnvironment(null);
    else {
      const img = this.environments.get(name) ?? presetEnvironment(name);
      this.viewport.renderer.setEnvironment(img);
      if (this.shell.shading.value !== 'pbr' && !this.shell.shadeActive.checked) {
        this.shell.shading.value = 'pbr';
        this.applyShading();
      }
    }
    this.applyLighting();
  }

  private applyLighting(): void {
    const s = this.shell;
    const r = this.viewport.renderer;
    r.lighting = {
      environment: s.envSelect.value !== 'off',
      background: s.envBackground.checked,
      rotation: (Number(s.envRotate.value) * Math.PI) / 180,
      exposure: Number(s.exposure.value) / 100,
      lights: Number(s.fixedLights.value) / 100,
      tone: s.tone.value,
      blur: Number(s.envBlur.value) / 100,
      keyAzimuth: this.lightSpecs[0].azimuth,
      keyElevation: this.lightSpecs[0].elevation,
      keyIntensity: this.lightSpecs[0].intensity,
      keyColor: this.lightSpecs[0].color,
      keyFollowsView: this.lightSpecs[0].followsView,
      shadows: s.shadows.checked,
      ground: s.groundShadow.checked,
      extra: this.lightSpecs.slice(1),
    };
    r.setPost({ ssao: s.postSsao.checked, bloom: s.postBloom.checked, dof: s.postDof.checked, vignette: s.postVignette.checked, grain: s.postGrain.checked });
    this.viewport.invalidate();
  }

  private async loadEnvironment(): Promise<void> {
    const file = this.shell.envInput.files?.[0];
    this.shell.envInput.value = '';
    if (!file) return;
    try {
      const img = file.name.toLowerCase().endsWith('.hdr') ? parseHdr(await file.arrayBuffer()) : await imageEnvironment(file);
      const value = `file:${file.name}`;
      this.environments.set(value, img);
      const select = this.shell.envSelect;
      if (![...select.options].some((o) => o.value === value)) {
        const option = el('option', '', file.name.replace(/\.[^.]+$/, ''));
        option.value = value;
        select.append(option);
      }
      select.value = value;
      await this.chooseEnvironment();
      this.note(`Environment: ${file.name} (${img.width} by ${img.height})`);
    } catch (err) {
      this.note(`That file could not be read as an environment: ${String(err instanceof Error ? err.message : err)}`);
    }
  }

  private applyShading(): void {
    const id = this.scene.active;
    const value = this.shell.shading.value;
    if (this.shell.shadeActive.checked && id !== null) this.viewport.renderer.setShading(value, id);
    else this.viewport.renderer.setShading(value);
    this.viewport.invalidate();
  }

  private async loadAlpha(): Promise<void> {
    const file = this.shell.alphaInput.files?.[0];
    this.shell.alphaInput.value = '';
    if (!file) return;
    const name = file.name.replace(/\.[^.]+$/, '').slice(0, 64) || 'alpha';
    const bytes = new Uint8Array(await file.arrayBuffer());
    let bin = '';
    for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    const reply = await dispatch<{ name: string }>('alpha_load', { name, png_base64: btoa(bin) });
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    const select = this.shell.alpha;
    if (![...select.options].some((o) => o.value === reply.result.name)) {
      const option = el('option', '', reply.result.name);
      option.value = reply.result.name;
      select.append(option);
    }
    select.value = reply.result.name;
    this.note(`Alpha loaded: ${reply.result.name}`);
  }

  private async loadMatcap(): Promise<void> {
    const file = this.shell.matcapInput.files?.[0];
    this.shell.matcapInput.value = '';
    if (!file) return;
    try {
      const bitmap = await createImageBitmap(file);
      const name = `img:${file.name}`;
      this.viewport.renderer.addMatcap(name, bitmap);
      bitmap.close();
      const select = this.shell.shading;
      if (![...select.options].some((o) => o.value === name)) {
        const option = el('option', '', file.name.replace(/\.[^.]+$/, ''));
        option.value = name;
        select.append(option);
      }
      select.value = name;
      this.applyShading();
      this.note(`Matcap: ${file.name}`);
    } catch {
      this.note('That image could not be read as a matcap');
    }
  }

  private async loadCatalogue(): Promise<void> {
    // With the engine unreachable the window's own keys still work.
    const found = await fetchCatalogue<{ commands: CatalogueCommand[] }>();
    if (found) this.catalogue = found.commands;
    this.registerCatalogue();
  }

  private registerCatalogue(): void {
    const fresh: Command[] = [];
    for (const c of this.catalogue) {
      const props = c.inputSchema?.properties ?? {};
      const takesId = 'id' in props;
      const needsId = takesId && (c.inputSchema?.required ?? []).includes('id');
      if (needsId) NEEDS_OBJECT.add(c.id);
      const have = commands.get(c.id);
      if (have) {
        have.group ??= c.group;
        have.description ??= c.description;
        have.mutates ??= !c.annotations?.readOnlyHint;
        continue;
      }
      const params: Record<string, CommandParam> = {};
      for (const [name, p] of Object.entries(props)) {
        if (p.type !== 'string' && p.type !== 'number' && p.type !== 'integer' && p.type !== 'boolean' && p.type !== 'array' && p.type !== 'object') continue;
        params[name] = { type: p.type, required: (c.inputSchema?.required ?? []).includes(name), enum: p.enum, min: p.minimum, max: p.maximum, description: p.description };
      }
      fresh.push({
        id: c.id,
        label: c.label,
        keys: c.id === 'multires.set_level' ? [] : c.keys,
        group: c.group,
        description: c.description,
        params,
        mutates: !c.annotations?.readOnlyHint,
        risky: c.annotations?.destructiveHint === true,
        run: (p) => {
          const args: Record<string, unknown> = { ...p };
          if (takesId && args.id === undefined && this.scene.active !== null) args.id = this.scene.active;
          return this.run(c.name, args);
        },
      });
    }
    commands.add(...fresh);
    applyShortcuts(commands);
    const legacy = Object.entries(takeLegacyKeys());
    for (const [raw, action] of legacy) {
      const combo = normalizeCombo(raw);
      if (!action) {
        const held = commands.forCombo(combo);
        if (held) commands.setKeys(held.id, (held.keys ?? []).filter((k) => k !== combo));
        continue;
      }
      const id = legacyTarget(action, this.catalogue);
      const target = id ? commands.get(id) : undefined;
      if (target) commands.setKeys(target.id, [...(target.keys ?? []).filter((k) => k !== combo), combo]);
    }
    if (legacy.length) persistShortcuts(commands);
    commands.notify();
  }

  /** Every window action: the panel buttons through their handlers, then views, toggles and object actions. */
  registerCommands(): void {
    const s = this.shell;
    const list: Command[] = [];
    const toggle = (box: HTMLInputElement): void => {
      box.checked = !box.checked;
      box.dispatchEvent(new Event('change', { bubbles: true }));
    };
    const layers = (): boolean => (this.activeObject()?.paint_layers?.length ?? 0) > 0;
    const special: Record<string, () => boolean> = {
      'paint.layer_delete': layers,
      'paint.layer_merge': layers,
      'paint.layer_flatten': layers,
      'paint.layer_up': layers,
      'paint.layer_down': layers,
      'add.apply_primitive': () => !!this.scene.objects.find((o) => o.id === this.scene.active)?.primitive,
      'light.remove': () => Number(s.lightPick.value) > 0,
    };
    for (const table of Object.values(PANEL_CMDS)) {
      for (const spec of Object.values(table)) {
        list.push({ id: spec.id, label: spec.label, keys: spec.keys, icon: spec.icon, run: () => this.handlers.get(spec.id)?.(), enabled: special[spec.id] });
      }
    }
    list.push({ id: 'add.apply_primitive', label: 'Apply to Primitive', run: () => this.handlers.get('add.apply_primitive')?.(), enabled: special['add.apply_primitive'] });
    for (const [kind, label] of BRUSHES) {
      list.push({ id: `tool.brush.${kind}`, label, icon: 'brush', run: () => this.handlers.get(`tool.brush.${kind}`)?.(), checked: () => this.brush === kind });
    }
    const views: [ViewName, string, string][] = [['front', 'Front', '1'], ['back', 'Back', 'Mod+1'], ['right', 'Right', '3'], ['left', 'Left', 'Mod+3'], ['top', 'Top', '7'], ['bottom', 'Bottom', 'Mod+7']];
    for (const [name, label, key] of views) list.push({ id: `view.${name}`, label: `View ${label}`, keys: [key], icon: 'cube', run: () => this.viewport.snap(name) });
    list.push(
      { id: 'view.ortho', label: 'Toggle Orthographic', keys: ['5'], run: () => this.viewport.toggleOrtho() },
      { id: 'view.frame', label: 'Frame Active Object', keys: ['F'], icon: 'frame', run: () => this.frameActive(), enabled: () => this.scene.active !== null },
      { id: 'view.groups', label: 'Show Face Groups', run: () => toggle(s.showGroups), checked: () => s.showGroups.checked },
      { id: 'view.trackball', label: 'Trackball Rotation', run: () => toggle(s.trackball), checked: () => s.trackball.checked },
      { id: 'view.floor_grid', label: 'Floor Grid', keys: ['Shift+P'], run: () => toggle(s.floorGrid), checked: () => s.floorGrid.checked },
      { id: 'view.backdrop_gradient', label: 'Gradient Background', run: () => { s.backdropMode.value = s.backdropMode.value === 'gradient' ? 'color' : 'gradient'; s.backdropMode.dispatchEvent(new Event('change', { bubbles: true })); }, checked: () => s.backdropMode.value === 'gradient' },
      { id: 'sculpt.mirror_x', label: 'Mirror X', run: () => toggle(s.symX), checked: () => s.symX.checked },
      { id: 'sculpt.invert', label: 'Invert Brush', run: () => toggle(s.invert), checked: () => s.invert.checked },
      { id: 'sculpt.dyntopo', label: 'Dynamic Topology', run: () => toggle(s.dyntopo), checked: () => s.dyntopo.checked, enabled: () => !s.dyntopo.disabled },
      { id: 'view.panel_left', label: 'Panel on the Left', run: () => toggle(s.panelLeft), checked: () => s.layout.panelLeft },
      {
        id: 'view.set_pivot',
        label: 'Set Pivot Here',
        run: () => {
          const e = this.contextEvent;
          if (e) void this.pivot(new MouseEvent('dblclick', { clientX: e.clientX, clientY: e.clientY, altKey: true }));
        },
        enabled: () => this.contextEvent !== null,
      },
      {
        id: 'object.rename',
        label: 'Rename Object',
        run: () => void this.rename(),
      },
      {
        id: 'object.delete',
        label: 'Delete Object',
        keys: ['Delete', 'Backspace'],
        icon: 'trash',
        mutates: true,
        risky: true,
        run: () => void this.run('delete_object', { id: this.scene.active }),
      },
    );
    NEEDS_OBJECT.add('object.rename');
    NEEDS_OBJECT.add('object.delete');
    list.push(
      { id: 'selection.save', label: 'Save Selection Set', mutates: true, run: () => void this.selectionSet('save') },
      { id: 'selection.restore', label: 'Restore Selection Set', mutates: true, run: () => void this.selectionSet('restore') },
      { id: 'selection.delete', label: 'Delete Selection Set', mutates: true, run: () => void this.selectionSet('delete') },
    );
    for (const id of ['selection.save', 'selection.restore', 'selection.delete']) NEEDS_OBJECT.add(id);
    list.push(
      { id: 'curve.edit_last_stroke', label: 'Edit Last Stroke as Curve', mutates: true, run: () => void this.editLastStroke(), enabled: () => this.lastStroke !== null },
      { id: 'curve.from_mask_edge', label: 'Curve from Mask Edge', run: () => void this.curveFromBorder('mask') },
      { id: 'curve.from_group_border', label: 'Curve from Group Border', run: () => void this.curveFromBorder('group') },
    );
    for (const id of ['curve.edit_last_stroke', 'curve.from_mask_edge', 'curve.from_group_border']) NEEDS_OBJECT.add(id);
    const channelVerbs: [string, string, string, string][] = [
      ['channel.copy_mask', 'Copy Mask', 'channel_copy', 'mask'], ['channel.paste_mask', 'Paste Mask', 'channel_paste', 'mask'], ['channel.clear_mask', 'Clear Mask Channel', 'channel_clear', 'mask'],
      ['channel.copy_groups', 'Copy Groups', 'channel_copy', 'groups'], ['channel.paste_groups', 'Paste Groups', 'channel_paste', 'groups'], ['channel.clear_groups', 'Clear Groups', 'channel_clear', 'groups'],
      ['channel.copy_paint', 'Copy Paint', 'channel_copy', 'paint'], ['channel.paste_paint', 'Paste Paint', 'channel_paste', 'paint'], ['channel.clear_paint', 'Clear Paint Channel', 'channel_clear', 'paint'],
    ];
    for (const [id, label, engine, channel] of channelVerbs) {
      list.push({ id, label, mutates: engine !== 'channel_copy', risky: id === 'channel.clear_paint', run: () => void this.run(engine, { id: this.scene.active, channel }) });
      NEEDS_OBJECT.add(id);
    }
    for (const size of ['small', 'normal', 'large']) {
      list.push({ id: `view.ui_${size}`, label: `Interface Size ${size[0].toUpperCase()}${size.slice(1)}`, run: () => this.setUiSize(size), checked: () => s.layout.size === size });
    }
    for (const n of [0, 1, 2, 3]) {
      list.push({
        id: `view.smooth_${n}`,
        label: n === 0 ? 'Smooth Preview Off' : `Smooth Preview ${n} Level${n > 1 ? 's' : ''}`,
        run: () => {
          s.smoothPick.value = String(n);
          this.setSmoothPreview(n);
        },
        checked: () => this.smoothPreview === n,
      });
    }
    for (const format of Array.from(s.exportFormat.options, (o) => o.value)) {
      list.push({
        id: `file.export.${format}`,
        label: `Export ${format.toUpperCase()}`,
        run: () => {
          s.exportFormat.value = format;
          void this.fileAction('export');
        },
      });
    }
    commands.blocked = (cmd) => (NEEDS_OBJECT.has(cmd.id) && this.scene.active === null ? t('Select an object first') : '');
    commands.add(...list);
    s.layout.onDraw = scheduleChrome;
    scheduleChrome();
  }

  private setUiSize(size: string): void {
    this.shell.layout.setSize(size);
    this.shell.uiSize.value = size;
  }

  private async rename(): Promise<void> {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o) return;
    const name = await this.prompt.ask(t('Rename Object'), o.name);
    if (name && name !== o.name) await this.run('rename_object', { id: o.id, name });
  }

  /** Selection Sets by name: saving offers the next free name, restoring and deleting offer the newest set. */
  private async selectionSet(action: 'save' | 'restore' | 'delete'): Promise<void> {
    const id = this.scene.active;
    if (id === null) return;
    const listed = await dispatch<{ sets: { name: string }[] }>('selection_set_list', { id });
    const names = listed.ok ? listed.result.sets.map((x) => x.name) : [];
    if (action !== 'save' && !names.length) {
      this.note(t('This object has no selection sets yet'));
      return;
    }
    const chosen = this.shell.setPick.value;
    let suggest = action !== 'save' && names.includes(chosen) ? chosen : names[names.length - 1] ?? '';
    if (action === 'save') for (let n = names.length + 1; !suggest || names.includes(suggest); n++) suggest = `Set ${n}`;
    const title = action === 'save' ? t('Save Selection Set') : action === 'restore' ? t('Restore Selection Set') : t('Delete Selection Set');
    const name = await this.prompt.ask(title, suggest);
    if (!name) return;
    if (action !== 'save' && !names.includes(name)) {
      this.note(t('No selection set is named {name}').replace('{name}', name));
      return;
    }
    await this.run(`selection_set_${action}`, { id, name });
  }

  /** Right-click menus: the viewport's, the object rows' and the pin menu of every panel button. */
  wireContextMenus(): void {
    const s = this.shell;
    const standalone = (): PopupMenu => {
      const menu = new PopupMenu(commands, null, true, 'context');
      menu.onClose = () => window.setTimeout(() => (this.contextEvent = null), 0);
      return menu;
    };
    const stage = standalone();
    const rows = standalone();
    const pin = standalone();
    let down = { x: 0, y: 0 };
    s.canvas.addEventListener('pointerdown', (e) => (down = { x: e.clientX, y: e.clientY }));
    s.canvas.addEventListener('contextmenu', (e) => {
      if (this.stroking() || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4) return;
      this.contextEvent = e;
      stage.setItems(tidy(VIEWPORT_MENU));
      stage.openAt(e.clientX, e.clientY);
    });
    s.objectList.addEventListener('contextmenu', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (!row?.dataset.id) return;
      e.preventDefault();
      const { clientX, clientY } = e;
      void this.run('select_object', { id: Number(row.dataset.id) }).then(() => {
        rows.setItems(tidy(OBJECT_ROW_MENU));
        rows.openAt(clientX, clientY);
      });
    });
    commands.add({
      id: 'rail.pin',
      label: () => (s.layout.isPinned(this.pinTarget) ? 'Take off tool rail' : 'Pin to tool rail'),
      run: () => s.layout.togglePin(this.pinTarget),
    });
    s.layout.onPinMenu = (id, x, y) => {
      if (!commands.get(id)) return;
      this.pinTarget = id;
      pin.setItems(tidy(PIN_MENU(id)));
      pin.openAt(x, y);
    };
  }

  /** Tools that act once on the clicked point: topology mask, group mask, hide group, insert. */
  private async clickTool(ray: Ray, wpp: number, ctrl: boolean): Promise<void> {
    const id = this.scene.active;
    if (id === null) return;
    const tool = this.brush;
    const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3; normal?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
    // Retopology lands on the sculpt or on the retopology already laid over it; the engine snaps to the sculpt.
    const onActive = hit.ok && hit.result.hit && hit.result.point !== undefined && (tool === 'insert' || tool === 'plugin' || tool === 'retopo' || hit.result.id === id);
    if (!hit.ok || !onActive || !hit.result.point) {
      this.note(tool === 'insert' ? 'Click on a surface to insert there' : 'Click on the active object');
      return;
    }
    const point = hit.result.point;
    const op = ctrl ? 'subtract' : 'add';
    if (tool === 'mask_topo') await this.run('mask_topology', { id, point, distance: Math.max(1e-4, Number(this.shell.radius.value) * wpp * 2), op });
    else if (tool === 'mask_group') await this.run('mask_by_group', { id, point, op });
    else if (tool === 'hide_group') await this.run('hide_faces', { id, action: ctrl ? 'isolate' : 'hide', point });
    else if (tool === 'plugin') await this.runPlugin({ point, normal: hit.result.normal, id: hit.result.id });
    else if (tool === 'edge_loop') await this.run('insert_edge_loop', { id, point });
    else if (tool === 'bevel_edge') await this.run('bevel_edge', { id, point, mode: ctrl ? 'round' : 'chamfer' });
    else if (tool === 'clip') {
      // A level cut through the click as seen: everything above it on screen goes (Ctrl keeps both halves).
      await this.run('clip_plane', { id, point, normal: this.viewport.camera.screenUp(), mode: ctrl ? 'slice' : 'keep' });
    } else if (tool === 'zsphere') {
      // A new joint just off the surface, linked to the nearest one; the skeleton is remade around it.
      const o = this.scene.objects.find((x) => x.id === id);
      const size = o ? Math.max(...[0, 1, 2].map((k) => o.bounds.max[k] - o.bounds.min[k])) : 1;
      const n = hit.result.normal ?? [0, 0, 0];
      const at = [0, 1, 2].map((k) => point[k] + n[k] * size * 0.12);
      await this.run('edit_build', { id, add: { point: at } });
    } else if (tool === 'pose_joint') {
      // Turns the clicked joint by the Angle field about the Deform axis (Ctrl the other way); the limb beyond follows.
      const axis = this.shell.deformAxis.value;
      const axisVec = axis === 'x' ? [1, 0, 0] : axis === 'y' ? [0, 1, 0] : [0, 0, 1];
      const angle = (Number(this.shell.deformAngle.value) || 30) * (ctrl ? -1 : 1);
      await this.run('pose_joint', { id, point, axis: axisVec, angle });
    } else if (tool === 'groom_comb' || tool === 'groom_cut') {
      // A short stroke through the click: comb sweeps toward the screen's right (Ctrl left), cut trims there.
      const reach = Math.max(1e-4, Number(this.shell.radius.value) * wpp);
      const cam = this.viewport.camera;
      const right = normalize(cross(sub(cam.target, cam.eye()), cam.screenUp()));
      const side = scale(right, reach * (ctrl ? -1 : 1));
      const points = [sub(point, side), point, add(point, side)];
      await this.run('groom_fibers', { id, mode: tool === 'groom_comb' ? 'comb' : 'cut', points, radius: reach });
    }
    else if (tool === 'retopo') {
      // Four clicks around a face, in order; each quad joins the corners it shares with earlier ones.
      this.retopoPoints.push(point);
      if (this.retopoPoints.length < 4) {
        this.note(`Retopology: corner ${this.retopoPoints.length} of 4 placed`);
        return;
      }
      const points = this.retopoPoints;
      this.retopoPoints = [];
      const into = this.retopoInto !== null && this.scene.objects.some((o) => o.id === this.retopoInto) ? this.retopoInto : undefined;
      const reply = await dispatch<{ id: number; faces: number }>('retopo_face', { target: id, points, ...(into ? { into } : {}) });
      if (!reply.ok) this.note(reply.error.message);
      else {
        this.retopoInto = reply.result.id;
        this.note(`Retopology: ${reply.result.faces} faces`);
      }
      await this.refresh();
    }
    else if (tool === 'vdm') {
      // As wide as the brush on screen; the alpha angle turns it, Ctrl presses it in.
      const radius = Math.max(1e-4, Number(this.shell.radius.value) * wpp);
      await this.run('vdm_stamp', { id, vdm: this.shell.vdmPick.value, point, normal: hit.result.normal, radius, angle: Number(this.shell.alphaAngle.value) || 0, strength: ctrl ? -1 : 1 });
    } else {
      const kind = this.shell.insertKind.value;
      const size = Math.max(0.01, Math.min(10, Number(this.shell.insertSize.value) || 0.2));
      const args: Record<string, unknown> = { point, normal: hit.result.normal, size };
      if (kind === 'previous') {
        if (this.previous === null) {
          this.note('Select the object to insert first, then the one to insert onto');
          return;
        }
        args.source_id = this.previous;
      } else args.kind = kind;
      await this.run('insert_mesh', args);
    }
  }

  private async pivot(e: MouseEvent): Promise<void> {
    // With a sculpting brush a plain double-click would also lay down dabs; Alt clicks never sculpt.
    if (this.brush !== 'orbit' && !e.altKey) return;
    const ray = this.viewport.rayAtClient(e.clientX, e.clientY);
    if (!ray) return;
    const hit = await dispatch<{ hit: boolean; point?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
    if (hit.ok && hit.result.hit && hit.result.point) this.viewport.pivotAt(hit.result.point);
  }

  private onPlane(ray: Ray): Vec3 | null {
    if (!this.tube) return null;
    const d = dot(ray.dir, this.tube.normal);
    if (Math.abs(d) < 1e-6) return null;
    return add(ray.origin, scale(ray.dir, dot(sub(this.tube.anchor, ray.origin), this.tube.normal) / d));
  }

  private tubeBegin(ray: Ray, wpp: number): void {
    const cam = this.viewport.camera;
    const anchor = cam.target;
    this.tube = { points: [], step: Math.max(wpp * 8, 1e-4), anchor, normal: normalize(sub(anchor, cam.eye())), screen: [] };
    this.tubeMove(ray);
  }

  private tubeMove(ray: Ray): void {
    const p = this.onPlane(ray);
    const t = this.tube;
    if (!p || !t) return;
    const last = t.points[t.points.length - 1];
    if (!last || distance3(last, p) >= t.step) t.points.push(p);
  }

  private async tubeEnd(): Promise<void> {
    const t = this.tube;
    this.tube = null;
    this.shell.tubeLine.setAttribute('points', '');
    if (!t || t.points.length < 2) {
      this.note('Drag across the view to draw a tube');
      return;
    }
    const points = t.points.length > 4096 ? t.points.filter((_, i) => i % Math.ceil(t.points.length / 4096) === 0) : t.points;
    await this.run('curve_tube', { points, smooth: 2, ...this.tubeArgs() });
  }

  private async build(action: string): Promise<void> {
    const first = this.scene.objects.length === 0;
    if (action === 'skeleton') {
      const preset = SKELETONS[this.shell.skeleton.value] ?? SKELETONS.biped;
      const resolution = Math.max(32, Math.min(256, Number(this.shell.resolution.value) || 128));
      await this.run('skeleton_mesh', { ...preset.build(), blend: 0.3, resolution, name: preset.label });
    } else if (action === 'lathe') {
      const preset = LATHES[this.shell.lathe.value] ?? LATHES.vase;
      await this.run('lathe', { profile: preset.profile, segments: 48, name: preset.label });
    } else if (action === 'array_line' || action === 'array_radial' || action === 'array_mirror' || action === 'array_curve') {
      const id = this.scene.active;
      if (id === null) {
        this.note('Select the object to repeat first');
        return;
      }
      const count = Math.max(2, Math.min(200, Math.round(Number(this.shell.arrayCount.value) || 6)));
      const linked = this.shell.arrayLinked.checked;
      if (action === 'array_curve') {
        // Along the waiting curve stroke's spline (draw one with the Curve stroke tool).
        if (!this.curve) {
          this.note('Draw a path with the Curve stroke tool first');
          return;
        }
        const points = SceneView.spline(this.curve.handles, 8);
        await this.run('array_object', { id, count, mode: 'curve', points, linked });
      } else if (action === 'array_mirror') await this.run('array_object', { id, mode: 'mirror', axis: 'x', linked });
      else await this.run('array_object', { id, count, mode: action === 'array_line' ? 'linear' : 'radial', axis: 'y', linked });
    } else if (action === 'shadow_box') {
      const s = this.silhouettes;
      const n = [s.front, s.side, s.top].filter(Boolean).length;
      if (n < 2) {
        this.note(`Draw ${2 - n} more outline${n === 1 ? '' : 's'} with the Silhouette tool from the front, side or top view`);
        return;
      }
      await this.run('shadow_box', { ...s, resolution: Math.max(32, Math.min(256, Number(this.shell.resolution.value) || 128)) });
    } else if (action === 'clear_outlines') {
      this.silhouettes = {};
      this.note('Outlines cleared');
    } else if (action === 'extrude_outline') {
      // The front outline drawn with the Silhouette tool, as a flat shape.
      if (!this.silhouettes.front) {
        this.note('Draw an outline with the Silhouette tool in the front view first');
        return;
      }
      await this.run('extrude_shape', { outlines: [this.silhouettes.front], round: 0.02 });
    } else if (action === 'text') {
      const text = this.shell.textInput.value.trim();
      if (!text) {
        this.note('Type the text to build first');
        return;
      }
      await this.run('text_mesh', { text });
    } else if (action === 'vdm_capture') {
      const id = this.scene.active;
      if (id === null) {
        this.note('Select a sculpted plane to capture first');
        return;
      }
      const name = `Map ${this.shell.vdmPick.options.length - 2}`;
      const reply = await dispatch<{ name: string }>('vdm_capture', { id, name });
      if (!reply.ok) {
        this.note(reply.error.message);
        return;
      }
      const option = el('option', '', name);
      option.value = name;
      this.shell.vdmPick.append(option);
      this.shell.vdmPick.value = name;
      this.note(`${name} captured; stamp it with the VDM stamp tool`);
      return;
    }
    if (first) this.frameActive();
  }

  private fetching = new Set<number>();
  private again = new Set<number>();

  private refetch(id: number): void {
    if (this.fetching.has(id)) {
      this.again.add(id);
      return;
    }
    this.fetching.add(id);
    void fetchMesh(id, this.smoothPreview).then((mesh) => {
      if (mesh) this.viewport.setMesh(mesh);
      this.fetching.delete(id);
      if (this.again.delete(id)) this.refetch(id);
    });
  }

  private brushState(): BrushState {
    const s = this.shell;
    return {
      brush: this.brush,
      radiusPx: Number(s.radius.value),
      strength: Number(s.strength.value) / 100,
      invert: s.invert.checked,
      falloff: s.falloff.value === 'custom' ? 'smooth' : s.falloff.value,
      falloffCurve: s.falloff.value === 'custom' ? this.falloffWeights.map((w) => Math.round(w * 1000) / 1000) : null,
      stroke: s.strokeMode.value,
      pressure: { gamma: Math.max(0.2, Math.min(5, Number(s.pressureGamma.value) || 1)), strength: s.pressureStrength.checked, size: s.pressureSize.checked },
      lazy: Number(s.lazy.value) / 100,
      symmetry: { x: s.symX.checked, y: false, z: false, radial: Math.max(1, Math.min(150, Number(s.radial.value) || 1)) },
      dyntopo: s.dyntopo.checked ? Number(s.detail.value) / 100 : null,
      dyntopoSize: s.dyntopo.checked && s.detailMode.value === 'constant' ? this.constantSize(Number(s.detail.value) / 100) : null,
      dyntopoScreenPx: s.dyntopo.checked && s.detailMode.value === 'screen' ? Number(s.detail.value) * 0.2 : null,
      paint: this.brush === 'paint' ? this.paintArgs(this.shell.paintMaterial.checked) : null,
      alpha: s.alpha.value === 'none' ? null : { alpha: s.alpha.value, alpha_angle: Number(s.alphaAngle.value) || 0, alpha_up: this.viewport.camera.screenUp() },
    };
  }

  /** The paint brush's or fill's channels: the colour always; with `material`, roughness, metalness, opacity and glow. */
  private paintArgs(material: boolean): { color: string; roughness?: number; metalness?: number; opacity?: number; emissive?: number } {
    const s = this.shell;
    const out: { color: string; roughness?: number; metalness?: number; opacity?: number; emissive?: number } = { color: s.paintColor.value };
    if (material) {
      out.roughness = Number(s.paintRough.value) / 100;
      out.metalness = Number(s.paintMetal.value) / 100;
      out.opacity = Number(s.paintOpacityCh.value) / 100;
      out.emissive = Number(s.paintEmissive.value) / 100;
    }
    return out;
  }

  private constantSize(frac: number): number | null {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o) return null;
    const [a, b] = [o.bounds.min, o.bounds.max];
    return Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]) * frac * 0.05;
  }

  async refresh(): Promise<void> {
    const reply = await dispatch<SceneSummary>('get_scene');
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    await this.apply(reply.result);
    void this.drawUv(false);
    const history = await dispatch<{ steps: string[]; done: number }>('get_history');
    if (history.ok) this.patchHistory(history.result.steps, history.result.done);
    await this.patchSets();
  }

  /** The Selection Sets list for the active object; options are reused and only their text changes. */
  private async patchSets(): Promise<void> {
    const pick = this.shell.setPick;
    const id = this.scene.active;
    const reply = id === null ? null : await dispatch<{ sets: { name: string }[] }>('selection_set_list', { id });
    const names = reply?.ok ? reply.result.sets.map((x) => x.name) : [];
    const keep = names.includes(pick.value) ? pick.value : names[names.length - 1] ?? '';
    const rows = names.length ? names : [''];
    while (pick.options.length > rows.length) pick.lastElementChild?.remove();
    rows.forEach((name, i) => {
      let o = pick.options[i];
      if (!o) {
        o = el('option', '', '');
        pick.append(o);
      }
      if (o.value !== name) o.value = name;
      setText(o, name || t('None saved'));
    });
    if (pick.value !== keep) pick.value = keep;
    pick.disabled = !names.length;
  }

  /** One row per state: "Start", then each step; rows are reused so the list never rebuilds. */
  private patchHistory(steps: string[], done: number): void {
    const list = this.shell.historyList;
    const rows = ['Start', ...steps];
    while (list.children.length > rows.length) list.lastElementChild?.remove();
    rows.forEach((label, i) => {
      let row = list.children[i] as HTMLLIElement | undefined;
      if (!row) {
        row = el('li', 'step', '');
        list.append(row);
      }
      if (row.dataset.done !== String(i)) row.dataset.done = String(i);
      setText(row, label);
      setAttr(row, 'aria-current', i === done ? 'true' : 'false');
      setAttr(row, 'data-undone', i > done ? 'true' : 'false');
    });
    const current = list.children[done] as HTMLElement | undefined;
    if (current && (current.offsetTop < list.scrollTop || current.offsetTop > list.scrollTop + list.clientHeight - current.offsetHeight)) list.scrollTop = current.offsetTop - list.clientHeight / 2;
  }

  private async apply(next: SceneSummary): Promise<void> {
    // Under Live boolean its two objects are drawn as their result (renderer id 0) instead of themselves.
    const replaced = this.live ? [this.live.a, this.live.b] : [];
    for (const o of next.objects) {
      if (o.visible && !replaced.includes(o.id) && !this.viewport.renderer.has(o.id, o.revision)) {
        const mesh = await fetchMesh(o.id, this.smoothPreview);
        if (mesh) this.viewport.setMesh(mesh);
      }
    }
    const keep = new Set(next.objects.filter((o) => o.visible && !replaced.includes(o.id)).map((o) => o.id));
    if (this.live && (await this.updateLive(next))) keep.add(0);
    for (const id of this.viewport.renderer.ids()) if (!keep.has(id)) this.viewport.removeMesh(id);
    this.viewport.setActive(next.active);
    this.patchList(next);
    const active = next.objects.find((o) => o.id === next.active);
    setText(this.shell.statVerts, active ? active.vertices.toLocaleString('en-US') : '0');
    setText(this.shell.statTris, active ? active.triangles.toLocaleString('en-US') : '0');
    const leveled = active !== undefined && active.levels > 1;
    setText(this.shell.statLevel, leveled ? `${active.level} of ${active.levels - 1}` : 'none');
    const fixed = leveled || (active !== undefined && active.layers.length > 0);
    if (this.shell.dyntopo.disabled !== fixed) this.shell.dyntopo.disabled = fixed;
    this.patchLayers(active);
    this.patchPaintLayers(active);
    this.patchPrimitive(active);
    let restyled = false;
    for (const o of next.objects) {
      const m = o.material;
      restyled = this.viewport.renderer.setMaterial(o.id, m ? { subsurface: m.subsurface, subsurfaceColor: m.subsurface_color, subsurfaceRadius: m.subsurface_radius, transmission: m.transmission, ior: m.ior } : null) || restyled;
    }
    if (active?.material) {
      if (document.activeElement !== this.shell.subsurface) this.shell.subsurface.value = String(Math.round(active.material.subsurface * 100));
      if (document.activeElement !== this.shell.transmission) this.shell.transmission.value = String(Math.round(active.material.transmission * 100));
    }
    if (restyled) this.viewport.invalidate();
    if (next.active !== this.scene.active && this.scene.active !== null) this.previous = this.scene.active;
    this.scene = next;
    scheduleChrome();
  }

  private previous: number | null = null;

  private patchLayers(active: ObjectSummary | undefined): void {
    const select = this.shell.layerSelect;
    const layers = active?.layers ?? [];
    while (select.options.length > layers.length + 1) select.options[select.options.length - 1].remove();
    layers.forEach((layer, i) => {
      let option = select.options[i + 1];
      if (!option) {
        option = el('option', '', '');
        select.append(option);
      }
      if (option.value !== String(i)) option.value = String(i);
      setText(option, `${layer.name} (${Math.round(layer.strength * 100)}%)`);
    });
    const pick = active?.active_layer ?? null;
    const value = pick === null ? '' : String(pick);
    if (select.value !== value) select.value = value;
    const strength = this.shell.layerStrength;
    const disabled = pick === null;
    if (strength.disabled !== disabled) strength.disabled = disabled;
    if (pick !== null && document.activeElement !== strength) {
      const v = String(Math.round(layers[pick].strength * 100));
      if (strength.value !== v) strength.value = v;
    }
  }

  /** The Paint section's layer controls follow the active object's paint stack; options are reused in place. */
  private patchPaintLayers(active: ObjectSummary | undefined): void {
    const s = this.shell;
    const layers = active?.paint_layers ?? [];
    const select = s.paintLayer;
    while (select.options.length > layers.length + 1) select.options[select.options.length - 1].remove();
    // Shown top first, as a layer stack reads; values stay the engine's bottom-first indices.
    layers.forEach((layer, k) => {
      const i = layers.length - 1 - k;
      let option = select.options[k + 1];
      if (!option) {
        option = el('option', '', '');
        select.append(option);
      }
      if (option.value !== String(i)) option.value = String(i);
      const l = layers[i];
      setText(option, `${l.name} · ${l.blend} ${Math.round(l.opacity * 100)}%${l.visible ? '' : ' (hidden)'}`);
    });
    const pick = active?.active_paint_layer ?? null;
    const value = pick === null ? '' : String(pick);
    if (select.value !== value) select.value = value;
    const none = pick === null || !layers[pick];
    for (const c of [s.paintBlend, s.paintOpacity, s.paintVisible]) if (c.disabled !== none) c.disabled = none;
    if (!none) {
      const l = layers[pick];
      if (s.paintBlend.value !== l.blend) s.paintBlend.value = l.blend;
      const o = String(Math.round(l.opacity * 100));
      if (document.activeElement !== s.paintOpacity && s.paintOpacity.value !== o) s.paintOpacity.value = o;
      if (s.paintVisible.checked !== l.visible) s.paintVisible.checked = l.visible;
    }
  }

  private paintLayerAction(action: string): void {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o) return;
    if (action === 'up' || action === 'down') void this.run('paint_layer_set', { id: o.id, move: action });
    else if (action === 'paint_layer_add') void this.run('paint_layer_add', { id: o.id });
    else void this.run(action, { id: o.id });
  }

  private objectAction(action: string): void {
    const id = this.scene.active;
    if (id === null) return;
    const axis = this.shell.deformAxis.value;
    const angle = Number(this.shell.deformAngle.value) || 0;
    const res = Math.max(16, Math.min(320, Number(this.shell.resolution.value) || 128));
    if (action === 'bevel') {
      // The Angle or taper % field doubles as the bevel radius, as a percentage of the object's size.
      const o = this.scene.objects.find((x) => x.id === id);
      const size = o ? Math.max(...[0, 1, 2].map((k) => o.bounds.max[k] - o.bounds.min[k])) : 1;
      const pct = Math.max(1, Math.min(25, Math.abs(Number(this.shell.deformAngle.value)) || 5));
      void this.run('bevel_edges', { id, radius: (size * pct) / 100, edges: 'both', resolution: res });
      return;
    }
    if (action === 'bridge') {
      void this.bridgeOut(id);
      return;
    }
    if (action === 'cloth') {
      // The active object is the cloth; it falls onto every other visible object.
      void this.run('cloth_drape', { id, steps: 240 });
      return;
    }
    if (action === 'pose') {
      // Turns the unmasked part about the view's pivot (double-click sets it) by the angle, around the deform axis.
      const axisVec = axis === 'x' ? [1, 0, 0] : axis === 'y' ? [0, 1, 0] : [0, 0, 1];
      const o = this.scene.objects.find((x) => x.id === id);
      const size = o ? Math.max(...[0, 1, 2].map((k) => o.bounds.max[k] - o.bounds.min[k])) : 1;
      void this.run('pose', { id, pivot: this.viewport.camera.target, axis: axisVec, angle: angle || 30, falloff: size * 0.08 });
      return;
    }
    if (action === 'fibers') {
      void this.run('grow_fibers', { id, color: this.shell.paintColor.value });
      return;
    }
    if (action === 'render') {
      const shading = this.shell.shading.value === 'pbr' ? 'pbr' : 'clay';
      void this.download(`/api/engine/view?render=path&samples=64&bounces=4&width=768&height=768&shading=${shading}`, 'render.png');
      this.note('Path rendering 64 samples per pixel; the picture downloads when it is done');
      return;
    }
    if (action === 'substance') {
      void this.substanceOut(id);
      return;
    }
    if (action === 'substance_sync') {
      void this.substanceIn(id);
      return;
    }
    if (action === 'duplicate') void this.run('duplicate_object', { id });
    else if (action === 'split') void this.run('split_parts', { id });
    else if (action === 'merge') {
      const ids = [id, ...this.scene.objects.map((o) => o.id).filter((x) => x !== id)];
      if (ids.length < 2) this.note('Merge needs at least two objects');
      else void this.run('merge_objects', { ids });
    } else if ((action === 'union' || action === 'difference' || action === 'intersection') && this.live) {
      // Keeps the live result: the same two objects, with the operation pressed.
      const { a, b } = this.live;
      this.live = null;
      this.liveKey = '';
      this.shell.liveBoolean.value = '';
      void this.run('boolean', { a, b, op: action, resolution: res, blend: Math.max(0, Math.min(10, Number(this.shell.fuseBlend.value) || 0)) });
    } else if (action === 'union' || action === 'difference' || action === 'intersection') {
      const others = this.scene.objects.map((o) => o.id).filter((x) => x !== id);
      const b = this.previous !== null && others.includes(this.previous) ? this.previous : others.length === 1 ? others[0] : null;
      if (b === null) this.note('Select the object to combine with first, then the one to keep');
      else void this.run('boolean', { a: id, b, op: action, resolution: res, blend: Math.max(0, Math.min(10, Number(this.shell.fuseBlend.value) || 0)) });
    } else if (action === 'project') {
      const others = this.scene.objects.map((o) => o.id).filter((x) => x !== id);
      const source = this.previous !== null && others.includes(this.previous) ? this.previous : others.length === 1 ? others[0] : null;
      if (source === null) this.note('Select the object to take detail from first, then the one to project onto');
      else void this.run('project_detail', { id, source_id: source });
    } else if (action === 'folder') {
      void this.run('set_folder', { ids: [id], folder: this.shell.folderName.value.trim() });
    } else if (action === 'isolate_folder') {
      const folder = this.scene.objects.find((o) => o.id === id)?.folder ?? '';
      if (!folder) this.note('The active object is not in a folder; Put in folder first');
      else void this.run('set_visibility', { action: 'isolate', folder });
    } else if (action === 'align' || action === 'distribute') {
      if (this.scene.objects.length < 2) this.note('Align and Distribute need at least two objects');
      else void this.run('arrange_objects', { axis, mode: action === 'align' ? 'center' : 'distribute' });
    } else if (action === 'taper') void this.run('deform_object', { id, kind: 'taper', axis, amount: Math.max(-2, Math.min(2, angle / 100)) });
    else void this.run('deform_object', { id, kind: action, axis, toward: axis === 'x' ? 'y' : 'x', angle });
  }

  private async fileAction(action: string): Promise<void> {
    const id = this.scene.active;
    if (action === 'open') {
      this.shell.fileInput.click();
    } else if (action === 'save') {
      await this.download('/api/engine/project', 'scene.fsc');
    } else if (action === 'export') {
      const format = this.shell.exportFormat.value;
      await this.download(`/api/engine/export?format=${encodeURIComponent(format)}`, `formsculptor.${format}`);
    } else if (action === 'recover') {
      const reply = await postJson<unknown>('/api/engine/autosave/recover');
      this.note(reply.ok ? 'Recovered the last autosave' : reply.error.message);
      await this.refresh();
      if (reply.ok) this.frameActive();
    } else if (action === 'nodemode') {
      await this.sendToNodeMode();
    } else if (id === null) {
      this.note('Select an object first');
    } else if (action === 'repair') {
      await this.run('repair_mesh', { id });
    } else if (action === 'print') {
      const reply = await dispatch<PrintReport>('print_check', { id });
      if (!reply.ok) {
        this.note(reply.error.message);
        return;
      }
      const r = reply.result;
      const size = r.size_mm.map((v) => v.toFixed(1)).join(' x ');
      const wall = r.wall_mm.min === null ? 'no wall samples' : `thinnest wall ${r.wall_mm.min.toFixed(2)} mm`;
      this.note(r.printable ? `Printable: ${size} mm, ${Math.round(r.volume_mm3).toLocaleString('en-US')} mm3, ${wall}` : `Not printable: ${r.problems.join('; ')}`);
    }
  }

  private async bake(id: number): Promise<void> {
    this.note('Baking maps');
    const reply = await dispatch<{ size: number; source: string; covered: number; ms: number }>('bake_maps', { id, size: Number(this.shell.bakeSize.value) });
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    const r = reply.result;
    this.note(`Baked ${r.size} px maps from ${r.source} in ${(r.ms / 1000).toFixed(1)} s; Export GLB or Download maps`);
    await this.refresh();
  }

  private async download(path: string, name: string): Promise<void> {
    const result = await fetchFile(path);
    if (typeof result === 'string') {
      this.note(result);
      return;
    }
    if (inDesktop) {
      try {
        const saved = await saveNative(result, name);
        this.note(saved ? `Saved ${saved}` : '');
      } catch (err) {
        this.note(`Could not save: ${String(err)}`);
      }
      return;
    }
    saveBlob(result, name);
    this.note(`Saved ${name}`);
  }

  private async sendToNodeMode(): Promise<void> {
    if (this.scene.objects.length === 0) {
      this.note('The scene is empty; there is nothing to send');
      return;
    }
    const title = this.scene.objects.find((o) => o.id === this.scene.active)?.name ?? 'formsculptor';
    const shading = this.shell.shading.value;
    this.note('Sending to NodeMode');
    if (this.host.embedded) {
      const [model, preview] = await Promise.all([fetchFile('/api/engine/export?format=glb'), fetchFile(`/api/engine/view?width=768&height=768&background=transparent&shading=${encodeURIComponent(shading)}`)]);
      if (typeof model === 'string' || typeof preview === 'string') {
        this.note(typeof model === 'string' ? model : String(preview));
        return;
      }
      this.host.assign([
        { name: `${title}.glb`, mime: 'model/gltf-binary', width: 0, height: 0, dataUrl: await dataUrl(model) },
        { name: `${title}.png`, mime: 'image/png', width: 768, height: 768, dataUrl: await dataUrl(preview) },
      ]);
      this.note(`Sent ${title} to the canvas`);
      return;
    }
    const reply = await postBody<{ placed: boolean; detail?: string }>('/api/nodemode/send', { target: this.shell.nodemode.value.trim(), title, shading });
    if (!reply.ok) this.note(reply.error.message);
    else this.note(reply.result.placed ? `Sent ${title} to NodeMode` : reply.result.detail ?? 'Uploaded to NodeMode');
  }

  private async uploadChosen(): Promise<void> {
    const file = this.shell.fileInput.files?.[0];
    this.shell.fileInput.value = '';
    if (!file) return;
    const reply = await uploadFile<unknown>(file, file.name);
    this.note(reply.ok ? `Opened ${file.name}` : reply.error.message);
    await this.refresh();
    if (reply.ok) this.frameActive();
  }

  /** Dynamic subdivision levels for display (0 off); the sculpted meshes are not changed. */
  private smoothPreview = 0;

  private setSmoothPreview(levels: number): void {
    this.smoothPreview = levels;
    for (const o of this.scene.objects) if (o.visible) this.refetch(o.id);
    this.note(levels ? `Showing ${levels} level${levels > 1 ? 's' : ''} of smooth subdivision; the mesh you sculpt is unchanged` : '');
  }

  private spaceMouse: SpaceMouse | null = null;

  /** Connects a 3Dconnexion device over WebHID; its cap then pans, zooms and turns the view. */
  private async connectSpaceMouse(): Promise<void> {
    if (!SpaceMouse.supported()) {
      this.note('This browser has no WebHID for a 3D mouse; use Chrome or Edge');
      return;
    }
    const cam = this.viewport.camera;
    this.spaceMouse ??= new SpaceMouse((m) => {
      if (m.orbitX || m.orbitY) cam.orbit(m.orbitX, m.orbitY);
      if (m.panX || m.panY) cam.pan(m.panX, m.panY, this.shell.canvas.clientHeight);
      if (m.zoom !== 1) cam.zoom(m.zoom);
      this.viewport.invalidate();
    });
    try {
      this.note(`${await this.spaceMouse.connect()} connected: push to pan, lift or press to zoom, tilt and twist to turn`);
    } catch (err) {
      this.note(err instanceof Error ? err.message : 'The 3D mouse could not be opened');
    }
  }

  private bridgeTimer = 0;

  /** Sends the object to the bridge folder for another app, then checks every few seconds for that app's saves. */
  private async bridgeOut(id: number): Promise<void> {
    const reply = await postBody<{ ok: boolean; result?: { path: string }; error?: { message: string } }>('/api/engine/bridge', { id, format: 'obj' });
    const body = reply.ok ? reply.result : null;
    if (!body?.ok || !body.result) {
      this.note(body?.error?.message ?? (reply.ok ? 'The bridge failed' : reply.error.message));
      return;
    }
    this.note(`Open ${body.result.path} in the other app; saving there brings it back here`);
    if (this.bridgeTimer) return;
    this.bridgeTimer = window.setInterval(() => void this.bridgeSync(), 3000);
  }

  /** Writes a Substance 3D Painter package for the object to ~/.formsculptor/substance and says where. */
  private async substanceOut(id: number): Promise<void> {
    const reply = await postBody<{ ok: boolean; result?: { folder: string; files: string[] }; error?: { message: string } }>('/api/engine/substance', { id });
    const body = reply.ok ? reply.result : null;
    if (!body?.ok || !body.result) {
      this.note(body?.error?.message ?? (reply.ok ? 'The package was not written' : reply.error.message));
      return;
    }
    this.note(`Substance package in ${body.result.folder}: open its _low.obj in Painter (its README has the steps)`);
  }

  /** Reads Painter's exported BaseColor (and Roughness, Metallic) from the package folder onto the paint. */
  private async substanceIn(id: number): Promise<void> {
    const reply = await postBody<{ ok: boolean; result?: { vertices_painted: number }; error?: { message: string } }>('/api/engine/substance/sync', { id });
    const body = reply.ok ? reply.result : null;
    if (!body?.ok || !body.result) {
      this.note(body?.error?.message ?? (reply.ok ? 'Nothing came back from Painter' : reply.error.message));
      return;
    }
    this.note(`Painter's textures are on ${body.result.vertices_painted} vertices`);
    await this.refresh();
  }

  private async bridgeSync(): Promise<void> {
    const reply = await dispatch<{ updated: { name: string }[]; problems: { message: string }[]; bridges: unknown[] }>('bridge_sync', {});
    if (!reply.ok) return;
    if (reply.result.updated.length) {
      this.note(`${reply.result.updated.map((o) => o.name).join(', ')} came back from the other app`);
      await this.refresh();
    }
    if (reply.result.bridges.length === 0) {
      window.clearInterval(this.bridgeTimer);
      this.bridgeTimer = 0;
    }
  }

  private plugins: PluginInfo[] = [];

  /** Draws the chosen plugin's PARAMS as fields; values typed there go to the plugin as its args. */
  private drawPluginParams(): void {
    const box = this.shell.pluginParams;
    const plugin = this.plugins.find((p) => p.name === this.shell.pluginPick.value);
    const fields: HTMLLabelElement[] = [];
    for (const [key, spec] of Object.entries(plugin?.params ?? {})) {
      const label = spec.label || key;
      let control: HTMLInputElement | HTMLSelectElement;
      if (spec.type === 'choice') {
        control = choices(label, (spec.options ?? []).map((o) => [o, o]));
        if (spec.default !== undefined) control.value = String(spec.default);
      } else if (spec.type === 'boolean') {
        control = input('checkbox', { 'aria-label': label });
        control.checked = spec.default === true;
      } else if (spec.type === 'text') {
        control = input('text', { 'aria-label': label, value: String(spec.default ?? ''), spellcheck: 'false' });
      } else {
        const attrs: Record<string, string> = { 'aria-label': label, value: String(spec.default ?? 0), step: String(spec.step ?? (spec.type === 'integer' ? 1 : 'any')) };
        if (spec.min !== undefined) attrs.min = String(spec.min);
        if (spec.max !== undefined) attrs.max = String(spec.max);
        control = input('number', attrs);
      }
      control.dataset.param = key;
      control.dataset.kind = spec.type;
      fields.push(field(label, control));
    }
    box.replaceChildren(...fields);
  }

  private pluginArgs(): Record<string, unknown> {
    const args: Record<string, unknown> = {};
    for (const c of this.shell.pluginParams.querySelectorAll<HTMLInputElement | HTMLSelectElement>('[data-param]')) {
      const key = c.dataset.param as string;
      if (c.dataset.kind === 'boolean') args[key] = (c as HTMLInputElement).checked;
      else if (c.dataset.kind === 'number' || c.dataset.kind === 'integer') args[key] = c.dataset.kind === 'integer' ? Math.round(Number(c.value) || 0) : Number(c.value) || 0;
      else args[key] = c.value;
    }
    return args;
  }

  /** Lists the plugins the service finds (plugins/ beside the app and ~/.formsculptor/plugins), reusing options. */
  private async loadPlugins(): Promise<void> {
    let list: PluginInfo[] = [];
    // Plugins are Python files the service runs; the browser engine has none.
    if (!browserEngine()) {
      try {
        const res = await fetch('/api/plugins');
        if (res.ok) list = ((await res.json()) as { plugins: typeof list }).plugins;
      } catch {
        // the service is down; the list stays empty
      }
    }
    this.plugins = list;
    const select = this.shell.pluginPick;
    const entries: [string, string, string][] = list.length ? list.map((p) => [p.name, p.title, p.description]) : [['', 'No plugins found', '']];
    while (select.options.length > entries.length) select.options[select.options.length - 1].remove();
    entries.forEach(([value, text, title], i) => {
      let option = select.options[i];
      if (!option) {
        option = el('option', '', '');
        select.append(option);
      }
      option.value = value;
      option.title = title;
      setText(option, text);
    });
    this.drawPluginParams();
  }

  /** Runs the chosen plugin with its fields as args; a click plugin first arms the Plugin tool and runs on the click. */
  private async runPlugin(at?: Record<string, unknown>): Promise<void> {
    const name = this.shell.pluginPick.value;
    if (!name) return;
    const plugin = this.plugins.find((p) => p.name === name);
    if (plugin?.click && !at) {
      this.shell.brushButtons.find((b) => b.dataset.brush === 'plugin')?.click();
      this.note(`Click a surface to run ${plugin.title}`);
      return;
    }
    this.note(`Running ${this.shell.pluginPick.selectedOptions[0]?.textContent ?? name}`);
    const reply = await postBody<{ ok: boolean; output?: string; error?: { message: string } }>('/api/plugins/run', { name, args: { ...this.pluginArgs(), ...(at ?? {}) } });
    const body = reply.ok ? reply.result : { ok: false, error: reply.error };
    this.note(body.ok ? (body.output ?? '').trim().split('\n').pop() ?? '' : body.error?.message ?? 'The plugin failed');
    await this.refresh();
  }

  private projectMode: 'view' | 'triplanar' = 'view';

  /** Paints the chosen image onto the active object: from the current view (as seen on screen) or tiled. Any image
   * the browser reads is re-encoded as PNG for the engine. */
  private async projectImage(): Promise<void> {
    const file = this.shell.projectInput.files?.[0];
    this.shell.projectInput.value = '';
    const id = this.scene.active;
    if (!file || id === null) return;
    try {
      const bitmap = await createImageBitmap(file);
      const scaleDown = Math.min(1, 2048 / Math.max(bitmap.width, bitmap.height));
      const canvas = new OffscreenCanvas(Math.max(1, Math.round(bitmap.width * scaleDown)), Math.max(1, Math.round(bitmap.height * scaleDown)));
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('no 2D canvas');
      ctx.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      bitmap.close();
      const bytes = new Uint8Array(await (await canvas.convertToBlob({ type: 'image/png' })).arrayBuffer());
      let bin = '';
      for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      const args: Record<string, unknown> = { id, png_base64: btoa(bin), mode: this.projectMode, opacity: Number(this.shell.projectOpacity.value) / 100 };
      if (this.projectMode === 'view') args.view_proj = Array.from(this.viewport.camera.viewProjection());
      await this.run('paint_project', args);
    } catch {
      this.note('That image could not be read');
    }
  }

  /** The primitive fields follow the active object while it is an editable primitive. */
  private patchPrimitive(active: ObjectSummary | undefined): void {
    const s = this.shell;
    const p = active?.primitive ?? null;
    const off = p === null;
    for (const c of [s.primSize, s.primDetail, s.primHeight]) if (c.disabled !== off) c.disabled = off;
    if (!p) return;
    const set = (c: HTMLInputElement, v: unknown, d: number): void => {
      const text = String(typeof v === 'number' ? v : d);
      if (document.activeElement !== c && c.value !== text) c.value = text;
    };
    const kind = String(p.kind);
    set(s.primSize, p.size, 1);
    set(s.primDetail, p.detail, kind === 'sphere' ? 5 : kind === 'cube' || kind === 'plane' ? 16 : 64);
    set(s.primHeight, p.height, 2);
    const tall = kind === 'cylinder' || kind === 'cone';
    if (s.primHeight.disabled === tall) s.primHeight.disabled = !tall;
  }

  private async applyPrimitive(): Promise<void> {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o?.primitive) return;
    const s = this.shell;
    const args: Record<string, unknown> = { id: o.id, size: Number(s.primSize.value) || 1, detail: Math.round(Number(s.primDetail.value) || 0) };
    if (!s.primHeight.disabled) args.height = Number(s.primHeight.value) || 2;
    await this.run('edit_primitive', args);
  }

  /** The noise editor's settings as surface_noise arguments, sizes taken from the object's own size. */
  private noiseArgs(o: ObjectSummary): Record<string, unknown> {
    const s = this.shell;
    const { min, max } = o.bounds;
    const size = Math.hypot(max[0] - min[0], max[1] - min[1], max[2] - min[2]) || 1;
    const pct = (el2: HTMLInputElement, d: number, lo: number, hi: number): number => Math.max(lo, Math.min(hi, Number(el2.value) || d)) / 100;
    const args: Record<string, unknown> = {
      pattern: s.noisePattern.value,
      scale: size * pct(s.noiseSize, 2, 0.2, 50),
      amount: size * (Math.max(-10, Math.min(10, Number(s.noiseHeight.value) || 0)) / 100),
      octaves: Math.max(1, Math.min(8, Math.round(Number(s.noiseOctaves.value) || 3))),
      gain: Number(s.noiseGain.value) / 100,
    };
    const axis = { x: 0, y: 1, z: 2 }[s.noiseStretch.value as 'x' | 'y' | 'z'];
    if (axis !== undefined) {
      const stretch = [1, 1, 1];
      stretch[axis] = 4;
      args.stretch = stretch;
    }
    if (s.noisePattern.value === 'alpha') args.alpha = s.alpha.value === 'none' ? 'dots' : s.alpha.value;
    return args;
  }

  private layerAction(action: string): void {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o) return;
    if (action === 'layer_add') void this.run(action, { id: o.id });
    else if (action === 'surface_noise') void this.run(action, { id: o.id, ...this.noiseArgs(o) });
    else if (o.active_layer === null) this.note('Pick a layer under Record into first');
    else void this.run(action, { id: o.id, index: o.active_layer });
  }


  private patchList(next: SceneSummary): void {
    const keep = new Set(next.objects.map((o) => o.id));
    for (const [id, row] of this.rows) {
      if (!keep.has(id)) {
        row.remove();
        this.rows.delete(id);
      }
    }
    for (const o of next.objects) {
      let row = this.rows.get(o.id);
      if (!row) {
        row = el('li', 'object', '');
        row.dataset.id = String(o.id);
        row.append(el('span', 'name', o.name), el('span', 'count', ''));
        this.shell.objectList.append(row);
        this.rows.set(o.id, row);
      }
      setText(row.children[0], o.folder ? `${o.folder} / ${o.name}` : o.name);
      setText(row.children[1], `${o.triangles.toLocaleString('en-US')} tris`);
      setAttr(row, 'aria-current', o.id === next.active ? 'true' : 'false');
    }
  }

  private note(text: string): void {
    setText(this.shell.message, text);
  }

  private async run(name: string, args: Record<string, unknown> = {}): Promise<void> {
    const reply = await dispatch<unknown>(name, args);
    this.note(reply.ok ? '' : reply.error.message);
    await this.refresh();
  }

  /** Retopology corners placed so far, and the retopology object the next quads go into. */
  private retopoPoints: Vec3[] = [];
  private retopoInto: number | null = null;

  /** Live boolean: objects a and b drawn as `a op b`, recomputed when either changes; the key is what was drawn. */
  private live: { a: number; b: number; op: string } | null = null;
  private liveKey = '';

  /** Turns Live boolean on (the active object as a, the one chosen before it as b) or off (`op` empty). */
  private setLive(op: string): void {
    if (!op) {
      this.live = null;
      this.liveKey = '';
      this.note('');
      void this.refresh();
      return;
    }
    const a = this.scene.active;
    const others = this.scene.objects.map((o) => o.id).filter((x) => x !== a);
    const b = this.previous !== null && others.includes(this.previous) ? this.previous : others.length === 1 ? others[0] : null;
    if (a === null || b === null) {
      this.shell.liveBoolean.value = '';
      this.note('Select the object to combine with first, then the one to keep');
      return;
    }
    this.live = { a, b, op };
    this.liveKey = '';
    const button = OBJECT_ACTIONS.find(([k]) => k === op)?.[1] ?? op;
    this.note(`Live ${op}: move or sculpt either object and the result follows; press ${button} to keep it`);
    void this.refresh();
  }

  /** Recomputes the live result when either object changed; false (and Live boolean off) when it cannot be shown. */
  private async updateLive(next: SceneSummary): Promise<boolean> {
    const live = this.live;
    if (!live) return false;
    const [a, b] = [next.objects.find((o) => o.id === live.a), next.objects.find((o) => o.id === live.b)];
    const stop = (message: string): boolean => {
      this.live = null;
      this.liveKey = '';
      this.shell.liveBoolean.value = '';
      this.note(message);
      void this.refresh();
      return false;
    };
    if (!a || !b) return stop('Live boolean stopped: one of its objects is gone');
    const key = `${a.revision}:${b.revision}:${live.op}`;
    if (key === this.liveKey) return true;
    const result = await fetchBooleanPreview(live.a, live.b, live.op);
    if (typeof result === 'string') return stop(`Live boolean stopped: ${result}`);
    this.liveKey = key;
    this.viewport.setMesh(result);
    return true;
  }

  /** The island chosen in the UV view (null: none), and what the picture last showed, `id:revision:island`. */
  private uvIsland: number | null = null;
  private uvShown = '';

  /** Redraws the UV view for the active object when it changed (or always with force). */
  private async drawUv(force: boolean): Promise<void> {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    const has = o?.uvs === true;
    const key = o && has ? `${o.id}:${o.revision}:${this.uvIsland ?? ''}` : '';
    if (!force && key === this.uvShown) return;
    this.uvShown = key;
    this.shell.uvEmpty.hidden = has;
    if (!o || !has) {
      this.shell.uvImage.removeAttribute('src');
      this.shell.uvImage.hidden = true;
      return;
    }
    const args: Record<string, unknown> = { id: o.id, size: 512 };
    if (this.uvIsland !== null) args.island = this.uvIsland;
    const reply = await dispatch<{ png_base64: string; islands: number; selected: number | null }>('get_uv_layout', args);
    if (!reply.ok) return;
    if (this.uvIsland !== null && reply.result.selected === null) this.uvIsland = null;
    this.shell.uvImage.src = `data:image/png;base64,${reply.result.png_base64}`;
    this.shell.uvImage.hidden = false;
  }

  /** A pointer position over the UV view as [u, v] (v up). */
  private uvAt(e: PointerEvent): [number, number] {
    const r = this.shell.uvView.getBoundingClientRect();
    return [(e.clientX - r.left) / r.width, 1 - (e.clientY - r.top) / r.height];
  }

  /** Click chooses the island under the pointer; a drag moves the island it started on by the drag, Shift-drag one UV point. */
  private wireUvView(): void {
    const view = this.shell.uvView;
    let start: { x: number; y: number; uv: [number, number] } | null = null;
    view.addEventListener('pointerdown', (e) => {
      if (this.scene.active === null) return;
      view.setPointerCapture(e.pointerId);
      start = { x: e.clientX, y: e.clientY, uv: this.uvAt(e) };
    });
    view.addEventListener('pointerup', (e) => {
      const from = start;
      start = null;
      const id = this.scene.active;
      if (!from || id === null) return;
      const to = this.uvAt(e);
      if (Math.hypot(e.clientX - from.x, e.clientY - from.y) > 3) {
        // Shift drags the nearest UV point (Alt with a soft reach) instead of the whole island.
        if (e.shiftKey) void this.uvRun('uv_move_point', { id, at: from.uv, to, radius: e.altKey ? 0.06 : 0 });
        else void this.uvRun('uv_transform', { id, at: from.uv, translate: [to[0] - from.uv[0], to[1] - from.uv[1]] });
        return;
      }
      void dispatch<{ selected: number | null }>('get_uv_layout', { id, size: 64, at: from.uv }).then((reply) => {
        if (!reply.ok) return;
        this.uvIsland = reply.result.selected;
        this.note(this.uvIsland === null ? 'No island there' : `Island ${this.uvIsland} chosen`);
        void this.drawUv(false);
      });
    });
    view.addEventListener('pointercancel', () => {
      start = null;
    });
    this.shell.uvButtons.forEach((b) =>
      this.on(b, () => {
        const id = this.scene.active;
        if (id === null) return;
        const action = b.dataset.action;
        if (action === 'unwrap') void this.unwrapUvs(id);
        else if (action === 'pack') void this.uvRun('uv_pack', { id });
        else if (action === 'refresh') void this.drawUv(true);
        else if (this.uvIsland === null) this.note('Click an island in the UV view first');
        else if (action === 'rotate') void this.uvRun('uv_transform', { id, island: this.uvIsland, rotate: 90 });
        else void this.uvRun('uv_transform', { id, island: this.uvIsland, scale: action === 'grow' ? 1.25 : 0.8 });
      }),
    );
  }

  private async uvRun(name: string, args: Record<string, unknown>): Promise<void> {
    const reply = await dispatch<{ island?: number; coverage?: number }>(name, args);
    if (!reply.ok) this.note(reply.error.message);
    else if (name === 'uv_pack') this.note(`Packed; islands cover ${Math.round((reply.result.coverage ?? 0) * 100)}% of the square`);
    else {
      if (reply.result.island !== undefined) this.uvIsland = reply.result.island;
      this.note('');
    }
    await this.refresh();
  }

  /** Conformal unwrap, then says how many charts it made and how much it bends angles (1 is none). */
  private async unwrapUvs(id: number): Promise<void> {
    this.uvIsland = null;
    const reply = await dispatch<{ charts: number; stretch: { mean: number; max: number } }>('uv_unwrap', { id, seams: this.shell.uvSeams.value });
    this.note(reply.ok ? `Unwrapped into ${reply.result.charts} charts; angle distortion ${reply.result.stretch.mean.toFixed(2)} on average, ${reply.result.stretch.max.toFixed(2)} at worst` : reply.error.message);
    await this.refresh();
  }

  private async add(kind: string): Promise<void> {
    const reply = await dispatch<{ object: ObjectSummary }>('add_primitive', { kind });
    if (!reply.ok) {
      this.note(reply.error.message);
      return;
    }
    this.note('');
    const first = this.scene.objects.length === 0;
    await this.refresh();
    if (first) this.frameActive();
  }

  private frameActive(): void {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (o) this.viewport.frameBounds(o.bounds.min, o.bounds.max);
  }
}

async function poll(shell: Shell): Promise<void> {
  const version = await fetchVersion();
  if (version && browserEngine()) {
    setAttr(shell.engineDot, 'data-state', 'ok');
    setText(shell.engineText, 'browser engine');
    setText(shell.engineVersion, version.version);
  } else if (version) {
    setAttr(shell.engineDot, 'data-state', 'ok');
    setText(shell.engineText, 'service connected');
    setText(shell.engineVersion, version.version);
  } else {
    setAttr(shell.engineDot, 'data-state', 'down');
    setText(shell.engineText, 'service unreachable');
    setText(shell.engineVersion, 'offline');
  }
}

/**
 * Picks the engine before anything asks it: the engine service when it answers, otherwise the browser engine
 * (frontend/public/engine/fsc_engine.wasm) so the window still sculpts on its own. ?engine=wasm or ?engine=service
 * forces one.
 */
async function chooseEngine(): Promise<void> {
  const want = new URLSearchParams(location.search).get('engine');
  if (want === 'service') return;
  if (want !== 'wasm' && (await fetchVersion())) return;
  const engine = await WasmEngine.load();
  if (engine) useBrowserEngine(engine);
}

const NO_TYPING = new Set(['checkbox', 'radio', 'range', 'button', 'file', 'color', 'submit', 'reset']);

function typingIn(target: HTMLElement | null): boolean {
  if (!target) return false;
  if (target.isContentEditable || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) return true;
  return target instanceof HTMLInputElement && !NO_TYPING.has(target.type);
}

function wireChrome(root: HTMLElement, shell: Shell): void {
  const shortcuts = new ShortcutsDialog(commands);
  const about = new AboutDialog();
  const palette = new CommandPalette(commands, menuDefs);
  commands.add(
    { id: 'app.shortcuts', label: 'Keyboard Shortcuts', keys: ['Mod+/'], icon: 'keyboard', run: () => shortcuts.show() },
    { id: 'app.palette', label: 'Command Palette', keys: ['Mod+K'], icon: 'search', run: () => palette.show() },
    { id: 'app.about', label: 'About formsculptor', icon: 'info', run: () => about.show() },
    {
      id: 'view.clear_rail',
      label: 'Clear Tool Rail',
      run: () => {
        shell.layout.clearRail();
        setText(shell.message, 'Rail cleared');
      },
    },
  );
  const bar = new MenuBar(shell.menubar, commands, menuDefs());
  root.addEventListener('click', (e) => {
    const node = (e.target as HTMLElement).closest<HTMLElement>('[data-cmd]');
    if (node && !(node as HTMLButtonElement).disabled) commands.run(node.dataset.cmd ?? '');
  });
  root.addEventListener('change', scheduleChrome);
  commands.onChange(() => {
    applyShortcuts(commands);
    scheduleChrome();
  });
  window.addEventListener('keydown', (e) => {
    if (e.defaultPrevented || e.isComposing || e.repeat) return;
    const target = e.target instanceof HTMLElement ? e.target : null;
    if (target?.closest('.xterm, .dock')) return;
    if (overlayOpen() || bar.isOpen) return;
    const cmd = commands.forEvent(e);
    if (!cmd) return;
    const combo = comboOf(e);
    if (typingIn(target) && (!combo.includes('Mod+') || /Mod\+[ZYXCVA]$/.test(combo))) return;
    if (commands.run(cmd.id)) e.preventDefault();
  });
  scheduleChrome();
}

export function start(root: HTMLElement): void {
  const shell = buildShell(root);
  wireChrome(root, shell);
  document.body.classList.add('app-ready');
  void boot(shell);
}

async function boot(shell: Shell): Promise<void> {
  await chooseEngine();
  try {
    const viewport = new Viewport(shell.canvas);
    setText(shell.statRenderer, viewport.renderer.kind);
    const view = new SceneView(shell, viewport);
    view.registerCommands();
    view.wireContextMenus();
    const tools = document.querySelector<HTMLElement>('.appbar-tools');
    const root = document.getElementById('app');
    if (tools && root) {
      registerTerminal({
        root, tools,
        available: () => !browserEngine(),
        context: () => view.agentContext(),
        note: () => view.agentNote(),
        fail: (err) => setText(shell.message, err instanceof Error ? err.message : String(err)),
        activity: (tool) => view.agentNotice(tool),
        select: (ids) => { if (ids.length) void view.selectObject(ids[0]); },
      });
    }
    viewport.onFrame = (s) => {
      setText(shell.statFrame, `${s.frameMs.toFixed(1)} ms`);
      view.drawCurve();
      view.drawTools();
    };
    void view.refresh();
    // Agents reach the engine through the service's MCP server; the browser engine has none to follow.
    const overlay = browserEngine() ? null : document.getElementById('overlay-root');
    if (overlay) {
      const link = new AgentLink(overlay);
      link.onChange = () => void view.refresh();
      link.shouldWait = () => view.stroking();
      link.start();
    }
  } catch (err) {
    setText(shell.statRenderer, 'unavailable');
    setText(shell.message, `The viewport could not start: ${String(err)}`);
  }
  void poll(shell);
  if (!browserEngine()) window.setInterval(() => void poll(shell), 5000);
}
