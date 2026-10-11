# frontend HASHOUT_LOG

Living log for the frontend side, _v0.1.0. Read the map and the TOC first; entries are anchored by file and symbol, and removed code is quoted in full.

## Org-navi-map

```text
frontend/
  index.html              critical style, pre-paint theme boot, #app and #overlay-root
  vite.config.ts          port 5196, host false, strictPort, isolation headers, /api proxy
  package.json            scripts dev, build, preview, typecheck, tauri
  public/favicon.svg      tab icon
  public/splash.html      the desktop splash window's page
  src/main.ts             stylesheet imports, start()
  src/app.ts              builds the shell once, polls the service, patches text in place
  src/version.ts          VERSION, a version site
  src/engine/service.ts   the only HTTP client so far (fetchVersion)
  src/ui/dom.ts           el, setText, setAttr (write only on change)
  src/ui/icons.ts         inline currentColor SVG icons
  src/styles/tokens.css   colour, spacing, size tokens and the stacking scale
  src/styles/fonts.css    fontsource imports
  src/styles/shell.css    grid shell, bars, rail, stage, panel, status
```

## Contents

- [index.html](#indexhtml)
- [vite.config.ts](#viteconfigts)
- [src/app.ts](#srcappts)
- [src/styles/tokens.css](#srcstylestokenscss)

## index.html

### index.html · theme boot script
- refers-to: index.html :: inline script in head
- kind: rationale
- logged: v0.1.0 · 2026-10-11
Runs before the body exists, so the saved accent and the light profile are on the root element before first paint; no theme flash on hard reload. It reads the shared `nodemode_ui_settings` key so a NodeMode host and formsculptor agree. `#app` stays hidden until `body.app-ready`, which `start()` adds after the shell is built in one pass.

## vite.config.ts

### vite.config.ts · ISOLATION headers
- refers-to: vite.config.ts :: ISOLATION
- kind: rationale
- logged: v0.1.0 · 2026-10-11
Cross-origin isolation (COOP same-origin, COEP require-corp) is set from day one because the WASM engine planned for 0.2.0 runs threads over SharedArrayBuffer, which browsers only allow on isolated pages. The engine service sends the same headers (backend `CrossOriginIsolation`).

## src/app.ts

### src/app.ts · buildShell and poll
- refers-to: src/app.ts :: buildShell, poll
- kind: rationale
- logged: v0.1.0 · 2026-10-11
The shell is built once. `poll` only writes the status dot's `data-state` and two text nodes through `setAttr` and `setText`, which skip unchanged values, so a 5 second poll never rebuilds or reflows. The status text has a reserved `min-width` (22ch) and the version cell 9ch with tabular numerals, so the change from "connecting" to "service connected" moves nothing.

## src/styles/tokens.css

### src/styles/tokens.css · stacking scale
- refers-to: src/styles/tokens.css :: --z-* tokens
- kind: explainer
- logged: v0.1.0 · 2026-10-11
Every z-index is a `var(--z-*)`. The bands mirror formvis and NodeMode so an embedded formsculptor never collides with its host.

| Token | Value | Use |
| --- | --- | --- |
| --z-base | 0 | stage |
| --z-raised | 8 | hovered items |
| --z-selected | 9 | selected items |
| --z-rail | 10 | tool rail |
| --z-chrome | 100 | app bar, status bar |
| --z-menu | 200 | menus anchored to chrome |
| --z-temp | 500 | drag artifacts |
| --z-companion | 800 | floating panels |
| --z-taskbar | 900 | floating taskbar |
| --z-modal | 1000 | modals |
| --z-settings | 1500 | settings popout |
| --z-context | 2000 | context menus, dialogs |
| --z-portal | 4000 | extension panels |
| --z-overlay | 50000 | full-screen editors |
| --z-overlay-modal | 60000 | modals over an overlay |
| --z-debug | 100000 | development only |

Anything that must escape its container renders into `#overlay-root`, a direct child of `body`.
