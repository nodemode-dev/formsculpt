---
name: agent-substrate
description: "Covers the formsculptor agent substrate: the MCP server (Streamable HTTP at /mcp), the shared PyO3 engine session, the window's change epoch and consent dialog, the stdio shim, and agent configs. Load when changing MCP behaviour, protocol versions, consent, live window refresh or agents install and sync."
---

# Agent Substrate

## Purpose

Own how agents drive formsculptor. Hands back an MCP change that behaves the same with a window open or closed.

## Preconditions

State: built at 0.3.0. The tool list always comes from the engine's catalogue (`fsc_engine.catalogue()`, the same JSON `fsc-catalogue` prints; see `sub-skills/command-registry/SUB-SKILL.md`).

## Procedure

1. Server: `backend/app/routers/mcp.py`, Streamable HTTP via POST `/mcp`, stateless JSON, Origin limited to this computer. Protocol versions 2026-07-28 (server/discover, cache hints) and 2025-11-25 (initialize). Methods: server/discover, initialize, ping, tools/list, tools/call, prompts/list, prompts/get (`sculpt_strategy`). Tool results carry the engine's JSON as text and as structuredContent; engine errors come back with isError and the code and message.
2. Stdio shim: `./formsculptor mcp` runs `setup/mcp_stdio.py`, which forwards each line to `FSC_MCP_URL` (default `http://127.0.0.1:8196/mcp`) and starts `./formsculptor serve` or `backend` once if nothing answers.
3. One engine: the window and MCP drive the same `fsc_engine.Engine` session in the backend. The window learns of outside changes through `GET /api/mcp/window?since=epoch` (polled about once a second; the epoch rises on every mutating dispatch and stroke) and refreshes the scene.
4. Consent: with a window open (it polled in the last 4 s) and no answer yet, the first mutating tool call waits up to 90 s while the window shows a dialog (`frontend/src/ui/agent-link.ts`); Allow lasts for the service's lifetime, Decline refuses calls for 600 s. With no window open, calls run without asking: the person started the agent. Read-only tools never ask. `POST /api/mcp/reset` clears the answer. The WASM engine, when it lands, keeps this contract: the service stays the authority agents talk to.
5. Every mutating command is one undo step on either path.
6. Agent configs and skills: `./formsculptor agents install` and `agents sync`. The operator skill `formsculptor-modeling` is generated; never hand-edit its tools.md.
7. Treat all tool arguments as untrusted input; validate against the schema before dispatch.

## Gate

Done when the same call succeeds over HTTP and stdio, with and without a window, and consent denial blocks window execution.

## Hands back

The change plus the transcript of calls made on each path.

## Owned resources

None bundled. Related: `sub-skills/engine-service/SUB-SKILL.md`.
