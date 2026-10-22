#!/usr/bin/env python3
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKILL = ROOT / ".claude" / "skills" / "formsculpt-modeling"
MIRROR = ROOT / ".agents" / "skills" / "formsculpt-modeling"
REFERENCE = SKILL / "references" / "tools.md"
GROUP_TITLES = {
    "system": "Engine",
    "scene": "Scene",
    "object": "Objects",
    "sculpt": "Sculpting",
    "mask": "Masking",
    "group": "Face groups",
    "topo": "Topology",
    "layer": "Layers",
    "paint": "Painting",
    "view": "Viewing",
    "file": "Files",
    "history": "History",
}


def say(text: str) -> None:
    print(f"[formsculpt] {text}")


def catalogue() -> dict:
    built = ROOT / "native" / "target" / "release" / "fsc-catalogue"
    if built.exists():
        out = subprocess.run([str(built)], check=True, capture_output=True, text=True).stdout
    else:
        out = subprocess.run(["cargo", "run", "-q", "--release", "-p", "fsc-commands", "--bin", "fsc-catalogue"], cwd=ROOT / "native", check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def describe(name: str, spec: dict, required: set) -> str:
    kind = spec.get("type", "value")
    if kind == "array" and isinstance(spec.get("items"), dict):
        kind = f"array of {spec['items'].get('type', 'value')}"
    parts = [f"- `{name}`: {kind}"]
    if "enum" in spec:
        parts.append(" (" + " | ".join(str(v) for v in spec["enum"]) + ")")
    if name in required:
        parts.append("; required")
    if "default" in spec:
        parts.append(f"; default {json.dumps(spec['default'])}")
    for lo, hi in (("minimum", "maximum"), ("exclusiveMinimum", "maximum")):
        if lo in spec or hi in spec:
            parts.append(f"; range {spec.get(lo, '-inf')} to {spec.get(hi, 'inf')}")
            break
    if spec.get("description"):
        parts.append(f"; {spec['description']}")
    if spec.get("type") == "object" and isinstance(spec.get("properties"), dict):
        parts.append("; fields " + ", ".join(spec["properties"].keys()))
    return "".join(parts)


def render(cat: dict) -> str:
    lines = [
        "# formsculpt tools",
        "",
        f"Generated from the engine's command registry (formsculpt {cat['version']}) by `./formsculpt agents sync`; do not edit by hand.",
        "",
        "## Contents",
        "",
    ]
    groups: dict[str, list[dict]] = {}
    for c in cat["commands"]:
        groups.setdefault(c["group"], []).append(c)
    for g, cmds in groups.items():
        lines.append(f"- {GROUP_TITLES.get(g, g.title())}: " + ", ".join(f"`{c['name']}`" for c in cmds))
    lines.append("")
    for c in cat["commands"]:
        lines += [f"## {c['name']}", "", c["description"], ""]
        schema = c["inputSchema"]
        props = schema.get("properties") or {}
        required = set(schema.get("required") or [])
        if props:
            lines += [describe(k, v, required) for k, v in props.items()]
        else:
            lines.append("- no parameters")
        hints = c["annotations"]
        notes = []
        if hints.get("readOnlyHint"):
            notes.append("read-only")
        if hints.get("destructiveHint"):
            notes.append("removes or replaces (undoable)")
        if hints.get("idempotentHint") and not hints.get("readOnlyHint"):
            notes.append("safe to repeat")
        if c.get("keys"):
            notes.append("in the window: " + ", ".join(c["keys"]))
        if notes:
            lines += ["", "Note: " + "; ".join(notes) + "."]
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def sync(check: bool) -> int:
    text = render(catalogue())
    current = REFERENCE.read_text(encoding="utf-8") if REFERENCE.exists() else ""
    mirrored = (MIRROR / "references" / "tools.md").read_text(encoding="utf-8") if (MIRROR / "references" / "tools.md").exists() else ""
    if check:
        drift = current != text or mirrored != text
        say("the modeling skill's tool reference has drifted; run ./formsculpt agents sync" if drift else "tool reference up to date")
        return 1 if drift else 0
    REFERENCE.parent.mkdir(parents=True, exist_ok=True)
    REFERENCE.write_text(text, encoding="utf-8")
    if MIRROR.exists():
        shutil.rmtree(MIRROR)
    shutil.copytree(SKILL, MIRROR)
    say(f"wrote {REFERENCE.relative_to(ROOT)} and mirrored the skill to {MIRROR.relative_to(ROOT)}")
    return 0


def install(dry: bool, targets: list[str]) -> int:
    command = str(ROOT / "formsculpt")
    targets = targets or ["claude", "codex", "cursor"]
    if "claude" in targets:
        if shutil.which("claude"):
            args = ["claude", "mcp", "add", "--scope", "user", "formsculpt", "--", command, "mcp"]
            say(("would run: " if dry else "running: ") + " ".join(args))
            if not dry:
                subprocess.run(args, check=False)
            skills = Path.home() / ".claude" / "skills" / "formsculpt-modeling"
            say(("would copy" if dry else "copying") + f" the modeling skill to {skills}")
            if not dry:
                if skills.exists():
                    shutil.rmtree(skills)
                shutil.copytree(SKILL, skills)
        else:
            say("Claude Code's claude command was not found; the project's .mcp.json still works inside this folder")
    if "codex" in targets:
        cfg = Path.home() / ".codex" / "config.toml"
        block = f'\n[mcp_servers.formsculpt]\ncommand = "{command}"\nargs = ["mcp"]\nstartup_timeout_sec = 90\ntool_timeout_sec = 600\n'
        existing = cfg.read_text(encoding="utf-8") if cfg.exists() else ""
        if "[mcp_servers.formsculpt]" in existing:
            say(f"{cfg} already has formsculpt")
        else:
            say(("would append" if dry else "appending") + f" formsculpt to {cfg}")
            if not dry:
                cfg.parent.mkdir(parents=True, exist_ok=True)
                if cfg.exists():
                    shutil.copy2(cfg, cfg.with_suffix(".toml.bak"))
                cfg.write_text(existing + block, encoding="utf-8")
    if "cursor" in targets:
        cfg = Path.home() / ".cursor" / "mcp.json"
        data = json.loads(cfg.read_text(encoding="utf-8")) if cfg.exists() else {}
        servers = data.setdefault("mcpServers", {})
        if servers.get("formsculpt", {}).get("command") == command:
            say(f"{cfg} already has formsculpt")
        else:
            servers["formsculpt"] = {"command": command, "args": ["mcp"]}
            say(("would write" if dry else "writing") + f" formsculpt into {cfg}")
            if not dry:
                cfg.parent.mkdir(parents=True, exist_ok=True)
                if cfg.exists():
                    shutil.copy2(cfg, cfg.with_suffix(".json.bak"))
                cfg.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    return 0


def main(argv: list[str]) -> int:
    if not argv or argv[0] not in ("sync", "install"):
        print("usage: ./formsculpt agents sync [--check] | install [--dry-run] [claude|codex|cursor ...]")
        return 2
    if argv[0] == "sync":
        return sync("--check" in argv[1:])
    rest = [a for a in argv[1:] if a != "--dry-run"]
    return install("--dry-run" in argv[1:], rest)


if __name__ == "__main__":
    os.chdir(ROOT)
    sys.exit(main(sys.argv[1:]))
