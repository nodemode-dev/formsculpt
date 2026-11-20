#!/usr/bin/env python3
import hashlib
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKILL = ROOT / ".claude" / "skills" / "formsculpt-modeling"
MIRROR = ROOT / ".agents" / "skills" / "formsculpt-modeling"
REFERENCE = SKILL / "references" / "tools.md"
LAUNCHER = "formsculpt"
LAUNCH = str(ROOT / LAUNCHER)
ARGS = ["mcp"]
MANIFEST = ROOT / "agents" / "manifest.json"
DOCK = {"present": True, "positions": ["bottom"], "tabs": ["chat", "terminal", "claude-code", "codex", "cursor", "antigravity"]}
DOCK_DIR = ROOT / "frontend" / "src" / "dock"
LOOKUP = ROOT / "backend" / "app" / "services" / "agent_lookup.py"
LOOKUP_HASHES = {"1.0.1": "62667f9b68b7d101e1896ce617240424e33b7e498c560bc79d45f0ab26c77d51"}
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


def skill_list() -> list[dict]:
    base = ROOT / ".claude" / "skills"
    found = []
    for skill in sorted(d for d in base.iterdir() if d.is_dir()):
        inner = skill / "sub-skills"
        subs = sorted(d.name for d in inner.iterdir() if (d / "SUB-SKILL.md").is_file()) if inner.is_dir() else []
        found.append({"name": skill.name, "sub_skills": subs})
    return found


def manifest_text(cat: dict) -> str:
    names = [c["name"] for c in cat["commands"]]
    read_only = [c["name"] for c in cat["commands"] if (c.get("annotations") or {}).get("readOnlyHint") is True]
    http = '@router.post("/mcp")' in (ROOT / "backend" / "app" / "routers" / "mcp.py").read_text(encoding="utf-8")
    transports = (["stdio"] if (ROOT / "setup" / "mcp_stdio.py").is_file() else []) + (["http"] if http else [])
    data = {
        "format": "form-agent-manifest",
        "version": 1,
        "app": LAUNCHER,
        "mcp": {"tools": names, "read_only": read_only, "approval_gated": [n for n in names if n not in read_only], "transports": transports},
        "skills": skill_list(),
        "dock": DOCK,
        "extensions": None,
    }
    return json.dumps(data, indent=2, ensure_ascii=False) + "\n"


def files_under(base: Path) -> dict:
    return {p.relative_to(base).as_posix(): p for p in sorted(base.rglob("*")) if p.is_file() and "__pycache__" not in p.parts and p.name != ".DS_Store"} if base.is_dir() else {}


def sync(check: bool) -> int:
    cat = catalogue()
    text = render(cat)
    manifest = manifest_text(cat)
    current = REFERENCE.read_text(encoding="utf-8") if REFERENCE.exists() else ""
    mirrored = (MIRROR / "references" / "tools.md").read_text(encoding="utf-8") if (MIRROR / "references" / "tools.md").exists() else ""
    if check:
        drift = current != text or mirrored != text
        stale = not MANIFEST.exists() or MANIFEST.read_text(encoding="utf-8") != manifest
        say("the modeling skill's tool reference has drifted; run ./formsculpt agents sync" if drift else "tool reference up to date")
        if stale:
            say("agents/manifest.json is not what the engine's tools and the skills folder say; run ./formsculpt agents sync")
        return 1 if drift or stale else 0
    REFERENCE.parent.mkdir(parents=True, exist_ok=True)
    REFERENCE.write_text(text, encoding="utf-8")
    if MIRROR.exists():
        shutil.rmtree(MIRROR)
    shutil.copytree(SKILL, MIRROR)
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(manifest, encoding="utf-8", newline="\n")
    say(f"wrote {REFERENCE.relative_to(ROOT)}, mirrored the skill to {MIRROR.relative_to(ROOT)} and wrote {MANIFEST.relative_to(ROOT)}")
    return 0


