---
name: io-formats
description: "Covers fsc-io in formsculptor: OBJ, STL, PLY, glTF and GLB, 3MF, FBX import, and the .fsc project zip. Load when adding or fixing import or export, changing the project file layout, or handling file association and the backend io and projects routers."
---

# IO Formats

## Purpose

Own `fsc-io`. Reads and writes meshes and projects. Hands back a format change that round-trips and does not corrupt existing `.fsc` files.

## Preconditions

State at 0.6.0: built in `native/crates/fsc-io`: `formats.rs` (OBJ, STL ASCII and binary, PLY ASCII and binary little-endian, 3MF read and write), `gltf.rs` (GLB and embedded glTF read with the node tree; GLB write with layers as morph targets, validated by the Khronos validator), `project.rs` (.fsc zip: manifest plus buffers for meshes, masks, level stacks and layers; reopens byte-identical). `fsc-commands/src/file_cmds.rs` holds save_project, open_project, import_mesh, export_mesh, print_check, repair_mesh and the byte entry points the Python module and the service routes use; the service autosaves to ~/.formsculptor/autosave. Since 0.14.0 `.fsc` also stores paint layers (`paint_layers` in the manifest: `paintbase.f32` at five floats and `paintlayerN.f32` at six per vertex) beside the composite `paint`. At 0.21.0 `gltf.rs` builds one document (`document`) that `write_glb` packs and `write_gltf` writes as a .gltf pointing at a .bin; readers take a `Resolve` for outside URIs, which `import_mesh` points at the .gltf's folder (no URLs or absolute paths) and byte imports refuse. Paint goes out as COLOR_0 with a material, baked maps as textures. Not built: FBX, big-endian PLY, outside image files on glTF import.

## Procedure

1. Formats: OBJ, STL, PLY, glTF/GLB, 3MF, FBX (import); the `.fsc` project is a zip.
2. A `.fsc` carries the scene plus per-object data (masks, groups, colour, layers). Version the layout, and keep readers tolerant of older versions.
3. Importers produce `fsc-core` buffers; exporters read them. No format code in other crates.
4. Large files reach the engine through the backend io router or the window; see `sub-skills/engine-service/SUB-SKILL.md`.
5. Register file commands in the `file` group via `sub-skills/command-registry/SUB-SKILL.md`.
6. The desktop `.fsc` file association is configured in `src-tauri/` (`sub-skills/desktop-shell/SUB-SKILL.md`).
7. Use permissively licensed parsers only.

## Gate

Done when export then import reproduces vertex count and topology, and an older `.fsc` still opens.

## Hands back

The format change and the round-trip evidence.

## Owned resources

None bundled.
