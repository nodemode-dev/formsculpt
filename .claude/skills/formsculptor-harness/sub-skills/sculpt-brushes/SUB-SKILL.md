---
name: sculpt-brushes
description: "Covers fsc-sculpt in formsculptor: stroke sampling, falloff curves, pressure, lazy rope, brush kernels, mirror and radial symmetry, and rayon parallelism. Load when adding or tuning a brush, changing stroke behaviour, symmetry, or sculpt performance."
---

# Sculpt Brushes

## Purpose

Own `fsc-sculpt`. Turns pointer input into vertex displacement through brush kernels. Hands back a brush or stroke change that is one undo step and exposed through the command registry.

## Preconditions

State: v0.1.0 scaffold, crate is a minimal shell; the pieces below are the intended design and are planned until the code exists. Read `sub-skills/engine-core/SUB-SKILL.md` for the buffers a kernel writes.

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
