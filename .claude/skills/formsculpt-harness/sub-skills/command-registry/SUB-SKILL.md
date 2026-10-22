---
name: command-registry
description: "Adds or changes a formsculpt command end to end through the fsc-commands registry: Rust args struct, JSON schema, MCP annotations, registry entry, catalogue.json, UI binding, agents sync and docs. Load when adding a tool, brush command, menu item, shortcut or MCP tool, or changing dispatch."
---

# Command Registry

## Purpose

Own `fsc-commands`, the single source of truth for every action. One registration yields the engine entry, the MCP tool, the palette item, the menu, the shortcut and the generated agent docs. Hands back a command that appears consistently on every surface.

## Preconditions

State: v0.1.0 scaffold; the registry is planned until code exists. If a surface (palette, MCP, menu) is hand-listing commands, that is a defect to fix at the registry.

## Procedure

1. Pick the id: dotted, in one of the groups scene, object, sculpt, mask, group, topo, layer, paint, view, file, history (for example `sculpt.stroke`). The MCP name is the snake_case form (`sculpt_stroke`).
2. Define the args struct in Rust. The JSON schema is derived from it; do not hand-write the schema.
3. Add MCP annotations (read-only, destructive, idempotent hints), a label and default keys.
4. Register the entry in `fsc-commands` so `dispatch(id, json)` reaches the handler. The handler calls into the owning crate.
5. Every mutating command is exactly one undo step. Read-only commands record none.
6. Export `catalogue.json` from the registry. The Python MCP server, the command palette, menus and shortcuts consume it. Never duplicate it by hand.
7. UI binding: the palette, menu and shortcut come from the catalogue; add an icon to `frontend/src/ui/icons.ts` only if a new one is needed (inline currentColor SVG).
8. Run `./formsculpt agents sync`. It regenerates the operator skill `formsculpt-modeling` (its `references/tools.md`) and mirrors it to `.agents/skills`. Do not edit generated files by hand.
9. Docs: add rationale to `native/HASHOUT_LOG.md`; add a CHANGELOG line when user visible.

## Gate

Done when `dispatch` runs the command with valid and invalid JSON, the catalogue contains it, the MCP tool list shows it, the palette shows it, and agents sync produced the new tools.md entry.

## Hands back

The command id, MCP name, files touched, and evidence of each surface.

## Owned resources

None bundled. Related: `sub-skills/agent-substrate/SUB-SKILL.md`, `sub-skills/ui-shell/SUB-SKILL.md`.
