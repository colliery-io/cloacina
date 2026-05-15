"""
Disk-space purge for Cloacina development trees.

`angreal purge` reclaims the biggest local consumers — per-example
`target/` directories, Python venvs, Docker volumes, and (optionally)
the cargo download cache. Shows reclaimed bytes per target.

Defaults to a safe set (workspace state only). Toggle `--cargo-cache`
to also drop `~/.cargo/registry/{cache,src}`, which forces a re-download
on the next build but reclaims a multi-GB tail that doesn't belong to
this project specifically.

Run `angreal purge --dry-run` first to preview.
"""

from __future__ import annotations

import shutil
import subprocess
from pathlib import Path
from typing import Iterable, List, Tuple

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
HOME = Path.home()


# ---------------------------------------------------------------------------
# Disk-usage helpers
# ---------------------------------------------------------------------------


def _dir_size(path: Path) -> int:
    """Return total size of a directory in bytes. 0 if missing."""
    if not path.exists():
        return 0
    total = 0
    for p in path.rglob("*"):
        try:
            if p.is_file() and not p.is_symlink():
                total += p.stat().st_size
        except (FileNotFoundError, PermissionError):
            continue
    return total


def _fmt(size: int) -> str:
    """Human-friendly size."""
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if size < 1024 or unit == "TB":
            return f"{size:6.1f} {unit}"
        size /= 1024
    return f"{size:.1f} TB"


# ---------------------------------------------------------------------------
# Target enumeration
# ---------------------------------------------------------------------------


def _workspace_targets() -> List[Path]:
    """Return every `target/` directory under PROJECT_ROOT, ignoring
    anything inside the cargo home (which we treat separately)."""
    cargo_home = (HOME / ".cargo").resolve()
    out: List[Path] = []
    for target in PROJECT_ROOT.rglob("target"):
        if not target.is_dir():
            continue
        try:
            if cargo_home in target.resolve().parents:
                continue
        except (FileNotFoundError, PermissionError):
            continue
        out.append(target)
    # Sort biggest first so the report leads with the wins.
    out.sort(key=_dir_size, reverse=True)
    return out


def _python_venvs() -> List[Path]:
    """The `test-env-*` venvs angreal builds for python scenarios."""
    return sorted(PROJECT_ROOT.glob("test-env-*"))


def _python_artifacts() -> List[Path]:
    """`__pycache__`, `.pytest_cache`, `*.egg-info` strewn across the tree."""
    out: List[Path] = []
    for pattern in ("__pycache__", ".pytest_cache", ".ruff_cache", "*.egg-info"):
        out.extend(p for p in PROJECT_ROOT.rglob(pattern) if p.is_dir())
    return out


def _cargo_cache_dirs() -> List[Path]:
    """Downloaded crate metadata + sources under `~/.cargo`. Safe to
    delete; cargo re-fetches transparently. Does NOT touch `~/.cargo/bin`
    so installed tools (angreal, cargo-watch, etc.) survive."""
    return [
        HOME / ".cargo" / "registry" / "cache",
        HOME / ".cargo" / "registry" / "src",
        HOME / ".cargo" / "git" / "checkouts",
        HOME / ".cargo" / "git" / "db",
    ]


# ---------------------------------------------------------------------------
# Docker
# ---------------------------------------------------------------------------


def _docker_compose_down(cwd: Path) -> int:
    """Stop docker services and remove volumes. Returns docker's exit code,
    or 0 if compose isn't on PATH."""
    compose_file = cwd / ".angreal" / "docker-compose.yaml"
    if not compose_file.exists():
        return 0
    if shutil.which("docker") is None:
        return 0
    return subprocess.run(
        ["docker", "compose", "-f", str(compose_file), "down", "-v", "--remove-orphans"],
        cwd=str(cwd),
        check=False,
    ).returncode


# ---------------------------------------------------------------------------
# Report + delete
# ---------------------------------------------------------------------------


def _report(label: str, paths: Iterable[Path]) -> Tuple[int, List[Tuple[Path, int]]]:
    """Print a sized inventory of paths and return (total, [(path, size)])."""
    rows: List[Tuple[Path, int]] = []
    for p in paths:
        rows.append((p, _dir_size(p)))
    total = sum(s for _, s in rows)
    if not rows:
        return total, rows
    print(f"\n=== {label} ({_fmt(total)} across {len(rows)} path(s)) ===")
    # Show only the top 12 to keep output readable; total covers all.
    for path, size in sorted(rows, key=lambda r: r[1], reverse=True)[:12]:
        try:
            rel = path.relative_to(PROJECT_ROOT)
        except ValueError:
            rel = path
        print(f"  {_fmt(size)}  {rel}")
    if len(rows) > 12:
        print(f"  … {len(rows) - 12} more")
    return total, rows


