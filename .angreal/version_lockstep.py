"""
Single source of version truth for Cloacina CORE (CLOACI-I-0134).

Standalone (no `angreal` import) so the pre-commit hook / CI can run it directly
with `python .angreal/version_lockstep.py check` and get readable output — the
angreal task runner swallows task stdout, so the guard lives here and
`task_release.py` is a thin wrapper.

Canonical version = `[workspace.package] version` in the root Cargo.toml.
Every other CORE touchpoint inherits it (Rust `{ workspace = true }`) or is
rewritten from it. Providers under `examples/` are independently versioned by
design (ADR A-0010 / I-0134 D-6) and are deliberately NOT touched or checked.
"""

import re
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent

_SEMVER = re.compile(r"^\d+\.\d+\.\d+$")

_JSON_FILES = [
    ("npm · typescript client", "clients/typescript/package.json"),
    ("npm · ui", "ui/package.json"),
    ("npm · ui harness", "ui/harness/package.json"),
]
_PY_PYPROJECT = ("python · client pyproject", "clients/python/pyproject.toml")
_PY_INIT = ("python · client __init__", "clients/python/src/cloacina_client/__init__.py")
_SCAFFOLD = ("scaffold · CLOACINA_CRATE_VERSION", "crates/cloacinactl/src/nouns/package/new.rs")


def _read(rel: str) -> str:
    return (PROJECT_ROOT / rel).read_text()


def _write(rel: str, text: str) -> None:
    (PROJECT_ROOT / rel).write_text(text)


def _minor(v: str) -> str:
    p = v.split(".")
    return f"{p[0]}.{p[1]}"


def source_version() -> str:
    """The canonical version: `version` under `[workspace.package]`."""
    text = _read("Cargo.toml")
    m = re.search(r"\[workspace\.package\][^\[]*?\bversion\s*=\s*\"([^\"]+)\"", text, re.DOTALL)
    if not m:
        raise SystemExit("version-lockstep: no [workspace.package] version in root Cargo.toml")
    return m.group(1)


def found_versions(source: str):
    """(label, found, expected) for every core touchpoint."""
    out = []
    cargo = _read("Cargo.toml")
    for m in re.finditer(
        r"^(cloacina[\w-]*)\s*=\s*\{\s*version\s*=\s*\"([^\"]+)\"[^}]*path\s*=\s*\"crates/",
        cargo,
        re.MULTILINE,
    ):
        out.append((f"Cargo · {m.group(1)} dep pin", m.group(2), source))
    for label, rel in _JSON_FILES:
        m = re.search(r'"version"\s*:\s*"([^"]+)"', _read(rel))
        out.append((label, m.group(1) if m else "<missing>", source))
    m = re.search(r'^version\s*=\s*"([^"]+)"', _read(_PY_PYPROJECT[1]), re.MULTILINE)
    out.append((_PY_PYPROJECT[0], m.group(1) if m else "<missing>", source))
    m = re.search(r'__version__\s*=\s*"([^"]+)"', _read(_PY_INIT[1]))
    out.append((_PY_INIT[0], m.group(1) if m else "<missing>", source))
    m = re.search(r'CLOACINA_CRATE_VERSION:\s*&str\s*=\s*"([^"]+)"', _read(_SCAFFOLD[1]))
    out.append((_SCAFFOLD[0], m.group(1) if m else "<missing>", _minor(source)))
    return out


def mismatches():
    source = source_version()
    return source, [(l, g, e) for (l, g, e) in found_versions(source) if g != e]


def set_version(new: str) -> None:
    source_minor = _minor(new)
    cargo = _read("Cargo.toml")
    cargo = re.sub(
        r"(\[workspace\.package\][^\[]*?\bversion\s*=\s*\")[^\"]+(\")",
        lambda mm: mm.group(1) + new + mm.group(2),
        cargo,
        count=1,
        flags=re.DOTALL,
    )
    cargo = re.sub(
        r"^(cloacina[\w-]*\s*=\s*\{\s*version\s*=\s*\")[^\"]+(\"[^}]*path\s*=\s*\"crates/)",
        lambda mm: mm.group(1) + new + mm.group(2),
        cargo,
        flags=re.MULTILINE,
    )
    _write("Cargo.toml", cargo)
    for _, rel in _JSON_FILES:
        t = _read(rel)
        t = re.sub(r'("version"\s*:\s*")[^"]+(")', r"\g<1>" + new + r"\g<2>", t, count=1)
        _write(rel, t)
    t = _read(_PY_PYPROJECT[1])
    t = re.sub(r'^(version\s*=\s*")[^"]+(")', r"\g<1>" + new + r"\g<2>", t, count=1, flags=re.MULTILINE)
    _write(_PY_PYPROJECT[1], t)
    t = _read(_PY_INIT[1])
    t = re.sub(r'(__version__\s*=\s*")[^"]+(")', r"\g<1>" + new + r"\g<2>", t, count=1)
    _write(_PY_INIT[1], t)
    t = _read(_SCAFFOLD[1])
    t = re.sub(r'(CLOACINA_CRATE_VERSION:\s*&str\s*=\s*")[^"]+(")', r"\g<1>" + source_minor + r"\g<2>", t, count=1)
    _write(_SCAFFOLD[1], t)


def changelog_stub(new: str) -> None:
    cl = PROJECT_ROOT / "CHANGELOG.md"
    if not cl.exists():
        return
    text = cl.read_text()
    if re.search(rf"^##\s*\[{re.escape(new)}\]", text, re.MULTILINE):
        return
    stub = f"## [{new}] - UNRELEASED\n\n### Added\n\n### Changed\n\n### Fixed\n\n"
    m = re.search(r"^##\s*\[", text, re.MULTILINE)
    text = (text[: m.start()] + stub + text[m.start():]) if m else (text.rstrip() + "\n\n" + stub)
    cl.write_text(text)


def run_check() -> int:
    source, bad = mismatches()
    if not bad:
        print(f"version-lockstep OK — {len(found_versions(source))} touchpoints all at {source}")
        return 0
    print(f"version DRIFT — canonical [workspace.package] version is {source}:", file=sys.stderr)
    for lbl, got, exp in bad:
        print(f"  ✗ {lbl}: found {got!r}, expected {exp!r}", file=sys.stderr)
    print("\nRun `angreal release bump <version>` to set every touchpoint from one input.", file=sys.stderr)
    return 1


def run_bump(new: str) -> int:
    if not _SEMVER.match(new):
        print(f"release bump: {new!r} is not MAJOR.MINOR.PATCH semver", file=sys.stderr)
        return 1
    old = source_version()
    set_version(new)
    changelog_stub(new)
    _, bad = mismatches()
    if bad:
        print("WARNING: touchpoints still drifted after bump:", file=sys.stderr)
        for lbl, got, exp in bad:
            print(f"  ✗ {lbl}: {got!r} != {exp!r}", file=sys.stderr)
        return 1
    print(f"bumped {old} -> {new} across all core touchpoints + CHANGELOG stub; version-lockstep verified.")
    print(f"Next: review, commit, then `git tag v{new}` when ready.")
    return 0


def main(argv) -> int:
    if not argv or argv[0] == "check":
        return run_check()
    if argv[0] == "bump":
        if len(argv) < 2:
            print("usage: version_lockstep.py bump <version>", file=sys.stderr)
            return 2
        return run_bump(argv[1])
    print(f"unknown command {argv[0]!r} (expected: check | bump <version>)", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
