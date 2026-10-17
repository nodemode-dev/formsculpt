import { commands } from './commands';
import type { MenuDef, MenuItem } from './menu';

const tool = (...kinds: string[]): string[] => kinds.map((k) => `tool.brush.${k}`);

export function tidy(items: MenuItem[], has: (id: string) => boolean = (id) => !!commands.get(id)): MenuItem[] {
  const kept: MenuItem[] = [];
  for (const item of items) {
    if (item === '-') {
      if (kept.length && kept[kept.length - 1] !== '-') kept.push(item);
    } else if (typeof item === 'string') {
      if (has(item)) kept.push(item);
    } else {
      const inner = tidy(item.items, has);
      if (inner.length) kept.push({ label: item.label, items: inner });
    }
  }
  while (kept.length && kept[kept.length - 1] === '-') kept.pop();
  return kept;
}

const EXPORTS = ['glb', 'obj', 'fbx', 'usda', 'stl', 'ply', '3mf'].map((f) => `file.export.${f}`);

const FILE: MenuItem[] = [
  'scene.new', 'file.open', 'file.save', '-',
  { label: 'Export As', items: EXPORTS }, 'file.export', '-',
  'file.print_check', 'file.repair', 'file.recover', '-',
  'file.send_nodemode', 'file.bridge_send', 'file.substance', 'file.substance_sync',
];

const EDIT: MenuItem[] = [
  'history.undo', 'history.redo', '-',
  'object.duplicate', 'object.delete', 'object.rename', '-',
  { label: 'Transform', items: [...tool('gizmo', 'lattice'), '-', 'object.bend', 'object.twist', 'object.taper', '-', 'object.align', 'object.distribute'] },
  { label: 'Channels', items: ['channel.copy_mask', 'channel.paste_mask', 'channel.clear_mask', '-', 'channel.copy_groups', 'channel.paste_groups', 'channel.clear_groups', '-', 'channel.copy_paint', 'channel.paste_paint', 'channel.clear_paint'] }, '-',
  'app.palette', 'app.shortcuts',
];

const ADD: MenuItem[] = [
  'add.sphere', 'add.cube', 'add.cylinder', 'add.cone', 'add.torus', 'add.plane', '-',
  'add.apply_primitive', '-',
  { label: 'Build', items: ['build.skeleton', 'build.lathe', 'build.text', 'build.shadow_box', 'build.clear_outlines', 'build.extrude_outline', 'sculpt.vdm_capture'] },
  { label: 'Array', items: ['build.array_line', 'build.array_radial', 'build.array_mirror', 'build.array_curve'] }, '-',
  ...tool('tube', 'insert', 'insert_brush', 'vdm'),
];

const SCULPT: MenuItem[] = [
  ...tool('orbit', 'draw', 'clay', 'inflate', 'pinch', 'crease', 'flatten', 'smooth', 'grab', 'relax', 'planar', 'trim', 'snake_hook', 'noise', 'smudge', 'elastic', 'polish', 'layer', 'nudge', 'layer_reset'), '-',
  ...tool('stamp', 'dragdot', 'curve', 'silhouette'), '-',
  'sculpt.apply_curve', 'sculpt.clear_curve', 'curve.edit_last_stroke', 'curve.from_mask_edge', 'curve.from_group_border', 'curve.snake_mask', 'curve.bridge_groups', 'sculpt.alpha_load', '-',
  'sculpt.mirror_x', 'sculpt.invert', 'sculpt.dyntopo',
];

const MASK: MenuItem[] = [
  ...tool('mask', 'mask_box', 'mask_lasso', 'mask_topo', 'mask_group'), '-',
  'mask.invert', 'mask.clear', 'mask.blur', 'mask.sharpen', 'mask.grow', 'mask.shrink', 'mask.cavity', 'mask.peaks', 'mask.color', '-',
  'group.from_mask', 'group.auto', 'group.by_paint', 'group.by_uv', ...tool('group_brush', 'slice_curve'), '-',
  'faces.hide_masked', 'faces.invert_hidden', 'faces.show', ...tool('hide_group'), '-',
  'mask.extract', 'mask.extrude', 'mask.inset', '-',
  { label: 'Selection Sets', items: ['selection.save', 'selection.restore', 'selection.delete'] },
];

