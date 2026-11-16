#!/usr/bin/env python3
"""Small dependency-free structural checker for MB5 example projects."""
from __future__ import annotations
import re
import sys
from pathlib import Path

CONTAINERS = {"card", "horizontal", "vertical", "grid", "scrollbox", "topbar", "gradient", "opacity"}
COMPONENTS = {
    "alert", "text", "list", "button", "card", "icon", "input", "checkbox", "switch",
    "slider", "progress", "divider", "scrollbox", "topbar", "navigation", "dropdown",
    "tab", "spacing", "horizontal", "vertical", "grid", "snackbar", "gradient", "opacity",
}
DECL_RE = re.compile(r"^([A-Za-z][A-Za-z0-9_-]*)\s*(?:\(|\{)")
ID_RE = re.compile(r'^\.id\(\s*["\']([^"\']+)["\']\s*\)')


def strip_line_comment(line: str) -> str:
    quote = None
    escaped = False
    i = 0
    while i < len(line):
        ch = line[i]
        if escaped:
            escaped = False
        elif ch == "\\" and quote:
            escaped = True
        elif quote:
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
        elif line.startswith("//", i):
            return line[:i]
        i += 1
    return line


def scan_balanced(text: str, path: Path, errors: list[str]) -> None:
    stack: list[tuple[str, int]] = []
    quote = None
    escaped = False
    line_no = 1
    for ch in text:
        if ch == "\n":
            line_no += 1
        if escaped:
            escaped = False
            continue
        if quote:
            if ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            continue
        if ch in "\"'":
            quote = ch
        elif ch in "({[":
            stack.append((ch, line_no))
        elif ch in ")}]":
            expected = {")": "(", "}": "{", "]": "["}[ch]
            if not stack or stack[-1][0] != expected:
                errors.append(f"{path}:{line_no}: unmatched closing {ch}")
            else:
                stack.pop()
    for opener, start_line in stack:
        errors.append(f"{path}:{start_line}: unclosed {opener}")
    if quote:
        errors.append(f"{path}: unterminated quoted string")


def lint_ui(path: Path, errors: list[str]) -> None:
    source = path.read_text(encoding="utf-8")
    scan_balanced(source, path, errors)
    board_depth: int | None = None
    brace_depth = 0
    topbar_count = 0
    ids: dict[str, int] = {}
    for number, raw in enumerate(source.splitlines(), 1):
        line = strip_line_comment(raw).strip()
        if not line:
            continue
        if line.startswith("board ") and "{" in line:
            board_depth = brace_depth + 1
            topbar_count = 0
        decl = DECL_RE.match(line)
        if decl:
            name = decl.group(1)
            if name in COMPONENTS:
                if name == "topbar":
                    if board_depth is None or brace_depth != board_depth:
                        errors.append(f"{path}:{number}: topbar must be declared directly inside a board")
                    topbar_count += 1
                    if topbar_count > 1:
                        errors.append(f"{path}:{number}: only one topbar is allowed in each board")
            elif line.startswith(("board ", "components ")):
                pass
            elif line.startswith("@"):
                pass
        match = ID_RE.match(line)
        if match:
            value = match.group(1)
            if value in ids:
                errors.append(f"{path}:{number}: duplicate component id {value!r} (first at line {ids[value]})")
            else:
                ids[value] = number
        brace_depth += line.count("{") - line.count("}")
        if brace_depth <= 0 and line.startswith("}"):
            board_depth = None


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: lint.py components_dir", file=sys.stderr)
        return 2
    root = Path(sys.argv[1]).resolve()
    ui_file = root / "app" / "main.mb5"
    script_file = root / "scripts" / "main.mb5"
    model_file = root / "models" / "main.yaml"
    errors: list[str] = []
    if not ui_file.is_file():
        errors.append(f"missing required file: {ui_file}")
    else:
        lint_ui(ui_file, errors)
    for required in (root / "config.ini", model_file):
        if not required.is_file():
            errors.append(f"missing required file: {required}")
    if script_file.exists():
        scan_balanced(script_file.read_text(encoding="utf-8"), script_file, errors)
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"MB5 lint passed: {root}")
    if not script_file.exists():
        print("NOTE: scripts/main.mb5 is absent; UI-only project")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
