# native HASHOUT_LOG

Living log for the Rust engine side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
native/
  Cargo.toml                          workspace, [workspace.package] version (a version site)
  crates/fsc-core/src/lib.rs          ENGINE_VERSION, engine_version
  crates/fsc-sculpt/src/lib.rs        shell
  crates/fsc-topo/src/lib.rs          shell
  crates/fsc-paint/src/lib.rs         shell
  crates/fsc-io/src/lib.rs            shell
  crates/fsc-commands/src/lib.rs      Command, Group, Hints, COMMANDS, find, dispatch, catalogue
  crates/fsc-commands/src/bin/catalogue.rs  prints the catalogue JSON
```

## Contents

- [crates/fsc-commands/src/lib.rs](#cratesfsc-commandssrclibrs)

## crates/fsc-commands/src/lib.rs

### crates/fsc-commands/src/lib.rs · COMMANDS
- refers-to: crates/fsc-commands/src/lib.rs :: Command, COMMANDS, dispatch, catalogue
- kind: rationale
- logged: v0.1.0 · 2026-10-11
The one registry. formvis kept tool names in Python, tool behaviour in a TypeScript if-chain and UI command ids in a third list; here a command is declared once with its dotted id, its MCP name, its group, label, description, default keys, MCP annotations, a schema function and a run function. `find` accepts either name, `dispatch` runs it, `catalogue` exports everything the UI, the MCP server and `agents sync` need. Schemas are hand-written `serde_json` values for now; the plan is to derive them from argument structs once the first real commands land.