def _delete(paths: Iterable[Path], dry_run: bool) -> int:
    """Delete each path. Returns the count actually removed."""
    deleted = 0
    for path in paths:
        if not path.exists():
            continue
        if dry_run:
            print(f"  [dry-run] would rm -rf {path}")
            deleted += 1
            continue
        try:
            shutil.rmtree(path)
            deleted += 1
        except (FileNotFoundError, PermissionError) as exc:
            print(f"  warning: failed to remove {path}: {exc}")
    return deleted


# ---------------------------------------------------------------------------
# Command
# ---------------------------------------------------------------------------


@angreal.command(
    name="purge",
    about="reclaim disk space — target dirs, venvs, docker volumes, optional cargo cache",
    when_to_use=[
        "freeing disk space",
        "before/after long-running builds",
        "wedged build state",
    ],
    when_not_to_use=[
        "during active builds",
        "if you need fast incremental rebuilds (target/ deletion forces full rebuild)",
    ],
    tool=angreal.ToolDescription(
        "Reclaim disk space across the Cloacina dev tree. Removes every "
        "workspace `target/` directory, deletes Python test venvs, "
        "scrubs `__pycache__`/`.pytest_cache`/etc., and stops docker "
        "services with their volumes. With `--cargo-cache`, also drops "
        "`~/.cargo/registry/{cache,src}` and `~/.cargo/git/{checkouts,db}` "
        "(re-downloads transparently on next build). Use `--dry-run` to "
        "preview reclaim before committing.",
        risk_level="destructive",
    ),
)
@angreal.argument(
    name="dry_run",
    long="dry-run",
    takes_value=False,
    is_flag=True,
    help="report what would be deleted without removing anything",
)
@angreal.argument(
    name="cargo_cache",
    long="cargo-cache",
    takes_value=False,
    is_flag=True,
    help="also drop ~/.cargo/registry/{cache,src} and ~/.cargo/git (will re-download next build)",
)
@angreal.argument(
    name="keep_docker",
    long="keep-docker",
    takes_value=False,
    is_flag=True,
    help="skip docker compose down (keep volumes + containers)",
)
def purge(dry_run: bool = False, cargo_cache: bool = False, keep_docker: bool = False) -> int:
    """Disk purge with sized reporting."""
    mode = "dry-run" if dry_run else "executing"
    print(f"\n🧹 Cloacina purge — {mode}")
    print(f"    root: {PROJECT_ROOT}")
    if cargo_cache:
        print("    also dropping cargo registry + git caches (~/.cargo)")
    if keep_docker:
        print("    preserving docker services")

    # Inventory each bucket, then act.
    targets = _workspace_targets()
    venvs = _python_venvs()
    pyart = _python_artifacts()

    targets_total, _ = _report("Workspace `target/` directories", targets)
    venvs_total, _ = _report("Python test venvs", venvs)
    pyart_total, _ = _report("Python caches (__pycache__ / .pytest_cache / .ruff_cache / *.egg-info)", pyart)

    cargo_total = 0
    cargo_paths: List[Path] = []
    if cargo_cache:
        cargo_paths = _cargo_cache_dirs()
        cargo_total, _ = _report("Cargo download caches (~/.cargo)", cargo_paths)

    grand_total = targets_total + venvs_total + pyart_total + cargo_total
    print(f"\n=== Total reclaimable: {_fmt(grand_total)} ===")

    if dry_run:
        print("\n(dry run — no files removed)\n")
        return 0

    print("\n--- removing target dirs ---")
    _delete(targets, dry_run=False)

    print("\n--- removing Python venvs ---")
    _delete(venvs, dry_run=False)

    print("\n--- scrubbing Python caches ---")
    _delete(pyart, dry_run=False)

    if cargo_cache:
        print("\n--- dropping cargo caches ---")
        _delete(cargo_paths, dry_run=False)

    if not keep_docker:
        print("\n--- docker compose down --volumes ---")
        _docker_compose_down(PROJECT_ROOT)

    print(f"\n✓ purge complete — reclaimed ~{_fmt(grand_total)}")
    return 0