def check_server(rel: str, server, problems: list[str]) -> None:
    if not isinstance(server, dict):
        problems.append(f"{rel} has no {LAUNCHER} server")
        return
    command, args = server.get("command"), server.get("args") or []
    joined = " ".join(str(a) for a in args)
    if command == "sh":
        if LAUNCHER not in joined or not joined.rstrip().endswith("mcp"):
            problems.append(f"{rel} must run the {LAUNCHER} launcher with mcp")
    elif not isinstance(command, str) or os.path.basename(command) != LAUNCHER or args != ARGS:
        problems.append(f"{rel} must name the {LAUNCHER} launcher with the argument mcp")


def check() -> int:
    problems: list[str] = []
    if sync(True):
        problems.append("the tool reference, its mirror or agents/manifest.json is out of date")
    source, mirror = files_under(SKILL), files_under(MIRROR)
    for rel in sorted(set(source) | set(mirror)):
        if rel not in mirror or rel not in source or source[rel].read_bytes() != mirror[rel].read_bytes():
            problems.append(f".agents/skills/formsculpt-modeling/{rel} does not match .claude/skills; run ./formsculpt agents sync")
    for skill in (ROOT / ".claude" / "skills").iterdir():
        text = (skill / "SKILL.md").read_text(encoding="utf-8") if (skill / "SKILL.md").is_file() else ""
        name = re.search(r"(?m)^name:\s*(\S+)", text)
        if not name or name.group(1) != skill.name:
            problems.append(f".claude/skills/{skill.name}/SKILL.md has no name matching its folder")
        if not re.search(r"(?m)^description:\s*\S", text):
            problems.append(f".claude/skills/{skill.name}/SKILL.md has no description")
    try:
        check_server(".mcp.json", json.loads((ROOT / ".mcp.json").read_text(encoding="utf-8")).get("mcpServers", {}).get(LAUNCHER), problems)
        check_server(".cursor/mcp.json", json.loads((ROOT / ".cursor" / "mcp.json").read_text(encoding="utf-8")).get("mcpServers", {}).get(LAUNCHER), problems)
        check_server(".codex/config.toml", tomllib.loads((ROOT / ".codex" / "config.toml").read_text(encoding="utf-8")).get("mcp_servers", {}).get(LAUNCHER), problems)
    except (OSError, ValueError) as exc:
        problems.append(f"a project MCP config cannot be read: {exc}")
    try:
        kept = json.loads((DOCK_DIR / "MANIFEST.json").read_text(encoding="utf-8")).get("files", {})
    except (OSError, ValueError):
        kept = {}
        problems.append("frontend/src/dock/MANIFEST.json is missing or not JSON")
    now = {n: hashlib.sha256(p.read_bytes()).hexdigest() for n, p in files_under(DOCK_DIR).items() if n != "MANIFEST.json"}
    for name in sorted(set(kept) | set(now)):
        if kept.get(name) != now.get(name):
            problems.append(f"frontend/src/dock/{name} does not match MANIFEST.json (the shared dock module changes only as a whole new version, copied from FormVis)")
    raw = LOOKUP.read_bytes() if LOOKUP.is_file() else b""
    said = re.search(rb'^VERSION = "([^"\r\n]+)"\r?$', raw, re.M)
    version = said.group(1).decode("ascii", "replace") if said else "unknown"
    if LOOKUP_HASHES.get(version) != hashlib.sha256(raw).hexdigest():
        problems.append(f"backend/app/services/agent_lookup.py does not match the hash recorded for version {version}")
    for line in problems:
        print(f"[formsculpt] {line}", file=sys.stderr)
    if problems:
        print(f"[formsculpt] {len(problems)} problem(s)", file=sys.stderr)
        return 1
    say("clean: the mirror matches, the skills have names and descriptions, the project MCP configs name the launcher, agents/manifest.json is current, the dock module and the agent finder match their hashes")
    return 0


