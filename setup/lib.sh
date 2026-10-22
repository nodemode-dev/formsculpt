#!/usr/bin/env bash
FSC_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export FSC_ROOT
fs_say() { printf '[formsculpt] %s\n' "$*"; }
fs_die() { printf '[formsculpt] error: %s\n' "$*" >&2; exit 1; }
fs_os() {
  case "$(uname -s 2>/dev/null)" in
    Darwin) echo macos ;;
    Linux) echo linux ;;
    MINGW*|MSYS*|CYGWIN*) echo windows ;;
    *) echo unknown ;;
  esac
}
fs_python() {
  local c
  for c in python3.14 python3.13 python3.12 python3; do
    if command -v "$c" >/dev/null 2>&1 && "$c" -c 'import sys; raise SystemExit(0 if sys.version_info >= (3, 12) else 1)' 2>/dev/null; then
      echo "$c"
      return 0
    fi
  done
  return 1
}
fs_venv() {
  local act="$FSC_ROOT/backend/venv/bin/activate"
  # Windows venvs (Git Bash on the CI runner) keep their activate script under Scripts.
  [ -f "$act" ] || act="$FSC_ROOT/backend/venv/Scripts/activate"
  [ -f "$act" ] || fs_die "backend venv missing, run ./formsculpt install"
  # shellcheck disable=SC1090
  source "$act"
}
fs_target_triple() {
  if [ -n "${FSC_TARGET:-}" ]; then echo "$FSC_TARGET"; return 0; fi
  command -v rustc >/dev/null 2>&1 || return 1
  rustc -vV | awk '/^host:/ {print $2}'
}
fs_exe() { [ "$(fs_os)" = windows ] && echo ".exe" || echo ""; }
fs_rust_env() {
  [ -d "$HOME/.cargo/bin" ] && PATH="$HOME/.cargo/bin:$PATH"
  export PATH
  unset RUSTC RUSTC_WORKSPACE_WRAPPER RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_RUSTFLAGS CARGO_BUILD_TARGET CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR CARGO_BUILD_RUSTC
  export RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WRAPPER=""
}
fs_rust_fresh() {
  local target="$FSC_ROOT/src-tauri/target" now
  now="$(rustc -vV 2>/dev/null | tr '\n' ' ')"
  if [ -d "$target/release" ] && [ "$(cat "$target/.formsculpt-rustc" 2>/dev/null)" != "$now" ]; then
    fs_say "the Rust compiler differs from the one that built src-tauri/target; cleaning it"
    ( cd "$FSC_ROOT/src-tauri" && cargo clean )
  fi
  mkdir -p "$target"
  printf '%s' "$now" > "$target/.formsculpt-rustc"
}
