"""
Code quality checking tasks for Cloacina.
"""

import concurrent.futures
import os
import shutil
import subprocess
from pathlib import Path
from typing import Dict, List, Tuple

import angreal  # type: ignore

# Project root for accessing all crates (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

# Define command group
check = angreal.command_group(name="check", about="commands for checking code quality")


# Cargo projects that aren't real build targets — skipped by `check all-crates`:
# - compiler-{broken,happy}-rust: templates with `__WORKSPACE__` placeholder
#   paths, rewritten at compiler-e2e test time.
# - validation-failures: intentionally fails to compile (negative-test demo).
SKIP_CRATES = {
    "examples/fixtures/compiler-broken-rust",
    "examples/fixtures/compiler-happy-rust",
    "examples/features/workflows/validation-failures",
}


def find_all_cargo_projects() -> List[Path]:
    """Find all Cargo.toml files tracked by git, excluding templates and
    intentionally-broken demos."""
    try:
        result = subprocess.run(
            ["git", "ls-files", "--", "*/Cargo.toml", "Cargo.toml"],
            capture_output=True,
            text=True,
            check=True,
            cwd=PROJECT_ROOT
        )

        paths = []
        for line in result.stdout.strip().split('\n'):
            if not line:
                continue
            rel_dir = str(Path(line).parent)
            if rel_dir in SKIP_CRATES:
                continue
            project_path = PROJECT_ROOT / Path(line).parent
            paths.append(project_path)

        return sorted(paths)
    except subprocess.CalledProcessError as e:
        print(f"Error finding cargo projects: {e}")
        return []


def run_cargo_command(project_path: Path, command: List[str]) -> Tuple[bool, str, str]:
    """Run a cargo command in the given project directory."""
    try:
        result = subprocess.run(
            ["cargo"] + command,
            cwd=project_path,
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout
        )
        return result.returncode == 0, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return False, "", "Command timed out after 5 minutes"
    except Exception as e:
        return False, "", f"Error running command: {e}"


def extract_warnings(stderr: str) -> List[str]:
    """Extract warning messages from cargo stderr output."""
    warnings = []
    lines = stderr.split('\n')

    current_warning = []
    in_warning = False

    for line in lines:
        if line.startswith('warning:'):
            if current_warning and in_warning:
                warnings.append('\n'.join(current_warning))
            current_warning = [line]
            in_warning = True
        elif in_warning and (line.startswith('   ') or line.startswith('  ') or line.strip() == ''):
            current_warning.append(line)
        elif in_warning and line.strip():
            # End of current warning
            warnings.append('\n'.join(current_warning))
            current_warning = []
            in_warning = False

    if current_warning and in_warning:
        warnings.append('\n'.join(current_warning))

    return warnings


def _is_standalone_workspace(project_path: Path) -> bool:
    """True iff the project declares its own `[workspace]` and therefore
    builds into its OWN `target/` (not the repo-root shared one).

    Cleaning the workspace-root `target/` mid-run would destroy other
    crates' build state, so we only sweep targets for standalone projects.
    """
    try:
        if project_path.resolve() == PROJECT_ROOT.resolve():
            return False
    except OSError:
        return False
    manifest = project_path / "Cargo.toml"
    if not manifest.is_file():
        return False
    try:
        for raw in manifest.read_text(encoding="utf-8", errors="replace").splitlines():
            line = raw.strip()
            if line.startswith("[workspace]"):
                return True
            # Stop scanning once we hit another section header — workspace
            # declarations always appear before [package] / [dependencies]
            # in the manifests we ship.
            if line.startswith("[") and line not in ("[workspace]",):
                # Keep scanning past comments; only stop on actual sections.
                if not line.startswith("[["):
                    # Heuristic: workspace is typically declared first.
                    # We've passed it without seeing it, so it's not here.
                    return False
    except OSError:
        return False
    return False


def _sweep_target(project_path: Path) -> int:
    """Remove the project's local `target/` if present. Returns bytes freed.
    Only called for standalone-workspace projects."""
    target = project_path / "target"
    if not target.is_dir():
        return 0
    freed = 0
    for p in target.rglob("*"):
        try:
            if p.is_file() and not p.is_symlink():
                freed += p.stat().st_size
        except (FileNotFoundError, PermissionError):
            continue
    try:
        shutil.rmtree(target)
    except (FileNotFoundError, PermissionError):
        return 0
    return freed


