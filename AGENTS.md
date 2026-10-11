# formsculptor

## Changing formsculptor's code

Read `CLAUDE.md` first, then route through `.claude/skills/formsculptor-harness/SKILL.md`. Commands are declared only in `native/crates/fsc-commands`.

## Sculpting through formsculptor (natural language)

Use the `formsculptor` MCP server with the `formsculptor-modeling` skill (`.agents/skills/formsculptor-modeling`). Codex reads `.codex/config.toml` and Cursor `.cursor/mcp.json`; both run `./formsculptor mcp`, which starts the engine when it is not running. `./formsculptor agents install` registers formsculptor for Claude Code, Codex and Cursor outside this folder. The tools work with or without the formsculptor window open; with it open, the person is asked once before the first change.
