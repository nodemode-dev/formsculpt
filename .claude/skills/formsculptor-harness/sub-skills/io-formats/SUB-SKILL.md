---
name: io-formats
description: "Covers fsc-io in formsculptor: OBJ, STL, PLY, glTF and GLB, 3MF, FBX import, and the .fsc project zip. Load when adding or fixing import or export, changing the project file layout, or handling file association and the backend io and projects routers."
---

# IO Formats

## Purpose

Own `fsc-io`. Reads and writes meshes and projects. Hands back a format change that round-trips and does not corrupt existing `.fsc` files.

## Preconditions

State: v0.1.0 scaffold; format support is planned until the code exists. FBX is import only.

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