def check_single_crate(
    project_path: Path,
    show_warnings: bool = True,
    cleanup: bool = True,
) -> Dict:
    """Check a single crate with cargo check and cargo build.

    `cleanup=True` deletes the project's local `target/` after the build
    completes — but ONLY for standalone-workspace projects (each example
    has its own `target/`). Workspace-member crates under `crates/` share
    the repo-root `target/` and are never swept here.
    """
    relative_path = str(project_path.relative_to(PROJECT_ROOT))
    result = {
        "path": relative_path,
        "check": {"success": False, "warnings": [], "errors": []},
        "build": {"success": False, "warnings": [], "errors": []},
        "freed_bytes": 0,
    }

    # Run cargo check
    success, stdout, stderr = run_cargo_command(project_path, ["check", "--all-targets"])
    result["check"]["success"] = success
    result["check"]["warnings"] = extract_warnings(stderr)
    if not success and not result["check"]["warnings"]:
        result["check"]["errors"] = [stderr.strip()]

    # Run cargo build if check succeeded
    if success:
        success, stdout, stderr = run_cargo_command(project_path, ["build", "--all-targets"])
        result["build"]["success"] = success
        result["build"]["warnings"] = extract_warnings(stderr)
        if not success and not result["build"]["warnings"]:
            result["build"]["errors"] = [stderr.strip()]
    else:
        result["build"]["errors"] = ["Skipped due to check failure"]

    # Sweep the local target/ — only if this is a standalone workspace
    # (so we don't clobber crates that share the repo-root target/).
    if cleanup and _is_standalone_workspace(project_path):
        result["freed_bytes"] = _sweep_target(project_path)

    return result


def print_crate_result(result: Dict, show_warnings: bool = True):
    """Print results for a single crate."""
    path = result["path"]
    check_success = result["check"]["success"]
    build_success = result["build"]["success"]

    # Status indicator
    if check_success and build_success:
        status = "✅"
    elif check_success:
        status = "⚠️ "
    else:
        status = "❌"

    print(f"{status} {path}")

    # Check errors
    if result["check"]["errors"]:
        print("  Check errors:")
        for error in result["check"]["errors"]:
            for line in error.split('\n'):
                if line.strip():
                    print(f"    {line}")

    # Build errors
    if result["build"]["errors"] and result["build"]["errors"][0] != "Skipped due to check failure":
        print("  Build errors:")
        for error in result["build"]["errors"]:
            for line in error.split('\n'):
                if line.strip():
                    print(f"    {line}")

    # Warnings
    if show_warnings:
        all_warnings = result["check"]["warnings"] + result["build"]["warnings"]
        if all_warnings:
            print(f"  Warnings ({len(all_warnings)}):")
            for warning in all_warnings:
                for line in warning.split('\n'):
                    if line.strip():
                        print(f"    {line}")


@check()
@angreal.command(
    name="crate",
    about="run cargo check and build on a specific crate",
    when_to_use=["checking single crate during development", "debugging specific compilation issues", "focused testing"],
    when_not_to_use=["checking entire codebase", "CI validation", "pre-commit checks"]
)
@angreal.argument(
    name="path",
    help="path to crate directory (relative to project root)",
    required=True
)
@angreal.argument(
    name="no_warnings",
    long="no-warnings",
    help="hide warnings, show only errors",
    takes_value=False,
    is_flag=True
)
def crate(path: str, no_warnings=False):
    """Check a specific crate with cargo check and build."""
    crate_path = PROJECT_ROOT / path

    if not crate_path.exists():
        print(f"❌ Path does not exist: {path}")
        return 1

    cargo_toml = crate_path / "Cargo.toml"
    if not cargo_toml.exists():
        print(f"❌ No Cargo.toml found in: {path}")
        return 1

    print(f"🔍 Checking crate: {path}")

    result = check_single_crate(crate_path, show_warnings=not no_warnings)
    print_crate_result(result, show_warnings=not no_warnings)

    # Return appropriate exit code
    if not result["check"]["success"]:
        return 1
    elif not result["build"]["success"]:
        return 1
    else:
        return 0