const TOPOLOGY: MenuItem[] = [
  'topo.subdivide', 'topo.remesh_voxel', 'topo.decimate', 'topo.fill_holes', 'topo.quad_remesh', 'topo.field_remesh', 'topo.delete_faces', 'topo.bridge', 'topo.bevel', 'topo.project', 'topo.check', '-',
  ...tool('retopo', 'edge_loop', 'bevel_edge', 'clip', 'cut', 'trim_shape', 'trim_curve', 'zsphere'), '-',
  { label: 'Multires', items: ['multires.divide', 'multires.higher', 'multires.lower', 'multires.delete_lower', 'multires.delete_higher', 'multires.reconstruct'] },
];

const OBJECT: MenuItem[] = [
  'object.duplicate', 'object.split', 'object.merge', 'object.rename', '-',
  'object.union', 'object.difference', 'object.intersection', '-',
  'object.align', 'object.distribute', 'object.folder', 'object.isolate_folder', '-',
  'object.cloth_drape', 'object.pose', 'object.fibers', ...tool('pose_joint', 'groom_comb', 'groom_cut'), '-',
  { label: 'Layers', items: ['layer.add', 'layer.apply', 'layer.delete', 'layer.surface_noise'] }, '-',
  'object.hide', 'object.isolate', 'object.show_all', '-', 'object.delete',
];

const PAINT: MenuItem[] = [
  ...tool('paint'), 'paint.fill', 'paint.clear', '-',
  'paint.unwrap', 'paint.uv_pack', 'uv.unwrap', 'uv.rotate', 'uv.scale_up', 'uv.scale_down', 'uv.redraw', 'uv.relax_view', 'uv.equalize_view', 'uv.density_view', 'uv.sew_view', 'uv.cut_path', 'uv.straighten_path', 'uv.clear_marks', '-',
  'paint.bake', 'paint.download_maps', 'paint.project', 'paint.project_tile', '-',
  { label: 'Paint Layers', items: ['paint.layer_add', 'paint.layer_delete', 'paint.layer_merge', 'paint.layer_flatten', 'paint.layer_up', 'paint.layer_down'] },
];

const VIEW: MenuItem[] = [
  'view.front', 'view.back', 'view.right', 'view.left', 'view.top', 'view.bottom', 'view.ortho', 'view.frame', 'view.set_pivot', '-',
  'view.groups', 'view.uv_checker', 'view.trackball', 'view.floor_grid', 'view.wireframe', 'view.xray', 'view.backdrop_gradient', { label: 'Smooth Preview', items: ['view.smooth_0', 'view.smooth_1', 'view.smooth_2', 'view.smooth_3'] }, '-',
  'view.reference', 'view.clear_reference', 'view.matcap', 'light.load_hdri', 'light.add', 'light.remove', '-',
  'view.turntable', 'view.history_movie', 'view.path_render', 'view.anim_key', 'view.anim_record', 'view.anim_clear', 'view.spacemouse',
];

const WINDOW: MenuItem[] = [
  'view.tablet_layout', 'view.panel_left', { label: 'Interface Size', items: ['view.ui_small', 'view.ui_normal', 'view.ui_large'] }, 'view.clear_rail', 'plugin.run', 'plugin.reload', '-',
  'view.terminal', 'view.newTerminal', 'agent.claude', 'agent.chat',
];

const HELP: MenuItem[] = ['app.shortcuts', 'app.palette', '-', 'app.about'];

export const VIEWPORT_MENU: MenuItem[] = [
  'history.undo', 'history.redo', '-', 'view.frame', 'view.set_pivot', '-',
  'mask.invert', 'mask.clear', 'mask.blur', 'mask.grow', 'mask.shrink', '-',
  'faces.hide_masked', 'faces.show', 'object.hide', 'object.isolate', 'object.show_all', '-',
  'object.duplicate', 'object.delete', '-', 'sculpt.mirror_x',
];

export const OBJECT_ROW_MENU: MenuItem[] = ['object.rename', 'object.duplicate', 'object.delete', '-', 'object.hide', 'object.isolate', 'object.show_all', '-', 'object.merge'];

export const PIN_MENU = (id: string): MenuItem[] => [id, '-', 'rail.pin'];

export function menuDefs(): MenuDef[] {
  const def = (label: string, items: MenuItem[]): MenuDef => ({ label, items: () => tidy(items) });
  return [
    def('File', FILE), def('Edit', EDIT), def('Add', ADD), def('Sculpt', SCULPT), def('Mask', MASK), def('Topology', TOPOLOGY),
    def('Object', OBJECT), def('Paint', PAINT), def('View', VIEW), def('Window', WINDOW), def('Help', HELP),
  ];
}
