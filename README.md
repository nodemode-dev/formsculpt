<p align="center">
  <img src="assets/app-icon/formsculpt-256.png" width="128" height="128" alt="FormSculpt icon: a planar bust in the Form family colours">
</p>

<h1 align="center">FormSculpt</h1>

<p align="center">
  <b>3D sculpting; a Rust engine and a studio viewport for the Form family.</b><br>
  Brushes, quad remesh, UVs, painting and a path renderer,<br>
  on your own machine with no account. Agents sculpt over MCP.
</p>

<p align="center">
  <img alt="version 1.19.6" src="https://img.shields.io/badge/version-1.19.6-eef0f4?style=flat-square">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.99.1-f97316?style=flat-square&logo=rust&logoColor=white">
  <img alt="TypeScript" src="https://img.shields.io/badge/typescript-vite-3178c6?style=flat-square">
  <img alt="macOS, Windows, Linux" src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-native-2f7bf5?style=flat-square">
  <img alt="License: Apache 2.0" src="https://img.shields.io/badge/license-Apache--2.0-3a3a3a?style=flat-square">
  <img alt="Status: pre-release" src="https://img.shields.io/badge/status-pre--release-d69e2e?style=flat-square">
</p>

<p align="center">
  <a href="https://formsculpt.formai.studio/"><b>Download FormSculpt on formai.studio</b></a> ·
  <a href="#run-it-from-source">Run it from source</a> ·
  <a href="#the-form-family">All Form apps</a>
</p>

<p align="center">
  <img src="docs/images/formsculpt-hero.png" alt="FormSculpt with the female head bust from the app icon shown as white plaster with visible Clay strips, Crease and Draw tool marks in the hair, cheek and plinth, the tool grid on the right and the Objects panel on the left" width="100%">
  <br>
  <sub>The head bust the FormSculpt icon was made from, imported as FBX, shown with a white plaster matcap and worked over with Clay strips, Crease and Draw strokes.</sub>
</p>

<p align="center">
  <a href="#features">Features</a> ·
  <a href="#everything-in-the-box">In the box</a> ·
  <a href="#built-for-agents">Agents</a> ·
  <a href="#under-the-hood">Under the hood</a> ·
  <a href="#download">Download</a> ·
  <a href="#run-it-from-source">Run from source</a> ·
  <a href="#the-form-family">Form family</a>
</p>

<table>
  <tr>
    <td width="25%" valign="top">
      <h3>Rust engine</h3>
      One Cargo workspace does the sculpting, remeshing and baking, and compiles to WebAssembly so the window works with no service.
    </td>
    <td width="25%" valign="top">
      <h3>Full pipeline</h3>
      Sculpt, retopologize, unwrap UVs, paint, bake maps and path-render without leaving the window.
    </td>
    <td width="25%" valign="top">
      <h3>Local and private</h3>
      No account and no upload. The servers bind to 127.0.0.1 and your scenes stay on your machine.
    </td>
    <td width="25%" valign="top">
      <h3>Agent-ready</h3>
      The UI, the keyboard and an MCP server call the same 184 commands from one registry.
    </td>
  </tr>
</table>

## Features

<table>
  <tr>
    <td width="50%" valign="top">
      <img src="docs/images/formsculpt-wireframe.png" alt="The marble bust shown with its triangle wireframe over the shaded surface" width="100%">
      <br><sub>The same bust with the wireframe view on.</sub>
      <h3>Dense meshes, live</h3>
      Strokes run in a WebGL2 viewport, with dynamic topology, multires levels and voxel remesh underneath.
    </td>
    <td width="50%" valign="top">
      <img src="docs/images/formsculpt-paint.png" alt="Red paint brushed across the bust's cheek with the Paint tool active and its colour and roughness options below" width="100%">
      <br><sub>Vertex paint with colour, roughness and metalness.</sub>
      <h3>Paint on the model</h3>
      Paint layers, a Spotlight projection tool and texture painting sit beside the sculpt brushes.
    </td>
  </tr>
</table>

## Everything in the box

