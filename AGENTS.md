# formsculpt

## Changing formsculpt's code

Read `CLAUDE.md` first, then route through `.claude/skills/formsculpt-harness/SKILL.md`. Commands are declared only in `native/crates/fsc-commands`.

## Sculpting through formsculpt (natural language)

Use the `formsculpt` MCP server with the `formsculpt-modeling` skill (`.agents/skills/formsculpt-modeling`). Codex reads `.codex/config.toml` and Cursor `.cursor/mcp.json`; both run `./formsculpt mcp`, which starts the engine when it is not running. `./formsculpt agents install` registers formsculpt for Claude Code, Codex and Cursor outside this folder. The tools work with or without the formsculpt window open; with it open, the person is asked once before the first change.
