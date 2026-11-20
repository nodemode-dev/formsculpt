# setup HASHOUT_LOG

Living log for the launcher's packaging scripts. Entries are anchored by file and symbol, and removed code is quoted in full.

## formsculpt 1.24.0 · a Windows engine with no console window

### freeze_backend, ../formsculpt.bat :freeze · `--noconsole` on Windows unless FSC_ENGINE_WINDOWED=0
- refers-to: setup/freeze_backend :: pyinstaller; formsculpt.bat :: :freeze
- kind: rationale
- logged: v1.24.0 · 2026-11-20
The owner's decision for every app: a Windows engine is packaged with no console window. `freeze_backend` gives PyInstaller `--noconsole` when `fs_os` says windows and `FSC_ENGINE_WINDOWED` is not `0`; the release build runs this bash script in Git Bash, so this is the path that makes the shipped engine. Its twin, the `:freeze` section of `formsculpt.bat`, does the same through `FSC_CONSOLE` (empty when `FSC_ENGINE_WINDOWED=0`; `setlocal` keeps it inside the script). The engine's entry file replaces missing standard streams before anything else (backend log, 1.23.0), so a windowed engine and its `--selfcheck` still run. Checked here with `bash -n` and by reading the `.bat`; no Windows build has shown it.
