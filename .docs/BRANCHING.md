# Branching and releases

- `main` is always releasable. Work happens on `feat/[topic]`, `fix/[topic]`, `docs/[topic]` or `chore/[topic]` and merges back.
- Commits follow Conventional Commits with the version in the summary, for example `feat(engine): dyntopo (0.4.0)`, authored as `nodemode-ai <mackin@nodemode.ai>`.
- Versions are `v*.*.*` with dots, bumped together in: `frontend/package.json`, `frontend/package-lock.json` (two fields), `frontend/src/version.ts`, `backend/app/__init__.py`, `native/Cargo.toml` (`[workspace.package] version`) and `native/Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`, the README version badge, and a new `CHANGELOG.md` section. The bump commit is the last commit of a release.
- Releases are built by formcompiler (`nodemode-ai/formcompiler`), not here. After the version bump lands on main, push a branch `compile_vX.Y.Z` cut from that commit. CI runs on it, and formcompiler builds macOS, Linux and Windows bundles and drafts `formsculpt-vX.Y.Z` with the updater feed on its own release page.
- The wiki is edited only in `.docs/wiki`; `wiki.yml` publishes it on push to `main`.
