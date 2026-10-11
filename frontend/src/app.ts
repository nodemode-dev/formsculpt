import { VERSION } from './version';
import { ICONS } from './ui/icons';
import { el, saveBlob, setAttr, setText } from './ui/dom';
import { dispatch, fetchFile, fetchMesh, fetchVersion, postBody, postJson, uploadFile, type ObjectSummary, type SceneSummary } from './engine/service';
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
import type { ViewName } from './viewport/camera';
import { add, distance3, dot, normalize, scale, sub, type Vec3 } from './viewport/math';
import { LATHES, SKELETONS } from './ui/presets';
import { StrokeClient, type BrushState } from './engine/stroke-client';
import { AgentLink } from './ui/agent-link';
import { comboOf, commandActions, Hotkeys, WINDOW_ACTIONS, type CatalogueCommand } from './ui/hotkeys';

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
  keyAction: HTMLSelectElement;
  keyCombo: HTMLInputElement;
  keyButtons: HTMLButtonElement[];
  falloff: HTMLSelectElement;
  falloffCurve: SVGSVGElement;
  falloffLine: SVGPolylineElement;
  falloffDots: SVGPathElement;
  pressureGamma: HTMLInputElement;
  pressureStrength: HTMLInputElement;
  pressureSize: HTMLInputElement;
  curveButtons: HTMLButtonElement[];
  curveLine: SVGPolylineElement;
  curveHandles: SVGPathElement;
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
  ['mask_box', 'Box mask'],
  ['mask_lasso', 'Lasso mask'],
  ['mask_topo', 'Topo mask'],
  ['mask_group', 'Mask group'],
  ['hide_group', 'Hide group'],
  ['insert', 'Insert'],
];

/** Window-side tools that draw or click instead of streaming a stroke to the engine. */
const SHAPE_TOOLS = new Set(['mask_box', 'mask_lasso', 'stamp', 'dragdot', 'curve']);

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
const CLICK_TOOLS = new Set(['mask_topo', 'mask_group', 'hide_group', 'insert']);

const TOPOLOGY: [string, string, Record<string, unknown>][] = [
  ['subdivide', 'Subdivide', { levels: 1 }],
  ['remesh_voxel', 'Voxel remesh', {}],
  ['decimate', 'Decimate 50%', { ratio: 0.5 }],
  ['fill_holes', 'Fill holes', {}],
  ['quad_remesh', 'Quad remesh', {}],
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
];