def _default_jobs() -> int:
    """Default parallelism: 1/4 of cores, clamped to [2, 8].

    Cargo itself parallelizes within a single build, so running too many
    concurrent cargo invocations over-subscribes the box. The workspace
    members share `target/` and serialize on cargo's build lock anyway —
    the parallel wins come from the example crates that each have their
    own `target/`. Empirically `cpu_count // 4` balances throughput
    against memory pressure during link steps.
    """
    cpu = os.cpu_count() or 4
    return max(2, min(8, cpu // 4))


@check()
@angreal.command(
    name="all-crates",
    about="run cargo check and build on all crates (workspace and standalone) in parallel",
    when_to_use=["before commits", "CI validation", "comprehensive code quality checks", "finding compilation issues across codebase"],
    when_not_to_use=["quick local testing", "when focusing on specific crate", "during active development of single component"]
)
@angreal.argument(
    name="warnings_only",
    long="warnings-only",
    help="only show crates with warnings, skip successful builds",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="no_warnings",
    long="no-warnings",
    help="hide warnings, show only errors",
    takes_value=False,
    is_flag=True
)
@angreal.argument(
    name="jobs",
    long="jobs",
    short="j",
    help="parallel cargo invocations (default: cpu_count/4, clamped [2,8])",
    takes_value=True,
)
@angreal.argument(
    name="serial",
    long="serial",
    help="force serial execution (equivalent to --jobs 1)",
    takes_value=False,
    is_flag=True,
)
@angreal.argument(
    name="no_clean",
    long="no-clean",
    help="keep per-example target/ dirs after build (default: sweep to reclaim disk)",
    takes_value=False,
    is_flag=True,
)
def all_crates(warnings_only=False, no_warnings=False, jobs=None, serial=False, no_clean=False):
    """Check all crates in parallel. Workspace members serialize on cargo's
    build lock; standalone example crates parallelize cleanly."""
    print("🔍 Finding all cargo projects...")
    projects = find_all_cargo_projects()

    if not projects:
        print("❌ No cargo projects found")
        return 1

    # Resolve parallelism.
    if serial:
        worker_count = 1
    else:
        try:
            worker_count = int(jobs) if jobs is not None else _default_jobs()
        except (TypeError, ValueError):
            worker_count = _default_jobs()
        worker_count = max(1, worker_count)

    print(f"Found {len(projects)} projects (running {worker_count} in parallel)\n")

    results: List[Dict] = []
    failed_crates: List[str] = []
    completed = 0
    total = len(projects)

    show_warnings = not no_warnings
    cleanup = not no_clean

    # Run cargo check+build across `worker_count` threads. `run_cargo_command`
    # blocks in subprocess.run; the GIL is released for the duration, so a
    # plain ThreadPoolExecutor is the right tool — no need for processes.
    with concurrent.futures.ThreadPoolExecutor(max_workers=worker_count) as pool:
        future_to_project = {
            pool.submit(check_single_crate, project, show_warnings, cleanup): project
            for project in projects
        }
        try:
            for future in concurrent.futures.as_completed(future_to_project):
                project = future_to_project[future]
                rel = str(project.relative_to(PROJECT_ROOT))
                completed += 1
                try:
                    result = future.result()
                except Exception as exc:
                    print(f"❌ [{completed}/{total}] {rel}: orchestration error: {exc}")
                    failed_crates.append(rel)
                    continue

                results.append(result)
                if not result["check"]["success"] or not result["build"]["success"]:
                    failed_crates.append(result["path"])

                # Compact progress line: status only for green crates so the
                # log stays readable; full per-crate detail rendered in the
                # sorted re-print below for anything noisy.
                check_ok = result["check"]["success"]
                build_ok = result["build"]["success"]
                if check_ok and build_ok and not (result["check"]["warnings"] or result["build"]["warnings"]):
                    print(f"✅ [{completed}/{total}] {result['path']}")
        except KeyboardInterrupt:
            print("\n⚠️  Interrupted by user — cancelling pending jobs")
            for f in future_to_project:
                f.cancel()
            return 1

    # Re-print noisy crates in stable (alphabetical) order so the report
    # is reproducible across runs regardless of completion order.
    results.sort(key=lambda r: r["path"])
    print()
    for result in results:
        check_ok = result["check"]["success"]
        build_ok = result["build"]["success"]
        has_warnings = bool(result["check"]["warnings"] or result["build"]["warnings"])

        if warnings_only:
            total_w = len(result["check"]["warnings"]) + len(result["build"]["warnings"])
            has_err = result["check"]["errors"] or (
                result["build"]["errors"]
                and result["build"]["errors"][0] != "Skipped due to check failure"
            )
            if total_w == 0 and not has_err:
                continue
            print_crate_result(result, show_warnings=show_warnings)
            continue

        # Default mode: re-print anything that wasn't already shown as a
        # bare-green status line.
        if not (check_ok and build_ok and not has_warnings):
            print_crate_result(result, show_warnings=show_warnings)

    # Print summary
    total_projects = len(results)
    check_failures = sum(1 for r in results if not r["check"]["success"])
    build_failures = sum(1 for r in results if not r["build"]["success"])
    total_warnings = sum(len(r["check"]["warnings"]) + len(r["build"]["warnings"]) for r in results)
    total_freed = sum(r.get("freed_bytes", 0) for r in results)

    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    print(f"Total projects: {total_projects}")
    print(f"Check failures: {check_failures}")
    print(f"Build failures: {build_failures}")
    print(f"Total warnings: {total_warnings}")
    if cleanup and total_freed > 0:
        # Pretty-print bytes reclaimed.
        size = float(total_freed)
        unit = "B"
        for u in ("KB", "MB", "GB", "TB"):
            if size < 1024:
                break
            size /= 1024
            unit = u
        print(f"Disk reclaimed: {size:.1f} {unit} (per-example target/ dirs swept)")
    elif not cleanup:
        print("Disk reclaim: disabled (--no-clean)")

    if failed_crates:
        print("\nFailed crates:")
        for crate in failed_crates:
            print(f"  - {crate}")

    # Return exit code based on failures
    if check_failures > 0 or build_failures > 0:
        return 1
    else:
        return 0
