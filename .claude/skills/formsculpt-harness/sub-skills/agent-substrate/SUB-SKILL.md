---
name: agent-substrate
description: "Covers the formsculpt agent substrate: the MCP server (Streamable HTTP at /mcp), the shared PyO3 engine session, the window's change epoch and consent dialog, the stdio shim, and agent configs. Load when changing MCP behaviour, protocol versions, consent, live window refresh or agents install and sync."
---

# Agent Substrate

## Purpose

Own how agents drive formsculpt. Hands back an MCP change that behaves the same with a window open or closed.

## Preconditions

State: built at 0.3.0. The tool list always comes from the engine's catalogue (`fsc_engine.catalogue()`, the same JSON `fsc-catalogue` prints; see `sub-skills/command-registry/SUB-SKILL.md`). At 0.9.0, a result carrying `png_base64` (get_view) is returned as an MCP image content block plus the other fields as text and structuredContent, so agents see the model.

## Procedure

1. Server: `backend/app/routers/mcp.py`, Streamable HTTP via POST `/mcp`, stateless JSON, Origin limited to this computer. Protocol versions 2026-07-28 (server/discover, cache hints) and 2025-11-25 (initialize). Methods: server/discover, initialize, ping, tools/list, tools/call, prompts/list, prompts/get (`sculpt_strategy`). Tool results carry the engine's JSON as text and as structuredContent; engine errors come back with isError and the code and message.
2. Stdio shim: `./formsculpt mcp` runs `setup/mcp_stdio.py`, which forwards each line to `FSC_MCP_URL` (default `http://127.0.0.1:8196/mcp`) and starts `./formsculpt serve` or `backend` once if nothing answers.
3. One engine: the window and MCP drive the same `fsc_engine.Engine` session in the backend. The window learns of outside changes through `GET /api/mcp/window?since=epoch` (polled about once a second; the epoch rises on every mutating dispatch and stroke) and refreshes the scene.
4. Consent: with a window open (it polled in the last 4 s) and no answer yet, the first mutating tool call waits up to 90 s while the window shows a dialog (`frontend/src/ui/agent-link.ts`); Allow lasts for the service's lifetime, Decline refuses calls for 600 s. With no window open, a changing call is refused with a message that says to open formsculpt and allow agents there, unless the person already allowed agents earlier in this engine's life (the answer is held in memory only). Read-only tools never ask and work without a window; the in-app token is never refused. `POST /api/mcp/reset` clears the answer. The WASM engine, when it lands, keeps this contract: the service stays the authority agents talk to.
5. Every mutating command is one undo step on either path.
6. Agent configs and skills: `./formsculpt agents install` and `agents sync`. The operator skill `formsculpt-modeling` is generated; never hand-edit its tools.md.
7. Treat all tool arguments as untrusted input; validate against the schema before dispatch.

## The dock

The window's terminal dock (Mod+J) holds shells, a Claude Code tab and a Chat tab. Its commands live on the service's small bus (`backend/app/services/commands.py`, `POST /api/commands/{id}`), not in the Rust registry, because they must never reach MCP: `terminal.*`, `claudecode.*`, `chat.*`, `agents.prefs`. Bytes travel over `WS /api/terminal/{id}` (local origins only); chat and terminal events over `GET /api/events`. Claude Code and Chat get `--mcp-config` for this service's `/mcp` with the `x-formsculpt-inapp` header and a per-run token, so they skip the outside-agent prompt; a recent decline still refuses them. Children lose `CLAUDE*` variables except the sign-in ones. A new window-only command is an `@command` in a `backend/app/services/cmd_*.py` imported by `app.main`, and its module is a hidden import in `setup/freeze_backend`.

## Gate

Done when the same call succeeds over HTTP and stdio, with and without a window, and consent denial blocks window execution.

## Hands back

The change plus the transcript of calls made on each path.

## Owned resources

None bundled. Related: `sub-skills/engine-service/SUB-SKILL.md`.