<table>
  <tr>
    <td width="33%" valign="top">
      <h4>Sculpt and topology</h4>
      Clay · Draw · Grab · Smooth · Crease · Snake hook · Dyntopo · Multires · Voxel remesh · Quad remesh · Decimate · Booleans
    </td>
    <td width="33%" valign="top">
      <h4>Surface and render</h4>
      UV unwrap · UDIM tiles · Map baking · Vertex paint · Paint layers · Spotlight · Path render · Lights and HDRIs
    </td>
    <td width="33%" valign="top">
      <h4>Formats</h4>
      Opens: <code>.fsc</code> · OBJ · STL · PLY · glTF/GLB · 3MF · FBX · USD · Saves: OBJ · STL · PLY · GLB · 3MF · Native: <code>.fsc</code>
    </td>
  </tr>
</table>

## Built for agents

184 commands live in one registry in `native/crates/fsc-commands`; the window, the keyboard and the MCP server all call them.

```sh
./formsculpt mcp                 # MCP server on stdio for Claude Code, Codex and Cursor
./formsculpt agents install      # register FormSculpt with Claude Code, Codex and Cursor
```

`.mcp.json`, `.codex/config.toml` and `.cursor/mcp.json` are already wired in this repo. See [`AGENTS.md`](AGENTS.md).

## Under the hood

- **Engine:** Rust crates for core, sculpt, topology, paint, I/O and the command registry (`native/`).
- **Window:** vanilla TypeScript and Vite with a WebGL2 viewport (`frontend/`), no UI framework.
- **Service:** a FastAPI engine service on 127.0.0.1:8260 (`backend/`), optional when the WebAssembly engine is built.
- **Tested:** engine suites, a service suite and a browser smoke test, run by `./formsculpt test`.

## Download

