---
name: agent-substrate
description: "Covers the formsculptor agent substrate: the MCP server (Streamable HTTP at /mcp), window bridge long-poll and consent dialog, headless execution through PyO3, the stdio shim, and agent configs. Load when changing MCP behaviour, protocol versions, consent, the bridge or agents install and sync."
---

# Agent Substrate

## Purpose

Own how agents drive formsculptor. Hands back an MCP change that behaves the same with a window open or closed.

## Preconditions

State: v0.1.0 scaffold; the MCP server is planned until code exists. The tool list always comes from `catalogue.json` (see `sub-skills/command-registry/SUB-SKILL.md`).

## Procedure

1. Server: Python, in the backend mcp router, Streamable HTTP via POST `/mcp`. Supported protocol versions: 2026-07-28 and 2025-11-25.
2. Stdio shim: `./formsculptor mcp`. It autostarts the engine if none is running and forwards.
3. Window open: MCP calls travel over a long-poll bridge to the window's WASM engine, after a one-time consent dialog in the UI. No consent, no execution.
4. No window: the backend runs the identical Rust `dispatch` headless through the `fsc-py` PyO3 module. Both paths must give identical results for identical JSON.
5. Every mutating command is one undo step on either path.
6. Agent configs and skills: `./formsculptor agents install` and `agents sync`. The operator skill `formsculptor-modeling` is generated; never hand-edit its tools.md.
7. Treat all tool arguments as untrusted input; validate against the schema before dispatch.

## Gate

Done when the same call succeeds over HTTP and stdio, with and without a window, and consent denial blocks window execution.

## Hands back

The change plus the transcript of calls made on each path.

## Owned resources

None bundled. Related: `sub-skills/engine-service/SUB-SKILL.md`.
