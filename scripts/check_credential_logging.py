#!/usr/bin/env python3
"""
Credential-logging guard (CI check, T-0443 / OPS-03).

Scans Rust sources for log and print macro invocations that reference
sensitive identifiers (database URLs, passwords, connection strings)
without routing them through `mask_db_url()`.

This is a defensive practice: raw `info!("url: {}", database_url)` style
calls leak credentials in logs. Any such call must either mask the value
(`mask_db_url(&url)`) or bypass this guard with an explicit
`// allow(credential-logging): <reason>` comment on the preceding line.

Exit 0 on clean, 1 if any violations are found.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

MACROS = (
    "info", "debug", "trace", "warn", "error",
    "eprintln", "println", "eprint", "print",
)

# Sensitive identifier names (whole-word). Keep the list tight — false
# positives are noisy and push developers toward disabling the check.
SENSITIVE = (
    "database_url",
    "db_url",
    "connection_string",
    "password",
)

MACRO_RE = re.compile(
    r"\b(" + "|".join(MACROS) + r")!\s*\(",
)
SENSITIVE_RE = re.compile(r"\b(" + "|".join(SENSITIVE) + r")\b")
ALLOW_RE = re.compile(r"//\s*allow\(credential-logging\)")

# Files that define or test the guard itself, or that intentionally
# reference these identifiers in non-logging contexts the regex cannot
# distinguish. Matches against a path relative to the repo root.
EXCLUDE_PATHS = (
    "scripts/check_credential_logging.py",
    "tests/check_credential_logging_test.py",
)


def repo_root() -> Path:
    out = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(out.stdout.strip())


def list_rust_files(root: Path) -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "*.rs"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    files = []
    for line in out.stdout.splitlines():
        if not line:
            continue
        if line in EXCLUDE_PATHS:
            continue
        files.append(root / line)
    return files


def find_macro_invocation_end(text: str, open_paren_idx: int) -> int:
    """Return index just past the closing `)` of a macro invocation
    whose opening `(` is at `open_paren_idx`. Handles nested parens,
    string literals (including raw strings), and char literals.
    """
    i = open_paren_idx
    depth = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == '"':
            # Skip string literal, honoring escapes.
            i += 1
            while i < n:
                if text[i] == "\\":
                    i += 2
                    continue
                if text[i] == '"':
                    i += 1
                    break
                i += 1
            continue
        if c == "'":
            # Char literal or lifetime — skip a single char/escape if it
            # looks like a char literal, otherwise just advance.
            if i + 1 < n and text[i + 1] == "\\":
                # escape seq; find closing '
                j = i + 2
                while j < n and text[j] != "'":
                    j += 1
                i = j + 1
                continue
            if i + 2 < n and text[i + 2] == "'":
                i += 3
                continue
            i += 1
            continue
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def line_of(text: str, idx: int) -> int:
    return text.count("\n", 0, idx) + 1


def preceding_line(text: str, idx: int) -> str:
    # The line containing `idx` is the macro-call line. The "preceding"
    # line is the one before it.
    line_start = text.rfind("\n", 0, idx) + 1
    prev_end = line_start - 1
    if prev_end <= 0:
        return ""
    prev_start = text.rfind("\n", 0, prev_end) + 1
    return text[prev_start:prev_end]


def scan_file(path: Path, root: Path) -> list[tuple[Path, int, str, str]]:
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return []

    violations: list[tuple[Path, int, str, str]] = []
    for m in MACRO_RE.finditer(text):
        open_paren = m.end() - 1
        end = find_macro_invocation_end(text, open_paren)
        body = text[open_paren + 1 : end - 1]

        # String literals (including the format string) are user-facing
        # text — `"database_url: {}"` is not a leak on its own. Strip
        # them before searching for sensitive identifiers so we only
        # flag actual Rust bindings being interpolated.
        stripped = re.sub(r'"(?:\\.|[^"\\])*"', "", body)
        # Raw string literals: r"..." and r#"..."# — strip a few hash depths.
        stripped = re.sub(r'r#*"[^"]*"#*', "", stripped)
        # Sub-expressions already routed through a `mask_*(...)` helper
        # (mask_db_url, mask_password, …) are safe by construction.
        stripped = re.sub(r"\bmask_\w+\s*\([^()]*\)", "", stripped)

        hit = SENSITIVE_RE.search(stripped)
        if not hit:
            continue

        # Respect explicit allow comment on the preceding line.
        if ALLOW_RE.search(preceding_line(text, m.start())):
            continue

        rel = path.relative_to(root)
        line = line_of(text, m.start())
        macro = m.group(1)
        violations.append((rel, line, macro, hit.group(1)))
    return violations


def main() -> int:
    root = repo_root()
    files = list_rust_files(root)
    all_violations: list[tuple[Path, int, str, str]] = []
    for f in files:
        all_violations.extend(scan_file(f, root))

    if not all_violations:
        print(
            f"credential-logging guard: clean ({len(files)} Rust files scanned)"
        )
        return 0

    print("credential-logging guard: VIOLATIONS FOUND")
    print()
    print(
        "Log/print macros referenced sensitive identifiers "
        "without mask_db_url(). Wrap the value with "
        "`cloacina::logging::mask_db_url(&url)` before logging, or "
        "suppress with `// allow(credential-logging): <reason>` on the "
        "line above the macro call.\n"
    )
    for rel, line, macro, ident in all_violations:
        print(f"  {rel}:{line}: {macro}!(...) references `{ident}`")
    print()
    print(f"Total: {len(all_violations)} violation(s) in {len(files)} file(s).")
    return 1


if __name__ == "__main__":
    sys.exit(main())