def lookup():
    spec = importlib.util.spec_from_file_location("formsculpt_agent_lookup", LOOKUP)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def ours(entry) -> bool:
    return isinstance(entry, dict) and entry.get("args") == ARGS and Path(str(entry.get("command", ""))).name == LAUNCHER


def write_json(cfg: Path, data: dict) -> None:
    cfg.parent.mkdir(parents=True, exist_ok=True)
    if cfg.exists():
        shutil.copy2(cfg, cfg.with_name(cfg.name + ".bak"))
    part = cfg.with_name(cfg.name + f".part-{os.getpid()}")
    part.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    os.replace(part, cfg)


def install_json(title: str, cfg: Path, dry: bool) -> None:
    try:
        data = json.loads(cfg.read_text(encoding="utf-8") or "{}") if cfg.exists() else {}
    except ValueError:
        data = None
    if not isinstance(data, dict) or not isinstance(data.get("mcpServers", {}), dict):
        say(f"{title}: {cfg} is not an MCP servers file formsculpt can read; left alone")
        return
    entry = {"command": LAUNCH, "args": ARGS}
    have = data.get("mcpServers", {}).get(LAUNCHER)
    if have == entry:
        say(f"{title}: {cfg} already has formsculpt")
    elif have is not None and not ours(have):
        say(f"{title}: {cfg} has a server named formsculpt that formsculpt did not write; left alone")
    else:
        say(f"{title}: " + ("would write" if dry else "writing") + f" formsculpt into {cfg}")
        if not dry:
            data.setdefault("mcpServers", {})[LAUNCHER] = entry
            write_json(cfg, data)


def install(dry: bool, targets: list[str]) -> int:
    command = LAUNCH
    targets = targets or ["claude", "codex", "cursor", "antigravity"]
    if "claude" in targets:
        finder = lookup()
        found = finder.check(finder.find("claude", override_vars=("FSC_CLAUDE_BIN",)))
        claude = found.path if found.state == "runs" else None
        if claude:
            env = finder.child_env(claude)
            have = finder.run([claude, "mcp", "get", "formsculpt"], env, 60)
            mine = have.returncode == 0 and "Scope: User" in have.stdout
            if mine and command in have.stdout:
                say("Claude Code: formsculpt is already registered (user scope)")
            elif mine:
                say("Claude Code: a server named formsculpt that formsculpt did not write is registered; left alone")
            else:
                args = [claude, "mcp", "add", "--scope", "user", "formsculpt", "--", command, "mcp"]
                say(("would run: " if dry else "running: ") + " ".join(args))
                if not dry:
                    finder.run(args, env, 60)
            skills = Path.home() / ".claude" / "skills" / "formsculpt-modeling"
            same = {k: v.read_bytes() for k, v in files_under(skills).items()} == {k: v.read_bytes() for k, v in files_under(SKILL).items()}
            say(f"the modeling skill is already at {skills}" if same else ("would copy" if dry else "copying") + f" the modeling skill to {skills}")
            if not dry and not same:
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
        install_json("Cursor", Path.home() / ".cursor" / "mcp.json", dry)
    if "antigravity" in targets:
        install_json("Antigravity", Path.home() / ".gemini" / "config" / "mcp_config.json", dry)
    return 0


def main(argv: list[str]) -> int:
    if not argv or argv[0] not in ("sync", "check", "install"):
        print("usage: ./formsculpt agents sync [--check] | check | install [--dry-run] [claude|codex|cursor|antigravity ...]")
        return 2
    if argv[0] == "check":
        return check()
    if argv[0] == "sync":
        return sync("--check" in argv[1:])
    rest = [a for a in argv[1:] if a != "--dry-run"]
    return install("--dry-run" in argv[1:], rest)


if __name__ == "__main__":
    os.chdir(ROOT)
    sys.exit(main(sys.argv[1:]))
