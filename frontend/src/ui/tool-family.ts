/**
 * Tool families, by what a tool does: the tablet palette's discs and the panel's tool buttons are coloured by family
 * (sculpt blue, mask grey, paint green, cut red, build yellow, move purple, view cyan), so tools are found by colour.
 */

export type Family = 'sculpt' | 'mask' | 'paint' | 'cut' | 'build' | 'move' | 'view';

const FAMILY: Record<string, Family> = {
  mask: 'mask', mask_box: 'mask', mask_lasso: 'mask', mask_topo: 'mask', mask_group: 'mask', hide_group: 'mask', group_brush: 'mask',
  paint: 'paint', smudge: 'paint', eyedropper: 'paint',
  trim: 'cut', cut: 'cut', trim_shape: 'cut', slice_curve: 'cut', trim_curve: 'cut', clip: 'cut', bevel_edge: 'cut',
  tube: 'build', curve: 'build', insert: 'build', insert_brush: 'build', vdm: 'build', zsphere: 'build', retopo: 'build', retopo_patch: 'build', edge_loop: 'build', zmodeler: 'build', silhouette: 'build',
  gizmo: 'move', lattice: 'move', pose_joint: 'move', grab: 'move', elastic: 'move', nudge: 'move', snake_hook: 'move',
  orbit: 'view', groom_comb: 'view', groom_cut: 'view', plugin: 'view',
};

/** A tool's family by its brush kind; any tool not listed sculpts. */
export const familyOf = (kind: string): Family => FAMILY[kind] ?? 'sculpt';
