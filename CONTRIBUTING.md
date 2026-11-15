# Contributing to FormSculpt

Thank you for helping. FormSculpt is the Form family's 3D sculpting and modeling app:
brushes, remeshing, UVs, painting and a path renderer in one window, with no account,
running on your own computer, and open to agents over MCP. It gets better through real
use and real contributions.

## Where to start

- **Use it and report what goes wrong.** A clear bug report, with steps, the version
  and your platform, is one of the most useful things you can give.
- **Look at the open issues.** Comment on one before starting larger work, so nobody
  builds the same thing twice.
- **Translations** are welcome: `./formsculpt i18n check` reports each language
  table's coverage, and a new interface string lands in every table under
  `frontend/src/i18n/` in the same change (`./formsculpt i18n extract`, then `check`).

## Setting up

Clone the repository, run `./formsculpt_oc_install` once, then `./formsculpt` (on
Windows, `formsculpt_oc_install.bat` and `formsculpt.bat`). The first checks the
toolchain and installs what the engine service and the interface need; the second
starts the engine service on 127.0.0.1:8260 and the window at http://localhost:5260.
You need Rust 1.99.1 (pinned in `rust-toolchain.toml`), Node 20.19+ or 22.12+ and
Python 3.12 or newer. `./formsculpt doctor` checks the toolchain, the installs and the
ports, and `./formsculpt desktop dev` opens the desktop window. The README's "Run it
from source" section has the details.

The code lives in:

| Folder | What is there |
| --- | --- |
| `native/crates` | the engine (Rust): core, sculpt, topology, paint, file formats and the command registry `fsc-commands` |
| `frontend/src` | the window (TypeScript, Vite): `engine/` never touches the DOM, the interface lives in `ui/` |
| `backend/app` | the engine service (Python, FastAPI): the sculpt library, MCP, terminals and the Claude Code dock |
| `backend/tests`, `frontend/tests/window` | the service suite and the window smoke test |
| `src-tauri` | the desktop shell (Rust, Tauri 2) |
| `integrations` | the Blender Live Link add-on and the Unreal and Unity GoF receivers |
| `plugins` | the example Python plugins |
| `setup` | the scripts behind `./formsculpt` |
| `assets` | repo-level art (the README logo and the desktop icon source), not app runtime assets |
| `.claude/skills` | agent skills; the modeling skill is mirrored into `.agents/skills` |
| `.docs/wiki` | the wiki's source, published on push |

## Before you open a pull request

Run the same checks CI runs:

```sh
(cd frontend && npm ci && npm audit --audit-level=low && npm run build)
./formsculpt engine check
./formsculpt engine build
./formsculpt engine wasm
./formsculpt test
./formsculpt i18n check
python -m pytest frontend/tests/window -q -p no:cacheprovider
```

`./formsculpt test` runs the engine and the service suites; the window smoke test
needs `pytest` and `playwright` with Chromium installed, and a built interface. CI
also boots the engine service and probes `/api/health` and `/api/capabilities`, runs
`bash -n` over the shell scripts, and fails on an em dash in tracked Markdown or an
emoji in any tracked text.

- Keep each pull request to one change.
- Write commit subjects the way the history does, in Conventional Commits with the
  version in the summary: `type(scope): plain sentence (X.Y.Z)`, for example
  `feat(engine): ...`, `fix(viewport): ...`, `docs(readme): ...`. Leave the version
  alone; a maintainer bumps it when releasing. See
  [.docs/BRANCHING.md](.docs/BRANCHING.md).
- A command is declared only in `native/crates/fsc-commands`. After adding or changing
  one, run `./formsculpt engine build`, then `./formsculpt agents sync`, so the
  modeling skill's tool reference and its `.agents/skills` mirror follow.
- Icons are inline `currentColor` SVG from the app's own icon module, and `z-index`
  comes only from the layer tokens in `frontend/src/styles/tokens.css`.
- No emoji anywhere, no em dash in Markdown, and keep code comments to a minimum: the
  context goes in the side's `HASHOUT_LOG.md` (`frontend/`, `backend/`, `native/`).
- The three-star sparkle icon is not used anywhere in the app; never add one.
- GoF is FormSculpt's own hand-off to other apps. Never copy another product's
  hand-off code or file format into this tree, and keep `gof.json` readable by the
  Blender, Unreal and Unity receivers when its entries change.

## How changes land

Only the maintainers push to `main`. Everyone else forks the repository and opens a
pull request into `main`. A pull request merges when every CI check passes on an
up-to-date branch and a maintainer has approved it after its last push.

## Releases

A release is a version, and only the maintainers make one: the version bump is the last
commit on `main`, a maintainer cuts a `compile_vX.Y.Z` branch from it, and every build
is compiled and released from
[FormCompiler](https://github.com/nodemode-dev/formcompiler), which builds every
platform and publishes them together. This repository holds the source; packaging and
releases stay in FormCompiler. See [.docs/BRANCHING.md](.docs/BRANCHING.md).

## Licensing

FormSculpt is licensed under the Apache License 2.0 ([LICENSE](LICENSE), SPDX
`Apache-2.0`). By contributing, you agree that your contribution is licensed under the
same terms. Only add code you have the right to contribute, and name any third-party
code and its licence in the pull request.

## Patents and other people's IP

Some features are deliberately not built because they come too close to someone else's
patent, trademark, copyright or design. The maintainers hold that line with counsel;
contributors do not decide it, and nobody here redesigns a feature on their own reading
of a patent. This is engineering diligence, not legal advice.

### Rules for every contribution

- Patents: do not add a feature, or a dependency that provides one, that reproduces a method you know to be patented. If you are unsure, open an issue that describes what the feature does, not which patent it resembles. Patent numbers stay out of code, commits, comments, logs and changelogs.
- Trademarks: other vendors' product and feature names appear only to name a file format or compatibility ("opens <format> files", "for use with"). No command, menu, panel, tool, preset, template, theme or agent tool takes another product's feature name, and no text says "like X" or "the way X does it". Describe what the feature does. FormSculpt's features carry its own names (ZUI Layout, LightCube, Quad Remesh, Voxel Remesh, Armature Spheres, Vertex Paint, Pose Part).
- Copyright: never copy another product's help, tooltip, dialog or documentation text (link to it instead); never trace or import vendor icons; never bundle vendor templates, themes, presets, brushes, swatches, LUTs, materials, sample files, sound libraries, stock media or fonts. Test fixtures come from this repository's own scripts.
- Licences: every new dependency, binary, model, font, profile or data file gets a row in `NOTICE.md` in the same commit that adds it. GPL stays outside the distributed app, AGPL nowhere, LGPL dynamically linked or a separate process and replaceable, MPL files unedited. CC BY credited, CC BY-SA only unmodified, CC NC and ND never.

## Conduct

Everyone taking part follows the [code of conduct](CODE_OF_CONDUCT.md). Security
problems go through [SECURITY.md](SECURITY.md), never a public issue.
