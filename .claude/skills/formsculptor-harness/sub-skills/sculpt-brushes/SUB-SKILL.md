---
name: sculpt-brushes
description: "Covers fsc-sculpt in formsculptor: stroke sampling, falloff curves, pressure, lazy rope, brush kernels, mirror and radial symmetry, and rayon parallelism. Load when adding or tuning a brush, changing stroke behaviour, symmetry, or sculpt performance."
---

# Sculpt Brushes

## Purpose

Own `fsc-sculpt`. Turns pointer input into vertex displacement through brush kernels. Hands back a brush or stroke change that is one undo step and exposed through the command registry.

## Preconditions

State at 0.4.0: built in `native/crates/fsc-sculpt` (`stroke.rs`, `brush.rs`, `symmetry.rs`): nine brushes, spacing, pressure, four falloff presets, lazy smoothing, mirror and radial symmetry, masks, grab, and dynamic topology through `fsc-topo::refine_sphere`. A stroke finishes into `StrokeEdit::Vertices` or, when topology changed, `StrokeEdit::Topology` with the pre-stroke snapshot. At 0.7.0 a tenth brush, `paint`, blends `BrushSettings.paint` (colour, roughness, metalness) into `Mesh.paint` for the channels in `paint_channels`, held by the mask; `BrushKind::moves_vertices` separates it and mask from the brushes that need normals and a BVH refit; `VertexEdit` carries paint before and after for undo. Dyntopo collapse joined refinement in 0.4.1. At 0.10.0: `relax` (tangential only), `planar` (plane locked at the first dab in `Stroke.planes`, one per symmetry copy, so `dab` takes the copy index), `trim` (only above the dab's plane), `snake_hook` (handled in `sample_point`: `snake_dab` moves the area around the previous centre by the step; after the first hit `sample_ray` follows a camera-facing plane; the core must follow fully or the hook stalls) and `noise` (`fbm`, also used by `surface_noise`); `lazy` is a rope of lazy times three radii and returns early while slack. At 0.12.0: `alpha.rs` (`Alpha`, seven generated built-ins, `from_pixels` for loaded PNGs, `turn`); `Stroke.alpha` multiplies each dab's falloff weight by the alpha sampled in the dab's frame (up from `alpha_up` or, with `roll`, the stroke direction from `prev_dab`, turned by `alpha_angle`), so a new brush gets alphas for free as long as it uses `weights`; `smudge` (paint only, excluded from `moves_vertices` and from `refine`) carries colour in `Stroke.smudge`. Loaded alphas live in `Session.alphas` and are resolved by `fsc_commands::stroke_alpha` for both `sculpt_stroke` and the window's `stroke_begin`. Items below not yet built are marked by their absence from those files. Read `sub-skills/engine-core/SUB-SKILL.md` for the buffers a kernel writes.

## Procedure

1. Pipeline order: stroke sampling (spacing along the path), lazy rope smoothing, pressure mapping, falloff weight, brush kernel, symmetry expansion, write with dirty range.
2. A kernel is a pure function of the affected vertices, brush parameters and mask. Respect masks and layers.
3. Mirror and radial symmetry expand one dab into several before the kernel runs.
4. Use rayon over the affected vertex set; keep results deterministic so benches compare.
5. Do not copy code from ZBrush or Nomad, and do not lift GPL code (Blender is study only).
6. Expose the brush by adding a command (`sculpt.stroke` is the stroke entry point) through `sub-skills/command-registry/SUB-SKILL.md`.
7. Brush parity targets are ZBrush 2026.2.1 and Nomad Sculpt 2.9.24 behaviour, measured, not copied.

## Gate

Done when a stroke on a test mesh changes only the expected region, respects the mask, is undone in one step, and the perf bench shows no regression (`sub-skills/verification/SUB-SKILL.md`).

## Hands back

The kernel or stroke change, the command that exposes it, and bench output.

## Owned resources

None bundled.