**Get the app:** download FormSculpt from [formai.studio](https://formsculpt.formai.studio/). **Keep it updated:** [FormPortal](https://github.com/nodemode-dev/formportal) installs, updates and rolls back every Form app from the same signed catalogue.

| Platform | Builds |
|---|---|
| macOS | Apple silicon |
| Windows | x64 |
| Linux | x64 |

All FormSculpt builds are compiled and released from [`nodemode-dev/formcompiler`](https://github.com/nodemode-dev/formcompiler/releases), not from this repo. This repo holds the source. No FormSculpt release has been published there yet.

## Run it from source

Prefer it in your browser, or want to contribute? Run the web UI straight from this repo.

**Requires:** Rust 1.99.1 (pinned in `rust-toolchain.toml`), Node 20.19+ or 22.12+, Python 3.12+.

```sh
git clone git@github.com:nodemode-dev/formsculpt.git
cd formsculpt
./formsculpt_oc_install     # checks the toolchain, installs the backend and frontend
./formsculpt                # backend on 8260, web UI at http://localhost:5260
```

To run with no service, build the browser engine with `./formsculpt engine wasm`, start `./formsculpt frontend`, and open `http://localhost:5260/?engine=wasm`. On Windows use `formsculpt_oc_install.bat` and `formsculpt.bat`. `./formsculpt desktop dev` opens the desktop window instead.

> [!IMPORTANT]
> **Status:** pre-release. CI builds the macOS and Windows bundles, but only the Linux build has been run. See [`CHANGELOG.md`](CHANGELOG.md).

**Contributing:** branch from `main`, keep changes to source here, and leave packaging and releases to formcompiler. See [`.docs/BRANCHING.md`](.docs/BRANCHING.md) and [`AGENTS.md`](AGENTS.md).

## Repository

| Path | What it is |
|---|---|
| `native/` | Rust engine workspace and the command registry |
| `frontend/` | Vite app: shell, engine client, WebGL2 viewport |
| `backend/` · `src-tauri/` | FastAPI engine service · Tauri 2 desktop shell |

Docs: [`.docs/wiki`](.docs/wiki) · Agents: [`CLAUDE.md`](CLAUDE.md), [`AGENTS.md`](AGENTS.md)

## The Form family

FormSculpt is one of the **Form apps**: native creative tools that install, update and roll back through FormPortal and can all be driven by agents.

| | App | What it's for | Download | Code |
|:-:|---|---|---|---|
| <img src=".github/assets/family/form.md.png" alt="" width="32" height="32"> | **form.md** | Documents, databases and realtime editing together | [formai.studio](https://formmd.formai.studio/) | [GitHub](https://github.com/nodemode-dev/form.md) |
| <img src=".github/assets/family/format.png" alt="" width="32" height="32"> | **FormAT** | Chat and agent workspace on your key | [formai.studio](https://format.formai.studio/) | [GitHub](https://github.com/nodemode-dev/format) |
| <img src=".github/assets/family/formCAD.png" alt="" width="32" height="32"> | **FormCAD** | Drafting, parametric parts and NURBS surfaces | | [GitHub](https://github.com/nodemode-dev/formCAD) |
| <img src=".github/assets/family/formcel.png" alt="" width="32" height="32"> | **FormCel** | Spreadsheets agents can read and edit | [formai.studio](https://formcel.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formcel) |
| <img src=".github/assets/family/formcraft.png" alt="" width="32" height="32"> | **FormCraft** | Generate images, video, audio, 3D and worlds | | [GitHub](https://github.com/nodemode-dev/formcraft) |
| <img src=".github/assets/family/formcut.png" alt="" width="32" height="32"> | **FormCut** | Multi-track video editing with no upload | [formai.studio](https://formcut.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formcut) |
| <img src=".github/assets/family/formDAM.png" alt="" width="32" height="32"> | **FormDAM** | Photo library and asset manager | [formai.studio](https://formdam.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formDAM) |
| <img src=".github/assets/family/formdeck.png" alt="" width="32" height="32"> | **FormDeck** | Presentations and pitch decks | [formai.studio](https://formdeck.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formdeck) |
| <img src=".github/assets/family/formdesign.png" alt="" width="32" height="32"> | **FormDesign** | Page layout for print and the web | [formai.studio](https://formdesign.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formdesign) |
| <img src=".github/assets/family/formencoder.png" alt="" width="32" height="32"> | **FormEncoder** | Media encoding queue with presets | [formai.studio](https://formencoder.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formencoder) |
| <img src=".github/assets/family/formfabric.png" alt="" width="32" height="32"> | **FormFabric** | Sewing patterns draped with cloth simulation | | [GitHub](https://github.com/nodemode-dev/formfabric) |
| <img src=".github/assets/family/formlit.png" alt="" width="32" height="32"> | **FormLit** | Photo library and raw development | [formai.studio](https://formlit.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formlit) |
| <img src=".github/assets/family/formmorpheus.png" alt="" width="32" height="32"> | **FormMorpheus** | Frame-by-frame and performed animation | | [GitHub](https://github.com/nodemode-dev/formmorpheus) |
| <img src=".github/assets/family/formNET.png" alt="" width="32" height="32"> | **FormNET** | See what your computer sends online | | [GitHub](https://github.com/nodemode-dev/formNET) |
| <img src=".github/assets/family/formpath.png" alt="" width="32" height="32"> | **FormPath** | Vector illustration with artboards and type | [formai.studio](https://formpath.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formpath) |
| <img src=".github/assets/family/formpdf.png" alt="" width="32" height="32"> | **FormPDF** | Edit, redact, sign and fill PDFs | [formai.studio](https://formpdf.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formpdf) |
| <img src=".github/assets/family/formpromo.png" alt="" width="32" height="32"> | **FormPromo** | Promo films from your own media | [formai.studio](https://formpromo.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formpromo) |
| <img src=".github/assets/family/formref.png" alt="" width="32" height="32"> | **FormRef** | Reference boards for creative work | [formai.studio](https://formref.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formref) |
| <img src=".github/assets/family/formrig.png" alt="" width="32" height="32"> | **FormRig** | Rig, skin and retarget any mesh | | [GitHub](https://github.com/nodemode-dev/formrig) |
| <img src=".github/assets/family/formsculpt.png" alt="" width="32" height="32"> | **FormSculpt** | 3D sculpting, remeshing and painting | [formai.studio](https://formsculpt.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formsculpt) |
| <img src=".github/assets/family/formsurface.png" alt="" width="32" height="32"> | **FormSurface** | Texture painting with PBR export | [formai.studio](https://formsurface.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formsurface) |
| <img src=".github/assets/family/formtools.png" alt="" width="32" height="32"> | **FormTools** | Viewers, batch tools and file utilities | | [GitHub](https://github.com/nodemode-dev/formtools) |
| <img src=".github/assets/family/formtxt.png" alt="" width="32" height="32"> | **FormTxt** | Word processing with .docx round trips | [formai.studio](https://formtxt.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formtxt) |
| <img src=".github/assets/family/formvfx.png" alt="" width="32" height="32"> | **FormVFX** | Motion graphics and VFX compositing | [formai.studio](https://formvfx.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formvfx) |
| <img src=".github/assets/family/formvis.png" alt="" width="32" height="32"> | **FormVis** | Layered image editing with local AI | [formai.studio](https://formvis.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formvis) |
| <img src=".github/assets/family/formwav.png" alt="" width="32" height="32"> | **FormWav** | Multitrack audio editing and mixing | [formai.studio](https://formwav.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formwav) |
| <img src=".github/assets/family/formweb.png" alt="" width="32" height="32"> | **FormWeb** | Visual website building with a CMS | [formai.studio](https://formweb.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formweb) |
| <img src=".github/assets/family/formportal.png" alt="" width="32" height="32"> | **FormPortal** | Install, update and roll back Form apps | [formai.studio](https://formportal.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formportal) |
| <img src=".github/assets/family/formtray.png" alt="" width="32" height="32"> | **FormTray** | One dock icon for every app | [formai.studio](https://formtray.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formtray) |
| <img src=".github/assets/family/formMCP.png" alt="" width="32" height="32"> | **FormMCP** | One MCP connection to the family | [formai.studio](https://formmcp.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formMCP) |
| <img src=".github/assets/family/formskills.png" alt="" width="32" height="32"> | **FormSkills** | Every app's agent skills in one place | [formai.studio](https://formskill.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formskills) |
| <img src=".github/assets/family/formbench.png" alt="" width="32" height="32"> | **FormBench** | Parity, testing and rankings for the family | [formai.studio](https://formranker.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formbench) |
| <img src=".github/assets/family/formslot.png" alt="" width="32" height="32"> | **FormSlot** | Codebase memory for coding agents | | [GitHub](https://github.com/nodemode-dev/formslot) |
| <img src=".github/assets/family/subform.png" alt="" width="32" height="32"> | **SubForm** | Workbench for parallel Claude Code agents | | [GitHub](https://github.com/nodemode-dev/subform) |
| <img src=".github/assets/family/formcompiler.png" alt="" width="32" height="32"> | **FormCompiler** | Builds, signs and releases every app | [formai.studio](https://formcompiler.formai.studio/) | [GitHub](https://github.com/nodemode-dev/formcompiler) |
| <img src=".github/assets/family/formpipe.png" alt="" width="32" height="32"> | **FormPipe** | Release pipeline and update feeds | | [GitHub](https://github.com/nodemode-dev/formpipe) |

## Maintainers

| Maintainer | Role |
|---|---|
| [@nodemode-ai](https://github.com/nodemode-ai) | Owner |
| [@gbarnai](https://github.com/gbarnai) | Co-owner |

Code owners are set in [`.github/CODEOWNERS`](.github/CODEOWNERS).

## License

FormSculpt is licensed under the Apache License 2.0 ([`LICENSE`](LICENSE), SPDX `Apache-2.0`). Copyright 2026 NodeAgentc, Inc. Third-party notices: [`NOTICE.md`](NOTICE.md).

<sub>Screenshot content: a CC0 model from Poly Haven. ZBrush is a trademark of Maxon Computer GmbH, and Nomad Sculpt is a trademark of its owner. FormSculpt is an independent project and is not affiliated with or endorsed by them.</sub>
