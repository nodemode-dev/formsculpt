"""Interface text for translation.

  extract   collect the English interface strings into frontend/src/i18n/strings.json
  check     report each language table's coverage of strings.json and any {placeholder} mismatch

English is the source and the key: every table frontend/src/i18n/<code>.json maps an English string to its
translation. Regional variants (es-MX, fr-CA) keep only the words that differ from their parent.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "frontend" / "src"
OUT = SRC / "i18n"
REGISTRY = ROOT / "native" / "crates" / "fsc-commands" / "src"
LANGS = ["es", "es-MX", "pt-BR", "fr", "fr-CA", "de", "it", "nl", "da", "fi", "nb", "sv", "ru", "pl", "cs", "hu", "uk", "tr",
         "zh-CN", "zh-TW", "ja", "ko", "id", "th", "vi"]
REGIONAL = {"es-MX", "fr-CA"}

CALLS = [
    re.compile(r"\bel\(\s*'[a-z0-9]+'\s*,\s*'[^']*'\s*,\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\bt\(\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\bfield\(\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\bchoices\(\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\bnote\(\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\[\s*'[a-z0-9_.-]+'\s*,\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\b(?:label|title|description)\s*:\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"'(aria-label|placeholder|title)'\s*,\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\b(?:aria-label|title|placeholder)=\"([^\"$<>{}]+)\""),
    re.compile(r">([A-Z][^<>{}$`]{1,200})<"),
    re.compile(r"\b[cn]\(\s*'[a-z0-9_.]+'\s*,\s*'((?:[^'\\]|\\.)+)'"),
    re.compile(r"\bdef\(\s*'((?:[^'\\]|\\.)+)'"),
]
RUST_LABEL = re.compile(r'^\s*label:\s*"([^"]+)"', re.M)


def looks_like_text(s: str) -> bool:
    if not re.search(r"[A-Za-z]", s) or len(s) > 300:
        return False
    if re.fullmatch(r"[a-z0-9_.:/#-]+", s):
        return False
    if s.startswith(("http", "/", "--", ".", "#")) or "=>" in s:
        return False
    return True


def extract() -> int:
    found: set[str] = set()
    for f in sorted(SRC.rglob("*.ts")):
        if "i18n" in f.parts:
            continue
        text = f.read_text(encoding="utf-8")
        for rx in CALLS:
            for m in rx.finditer(text):
                s = m.group(m.lastindex or 1).replace("\\'", "'")
                if looks_like_text(s):
                    found.add(s.strip())
    for f in sorted(REGISTRY.glob("*.rs")):
        for m in RUST_LABEL.finditer(f.read_text(encoding="utf-8")):
            found.add(m.group(1))
    found.discard("")
    OUT.mkdir(parents=True, exist_ok=True)
    strings = sorted(found, key=str.lower)
    (OUT / "strings.json").write_text(json.dumps(strings, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"[formsculpt] {len(strings)} interface strings in frontend/src/i18n/strings.json")
    return 0


def check() -> int:
    strings = json.loads((OUT / "strings.json").read_text(encoding="utf-8"))
    bad = 0
    for code in LANGS:
        path = OUT / f"{code}.json"
        table = json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {}
        have = sum(1 for s in strings if s in table)
        for en, tr in table.items():
            if sorted(re.findall(r"\{\w+\}", en)) != sorted(re.findall(r"\{\w+\}", tr)):
                print(f"  {code}: placeholders differ in {en!r}")
                bad += 1
        share = "regional overrides" if code in REGIONAL else f"{have}/{len(strings)} ({100 * have // max(1, len(strings))}%)"
        print(f"  {code:6} {share}")
    return 1 if bad else 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "check"
    sys.exit(extract() if cmd == "extract" else check() if cmd == "check" else 2)