/** Edit buttons whose engine call differs from their name. */
const ACTION_CALLS: Record<string, [string, Record<string, unknown>]> = {
  mask_grow: ['mask_grow', { steps: 1 }],
  mask_shrink: ['mask_grow', { steps: -1 }],
  mask_cavity: ['mask_cavity', { mode: 'cavity' }],
  mask_peaks: ['mask_cavity', { mode: 'peaks' }],
  group_mask: ['group_from_mask', {}],
  group_auto: ['group_auto', { by: 'angle', angle: 30 }],
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
  bar.append(brand, el('div', 'spacer'));

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
  overlay.append(tubeLine, curveLine, curveHandles);
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
  const fileInput = input('file', { accept: '.fsc,.obj,.stl,.ply,.glb,.gltf,.3mf', 'aria-label': 'Choose a file' });
  fileInput.hidden = true;
  panel.append(fileInput);
  const exportFormat = el('select');
  exportFormat.setAttribute('aria-label', 'Export format');
  for (const f of ['glb', 'obj', 'stl', 'ply', '3mf']) {
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

  panel.append(el('h2', 'gap', 'Build'));
  const skeleton = choices('Skeleton', Object.entries(SKELETONS).map(([k, v]) => [k, v.label]));
  const lathe = choices('Lathe profile', Object.entries(LATHES).map(([k, v]) => [k, v.label]));
  const tubeRadius = input('number', { min: '0.005', max: '1', step: '0.01', value: '0.05', 'aria-label': 'Tube radius' });
  const insertKind = choices('Insert shape', [['sphere', 'Sphere'], ['cube', 'Cube'], ['cylinder', 'Cylinder'], ['cone', 'Cone'], ['torus', 'Torus'], ['previous', 'Previous object']]);
  const insertSize = input('number', { min: '0.01', max: '10', step: '0.05', value: '0.2', 'aria-label': 'Insert size' });
  const arrayCount = input('number', { min: '2', max: '200', value: '6', 'aria-label': 'Array copies' });
  const buildFields = el('div', 'fields');
  buildFields.append(field('Skeleton', skeleton), field('Lathe profile', lathe), field('Tube radius', tubeRadius), field('Insert shape', insertKind), field('Insert size', insertSize), field('Array copies', arrayCount));
  panel.append(buildFields);
  const buildGrid = el('div', 'add-grid two');
  const buildButtons = (
    [
      ['skeleton', 'Build skeleton'],
      ['lathe', 'Lathe'],
      ['array_line', 'Array in line'],
      ['array_radial', 'Array radial'],
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

  panel.append(el('h2', 'gap', 'Paint'));
  const paintColor = input('color', { value: '#c8402f', 'aria-label': 'Paint colour' });
  const paintRough = input('range', { min: '0', max: '100', value: '50', 'aria-label': 'Paint roughness' });
  const paintMetal = input('range', { min: '0', max: '100', value: '0', 'aria-label': 'Paint metalness' });
  const paintMaterial = input('checkbox', { 'aria-label': 'Paint roughness and metalness' });
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
  paintFields.append(field('Colour', paintColor), field('Roughness', paintRough), field('Metalness', paintMetal), field('Paint material too', paintMaterial), field('Shading', shading), field('Bake size', bakeSize));
  panel.append(paintFields);
  const paintGrid = el('div', 'add-grid two');
  const paintButtons = (
    [
      ['paint_fill', 'Fill object'],
      ['paint_clear', 'Clear paint'],
      ['uv_unwrap', 'Unwrap UVs'],
      ['bake_maps', 'Bake maps'],
      ['maps', 'Download maps'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    paintGrid.append(b);
    return b;
  });
  panel.append(paintGrid);

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
  objectFields.append(field('Deform axis', deformAxis), field('Angle or taper %', deformAngle), field('Folder name', folderName));
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
  layerFields.append(field('Record into', layerSelect), field('Strength', layerStrength));
  panel.append(layerFields);
  const layerGrid = el('div', 'add-grid three');
  const layerButtons = LAYER_ACTIONS.map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    layerGrid.append(b);
    return b;
  });
  panel.append(layerGrid);

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
  viewFields.append(field('Field of view', fov), field('Face groups', showGroups), field('Trackball', trackball), field('Reference opacity', referenceOpacity), field('Shade active only', shadeActive));
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
      ['matcap', 'Load matcap'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    viewGrid.append(b);
    return b;
  });
  panel.append(viewGrid);

  panel.append(el('h2', 'gap', 'Keys'));
  const keyAction = choices('Key action', []);
  const keyCombo = input('text', { readonly: '', placeholder: 'Click, then press keys', 'aria-label': 'Shortcut', spellcheck: 'false' });
  const keyFields = el('div', 'fields');
  keyFields.append(field('Action', keyAction), field('Shortcut', keyCombo));
  panel.append(keyFields);
  const keyGrid = el('div', 'add-grid three after-fields');
  const keyButtons = (
    [
      ['set', 'Set key'],
      ['clear', 'Clear key'],
      ['reset', 'Reset keys'],
    ] as const
  ).map(([action, label]) => {
    const b = el('button', 'btn', label);
    b.type = 'button';
    b.dataset.action = action;
    keyGrid.append(b);
    return b;
  });
  panel.append(keyGrid);

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
  return { engineDot, engineText, engineVersion, canvas, addButtons, objectList, statVerts, statTris, statLevel, statRenderer, statFrame, message, brushButtons, radius, strength, invert, symX, radial, actionButtons, dyntopo, detail, detailMode, topoButtons, resolution, quadTarget, paintColor, paintRough, paintMetal, paintMaterial, shading, paintButtons, bakeSize, fileButtons, fileInput, exportFormat, nodemode, objectButtons, deformAxis, deformAngle, layerSelect, layerStrength, layerButtons, skeleton, lathe, tubeRadius, buildButtons, tubeLine, lazy, fov, viewButtons, historyList, showGroups, trackball, insertKind, insertSize, arrayCount, reference, referenceInput, referenceOpacity, folderName, alpha, alphaAngle, strokeMode, alphaInput, alphaLoad, matcapInput, shadeActive, keyAction, keyCombo, keyButtons, falloff, falloffCurve, falloffLine, falloffDots, pressureGamma, pressureStrength, pressureSize, curveButtons, curveLine, curveHandles };
}

class SceneView {
  private rows = new Map<number, HTMLLIElement>();
  private scene: SceneSummary = { objects: [], active: null };

  constructor(private shell: Shell, private viewport: Viewport) {
    shell.objectList.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (row?.dataset.id) void this.run('select_object', { id: Number(row.dataset.id) });
    });
    shell.addButtons.forEach((b) => b.addEventListener('click', () => void this.add(b.dataset.kind ?? 'sphere')));
    shell.canvas.addEventListener('keydown', (e) => this.key(e));
    shell.brushButtons.forEach((b) =>
      b.addEventListener('click', () => {
        this.brush = b.dataset.brush ?? 'draw';
        if (!SHAPE_TOOLS.has(this.brush) && !CLICK_TOOLS.has(this.brush) && this.brush !== 'orbit' && this.brush !== 'tube') this.lastBrush = this.brush;
        shell.brushButtons.forEach((x) => setAttr(x, 'aria-pressed', x === b ? 'true' : 'false'));
      }),
    );
    shell.actionButtons.forEach((b) =>
      b.addEventListener('click', () => {
        const action = b.dataset.action ?? 'undo';
        const mapped = ACTION_CALLS[action];
        if (mapped) void this.run(mapped[0], mapped[1]);
        else if (action === 'mask_color') void this.run('mask_by_color', { color: shell.paintColor.value, tolerance: 0.2 });
        else void this.run(action);
      }),
    );
    const applyOpacity = (): void => {
      shell.reference.style.opacity = String(Number(shell.referenceOpacity.value) / 100);
    };
    applyOpacity();
    shell.referenceOpacity.addEventListener('input', applyOpacity);
    shell.referenceInput.addEventListener('change', () => {
      const file = shell.referenceInput.files?.[0];
      shell.referenceInput.value = '';
      if (!file) return;
      if (shell.reference.src.startsWith('blob:')) URL.revokeObjectURL(shell.reference.src);
      shell.reference.src = URL.createObjectURL(file);
      shell.reference.hidden = false;
      this.note(`Reference: ${file.name}`);
    });
    shell.viewButtons.forEach((b) =>
      b.addEventListener('click', () => {
        const action = b.dataset.action ?? 'show_all';
        if (action === 'reference') {
          shell.referenceInput.click();
          return;
        }
        if (action === 'clear_reference') {
          if (shell.reference.src.startsWith('blob:')) URL.revokeObjectURL(shell.reference.src);
          shell.reference.hidden = true;
          shell.reference.removeAttribute('src');
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
    shell.canvas.addEventListener('dblclick', (e) => void this.pivot(e));
    shell.historyList.addEventListener('click', (e) => {
      const row = (e.target as HTMLElement).closest('li');
      if (row?.dataset.done !== undefined) void this.run('history_go_to', { done: Number(row.dataset.done) });
    });
    this.strokes.onUpdate = (u) => this.viewport.patchMesh(u);
    this.strokes.onEnd = (r) => {
      if (r) this.viewport.renderer.setRevision(r.id, r.revision);
      void this.refresh();
    };
    this.strokes.onError = (m) => this.note(m);
    this.strokes.onTopology = (id) => this.refetch(id);
    shell.topoButtons.forEach((b) =>
      b.addEventListener('click', () => {
        const args = JSON.parse(b.dataset.args ?? '{}') as Record<string, unknown>;
        if (b.dataset.action === 'remesh_voxel') args.resolution = Math.max(8, Math.min(320, Number(shell.resolution.value) || 128));
        if (b.dataset.action === 'quad_remesh') args.faces = Math.max(100, Math.min(400000, Number(shell.quadTarget.value) || 4000));
        void this.run(b.dataset.action ?? 'mesh_check', args);
      }),
    );
    shell.paintButtons.forEach((b) =>
      b.addEventListener('click', () => {
        const id = this.scene.active;
        if (id === null) return;
        const action = b.dataset.action;
        if (action === 'paint_clear' || action === 'uv_unwrap') void this.run(action, { id });
        else if (action === 'bake_maps') void this.bake(id);
        else if (action === 'maps') void this.download(`/api/engine/maps/${id}`, 'maps.zip');
        else void this.run('paint_fill', { id, ...this.paintArgs(true) });
      }),
    );
    shell.shading.addEventListener('change', () => this.applyShading());
    shell.curveButtons.forEach((b) =>
      b.addEventListener('click', () => {
        if (b.dataset.action === 'alpha') shell.alphaInput.click();
        else if (b.dataset.action === 'apply_curve') void this.applyCurve();
        else this.clearCurve();
      }),
    );
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
    shell.keyAction.addEventListener('change', () => this.showKeys());
    shell.keyCombo.addEventListener('keydown', (e) => {
      const combo = comboOf(e);
      e.preventDefault();
      e.stopPropagation();
      if (!combo) return;
      this.keyPending = combo;
      shell.keyCombo.value = combo;
    });
    shell.keyButtons.forEach((b) => b.addEventListener('click', () => this.keyButton(b.dataset.action ?? '')));
    void this.loadCatalogue();
    shell.fileButtons.forEach((b) => b.addEventListener('click', () => void this.fileAction(b.dataset.action ?? '')));
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
    shell.objectButtons.forEach((b) => b.addEventListener('click', () => this.objectAction(b.dataset.action ?? '')));
    shell.layerButtons.forEach((b) => b.addEventListener('click', () => this.layerAction(b.dataset.action ?? '')));
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
    shell.buildButtons.forEach((b) => b.addEventListener('click', () => void this.build(b.dataset.action ?? '')));
    const trace = (e: PointerEvent): void => {
      if (this.shape) {
        const p: [number, number] = [e.offsetX, e.offsetY];
        if (this.shape.kind === 'curve' && this.shape.pts.length === 0 && this.curve) {
          // Pressing on a handle of the waiting curve moves that handle instead of drawing a new curve.
          const near = this.curveScreen().findIndex((q) => q !== null && Math.hypot(q[0] - p[0], q[1] - p[1]) < 10);
          if (near >= 0) this.shape.edit = near;
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
      enabled: () => this.brush === 'tube' || (this.brush !== 'orbit' && this.scene.active !== null),
      begin: (ray, pressure, wpp, inv) => {
        if (this.brush === 'tube') this.tubeBegin(ray, wpp);
        else if (SHAPE_TOOLS.has(this.brush)) this.shape = { kind: this.brush, pts: [], subtract: inv, wpp };
        else if (CLICK_TOOLS.has(this.brush)) void this.clickTool(ray, wpp, inv);
        else if (this.scene.active !== null) this.strokes.begin(this.scene.active, this.brushState(), wpp, inv, ray.origin, ray.dir, pressure);
      },
      move: (ray, pressure) => {
        if (this.tube) this.tubeMove(ray);
        else if (!this.shape && !CLICK_TOOLS.has(this.brush)) this.strokes.move(ray.origin, ray.dir, pressure);
      },
      end: () => {
        if (this.tube) void this.tubeEnd();
        else if (this.shape) void this.shapeEnd();
        else if (!CLICK_TOOLS.has(this.brush)) this.strokes.end();
      },
      missed: () => (this.tube || this.shape || CLICK_TOOLS.has(this.brush) ? false : this.strokes.missed()),
    };
  }

  private brush = 'draw';
  private strokes = new StrokeClient();
  private host = new HostLink(() => undefined, () => undefined);
  private tube: { points: Vec3[]; step: number; anchor: Vec3; normal: Vec3; screen: string[] } | null = null;

  private shape: { kind: string; pts: [number, number][]; subtract: boolean; wpp: number; edit?: number } | null = null;
  /** The brush a curve stroke applies: the last sculpting or painting brush chosen before the Curve tool. */
  private lastBrush = 'draw';
  /** A curve stroke waiting to be applied: handles on the surface, and the world size of a pixel when it was drawn. */
  private curve: { handles: Vec3[]; wpp: number } | null = null;
  private hotkeys = new Hotkeys();
  private catalogue: CatalogueCommand[] = [];
  private keyPending: string | null = null;

  stroking(): boolean {
    return this.strokes.active() || this.tube !== null || this.shape !== null;
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
      this.shell.curveLine.setAttribute('points', '');
      this.shell.curveHandles.setAttribute('d', '');
      return;
    }
    const line = SceneView.spline(shown, 8);
    this.shell.curveLine.setAttribute('points', line.map((q) => `${q[0].toFixed(1)},${q[1].toFixed(1)}`).join(' '));
    this.shell.curveHandles.setAttribute('d', shown.map((q) => `M${(q[0] - 4).toFixed(1)},${q[1].toFixed(1)}a4,4 0 1,0 8,0a4,4 0 1,0 -8,0`).join(''));
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
      const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
      return hit.ok && hit.result.hit && hit.result.id === id && hit.result.point ? hit.result.point : null;
    };
    if (edit !== undefined && this.curve) {
      const q = pts[pts.length - 1];
      const point = q ? await pick(q) : null;
      if (point) this.curve.handles[edit] = point;
      else this.note('A handle must stay on the active object');
      this.drawCurve();
      return;
    }
    // Handles about every 40 pixels along the drawn path (2 to 32 of them).
    let length = 0;
    for (let i = 1; i < pts.length; i++) length += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
    if (pts.length < 2 || length < 8) {
      this.note('Drag across the object to draw a curve');
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
    this.curve = { handles, wpp };
    this.drawCurve();
    this.note(`Curve ready: ${handles.length} handles; drag one to bend it, then Apply curve`);
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
    let points = SceneView.spline(c.handles, per);
    if (points.length > 4096) points = points.filter((_, i) => i % Math.ceil(points.length / 4096) === 0);
    await this.run('sculpt_stroke', { ...this.brushArgs(radius, false), id, points });
  }

  private clearCurve(): void {
    this.curve = null;
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
    try {
      const res = await fetch('/api/engine/catalogue');
      if (res.ok) this.catalogue = ((await res.json()) as { commands: CatalogueCommand[] }).commands;
    } catch {
      // the service is down; the window's own keys still work
    }
    this.hotkeys.addCatalogue(this.catalogue);
    const actions = [...WINDOW_ACTIONS, ...BRUSHES.map(([k, label]) => ({ id: `brush:${k}`, label: `Tool: ${label}` })), ...commandActions(this.catalogue)];
    for (const a of actions) {
      const option = el('option', '', a.label);
      option.value = a.id;
      this.shell.keyAction.append(option);
    }
    this.showKeys();
  }

  private showKeys(): void {
    this.keyPending = null;
    this.shell.keyCombo.value = this.hotkeys.combosFor(this.shell.keyAction.value).join(' or ');
  }

  private keyButton(action: string): void {
    const target = this.shell.keyAction.value;
    if (action === 'reset') {
      this.hotkeys.reset();
      this.note('Keys reset to the defaults');
    } else if (action === 'clear' && target) {
      this.hotkeys.clear(target);
    } else if (action === 'set' && target) {
      if (!this.keyPending) {
        this.note('Click the Shortcut box and press the keys first');
        return;
      }
      const before = this.hotkeys.action(this.keyPending);
      this.hotkeys.bind(this.keyPending, target);
      this.note(before && before !== target ? `${this.keyPending} moved from ${before}` : `${this.keyPending} set`);
    }
    this.showKeys();
  }

  /** Runs the action a shortcut is bound to; false when it is bound to nothing. */
  private runAction(action: string): boolean {
    const [kind, name, step] = action.split(':');
    if (kind === 'view') {
      if (name === 'ortho') this.viewport.toggleOrtho();
      else if (name === 'frame') this.frameActive();
      else this.viewport.snap(name as ViewName);
    } else if (kind === 'file') void this.fileAction(name);
    else if (kind === 'brush') {
      const b = this.shell.brushButtons.find((x) => x.dataset.brush === name);
      if (!b) return false;
      b.click();
    } else if (kind === 'cmd') {
      const command = this.catalogue.find((c) => c.name === name);
      const args: Record<string, unknown> = {};
      const takesId = command ? 'id' in (command.inputSchema?.properties ?? {}) : name === 'delete_object';
      if (takesId && this.scene.active !== null) args.id = this.scene.active;
      if (takesId && command?.inputSchema?.required?.includes('id') && args.id === undefined) return true;
      if (step) args.step = step;
      void this.run(name, args);
    } else return false;
    return true;
  }

  /** Tools that act once on the clicked point: topology mask, group mask, hide group, insert. */
  private async clickTool(ray: Ray, wpp: number, ctrl: boolean): Promise<void> {
    const id = this.scene.active;
    if (id === null) return;
    const tool = this.brush;
    const hit = await dispatch<{ hit: boolean; id?: number; point?: Vec3; normal?: Vec3 }>('pick', { origin: ray.origin, direction: ray.dir });
    const onActive = hit.ok && hit.result.hit && hit.result.point !== undefined && (tool === 'insert' || hit.result.id === id);
    if (!hit.ok || !onActive || !hit.result.point) {
      this.note(tool === 'insert' ? 'Click on a surface to insert there' : 'Click on the active object');
      return;
    }
    const point = hit.result.point;
    const op = ctrl ? 'subtract' : 'add';
    if (tool === 'mask_topo') await this.run('mask_topology', { id, point, distance: Math.max(1e-4, Number(this.shell.radius.value) * wpp * 2), op });
    else if (tool === 'mask_group') await this.run('mask_by_group', { id, point, op });
    else if (tool === 'hide_group') await this.run('hide_faces', { id, action: ctrl ? 'isolate' : 'hide', point });
    else {
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
    const radius = Math.max(0.005, Math.min(1, Number(this.shell.tubeRadius.value) || 0.05));
    const points = t.points.length > 4096 ? t.points.filter((_, i) => i % Math.ceil(t.points.length / 4096) === 0) : t.points;
    await this.run('curve_tube', { points, radius, smooth: 2 });
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
    } else if (action === 'array_line' || action === 'array_radial') {
      const id = this.scene.active;
      if (id === null) {
        this.note('Select the object to repeat first');
        return;
      }
      const count = Math.max(2, Math.min(200, Math.round(Number(this.shell.arrayCount.value) || 6)));
      await this.run('array_object', { id, count, mode: action === 'array_line' ? 'linear' : 'radial', axis: 'y' });
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
    void fetchMesh(id).then((mesh) => {
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

  private paintArgs(material: boolean): { color: string; roughness?: number; metalness?: number } {
    const s = this.shell;
    const out: { color: string; roughness?: number; metalness?: number } = { color: s.paintColor.value };
    if (material) {
      out.roughness = Number(s.paintRough.value) / 100;
      out.metalness = Number(s.paintMetal.value) / 100;
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
    const history = await dispatch<{ steps: string[]; done: number }>('get_history');
    if (history.ok) this.patchHistory(history.result.steps, history.result.done);
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
    for (const o of next.objects) {
      if (o.visible && !this.viewport.renderer.has(o.id, o.revision)) {
        const mesh = await fetchMesh(o.id);
        if (mesh) this.viewport.setMesh(mesh);
      }
    }
    const keep = new Set(next.objects.filter((o) => o.visible).map((o) => o.id));
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
    if (next.active !== this.scene.active && this.scene.active !== null) this.previous = this.scene.active;
    this.scene = next;
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

  private objectAction(action: string): void {
    const id = this.scene.active;
    if (id === null) return;
    const axis = this.shell.deformAxis.value;
    const angle = Number(this.shell.deformAngle.value) || 0;
    const res = Math.max(16, Math.min(320, Number(this.shell.resolution.value) || 128));
    if (action === 'duplicate') void this.run('duplicate_object', { id });
    else if (action === 'split') void this.run('split_parts', { id });
    else if (action === 'merge') {
      const ids = [id, ...this.scene.objects.map((o) => o.id).filter((x) => x !== id)];
      if (ids.length < 2) this.note('Merge needs at least two objects');
      else void this.run('merge_objects', { ids });
    } else if (action === 'union' || action === 'difference' || action === 'intersection') {
      const others = this.scene.objects.map((o) => o.id).filter((x) => x !== id);
      const b = this.previous !== null && others.includes(this.previous) ? this.previous : others.length === 1 ? others[0] : null;
      if (b === null) this.note('Select the object to combine with first, then the one to keep');
      else void this.run('boolean', { a: id, b, op: action, resolution: res });
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

  private layerAction(action: string): void {
    const o = this.scene.objects.find((x) => x.id === this.scene.active);
    if (!o) return;
    if (action === 'layer_add' || action === 'surface_noise') void this.run(action, { id: o.id });
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

  private key(e: KeyboardEvent): void {
    const combo = comboOf(e);
    const action = combo ? this.hotkeys.action(combo) : null;
    if (!action || !this.runAction(action)) return;
    e.preventDefault();
  }
}

async function poll(shell: Shell): Promise<void> {
  const version = await fetchVersion();
  if (version) {
    setAttr(shell.engineDot, 'data-state', 'ok');
    setText(shell.engineText, 'service connected');
    setText(shell.engineVersion, version.version);
  } else {
    setAttr(shell.engineDot, 'data-state', 'down');
    setText(shell.engineText, 'service unreachable');
    setText(shell.engineVersion, 'offline');
  }
}

export function start(root: HTMLElement): void {
  const shell = buildShell(root);
  document.body.classList.add('app-ready');
  try {
    const viewport = new Viewport(shell.canvas);
    setText(shell.statRenderer, viewport.renderer.kind);
    const view = new SceneView(shell, viewport);
    viewport.onFrame = (s) => {
      setText(shell.statFrame, `${s.frameMs.toFixed(1)} ms`);
      view.drawCurve();
    };
    void view.refresh();
    const overlay = document.getElementById('overlay-root');
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
  window.setInterval(() => void poll(shell), 5000);
}
