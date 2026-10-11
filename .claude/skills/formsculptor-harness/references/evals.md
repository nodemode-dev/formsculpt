# Evaluations and Trigger List

## Contents

- Evaluations
- Should trigger
- Should not trigger

## Evaluations

### Eval 1: add a brush end to end

Prompt: "Add a Clay Buildup brush to formsculptor."
Expected observable behaviour:
- Loads sculpt-brushes, then command-registry, then verification.
- Puts the kernel in fsc-sculpt and registers a `sculpt.*` command with snake_case MCP name, schema derived from the args struct, annotations, label and keys.
- Runs `./formsculptor agents sync` rather than editing tools.md by hand.
- Does not copy ZBrush, Nomad or GPL code, and logs rationale in `native/HASHOUT_LOG.md` instead of comments.
- States that fsc-sculpt is a minimal shell at v0.1.0 and marks unbuilt parts planned.

### Eval 2: restyle a panel under the laws

Prompt: "Add a floating layers panel with a close icon over the viewport."
Expected observable behaviour:
- Loads ui-shell and verification.
- Uses a `var(--z-*)` token for stacking, colours from tokens.css, and an inline currentColor SVG added to icons.ts.
- Updates the DOM surgically with no full re-render, edits by hand (no sed or scripts).
- Runs or plans the throttled DOM check and reports it.

### Eval 3: MCP call with no window open

Prompt: "An agent calls sculpt_stroke but the app is not open. What happens and how do we test it?"
Expected observable behaviour:
- Loads agent-substrate.
- Explains the headless path: backend mcp router runs the same Rust dispatch through the fsc-py PyO3 module; with a window it would use the long-poll bridge after consent.
- Names the stdio shim `./formsculptor mcp` autostart and POST `/mcp` with protocol versions 2026-07-28 and 2025-11-25.
- Notes one undo step per mutating command and flags the path as planned if code is absent.

## Should trigger

- Add a smooth brush to formsculptor
- Why is dyntopo leaving holes in the mesh in fsc-topo
- Bump formsculptor to v0.2.0 everywhere
- Wire a new command into the registry and MCP catalogue
- The viewport does not upload the dirty range after a stroke
- Fix the Tauri sidecar not being killed on exit
- Import glTF into formsculptor and save as .fsc
- Edit native/fsc-core adjacency rebuild
- Run the formsculptor doctor and bench
- Why does the command palette in frontend/src/ui miss my new tool

## Should not trigger

- Add a smooth brush to Blender (different app; Blender is study only)
- Fix a formvis viewport bug (sibling repo, own harness)
- How do I sculpt a dragon in ZBrush
- Write a generic Rust PyO3 tutorial unrelated to this repo
- Create a Tauri updater signing key for another project
- Explain what voxel remeshing is in general
- Build a skill pack for Terraform modules
- Implement an MCP server for a Slack integration
