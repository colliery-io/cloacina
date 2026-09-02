"""demos tutorials rust — run individual Rust tutorial examples."""

import angreal  # type: ignore

from utils import run_example_or_tutorial

from .._utils import (
    PROJECT_ROOT,
    get_rust_tutorial_directories,
)

demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")
tutorials = angreal.command_group(name="tutorials", about="run tutorial examples")
rust = angreal.command_group(name="rust", about="Rust tutorial examples")


def _register(dir_name, rel_path):
    parts = dir_name.split("-", 1)
    if parts[0].isdigit():
        number = parts[0]
        display_name = f"Rust Tutorial {number}"
        leaf = number
    else:
        display_name = f"Rust {dir_name.title()}"
        leaf = dir_name

    @demos()
    @tutorials()
    @rust()
    @angreal.command(
        name=leaf,
        about=f"run {display_name}",
        when_to_use=["learning Cloacina's Rust surface", "validating a tutorial change"],
        when_not_to_use=["production deployment", "performance benchmarking"],
    )
    def _cmd():
        return run_example_or_tutorial(PROJECT_ROOT, rel_path, display_name)

    _cmd.__name__ = f"rust_tutorial_{leaf}".replace("-", "_")
    return _cmd


_commands = {
    name: _register(name, path)
    for name, path in get_rust_tutorial_directories()
}


# --- `demos tutorials rust all` — every tutorial, one process -----------------
#
# CI used to run each tutorial in its own runner (10 legs). Every tutorial is
# its own `[workspace]` with path deps on the same crates, so each leg paid a
# full cold compile and a full runner setup for a ~2s program. Running them
# in sequence with a shared CARGO_TARGET_DIR compiles the dependency graph
# once; with CARGO_TARGET_DIR pointed at the root `target/` the deps are
# already there from the workspace build.

@demos()
@tutorials()
@rust()
@angreal.command(
    name="all",
    about="run every Rust tutorial in sequence (the CI lane form)",
    when_to_use=["CI tutorial gate", "validating all tutorials after a core change"],
    when_not_to_use=["iterating on a single tutorial (use its numbered command)"],
)
@angreal.argument(
    name="attempts",
    long="attempts",
    required=False,
    help="attempts per tutorial before it counts as failed (default 1)",
)
def run_all(attempts=None):
    """Run all discovered Rust tutorials, report every failure, exit non-zero
    if any failed. Per-tutorial retry replaces the old per-leg retry wrapper
    (docker-stack / crates.io flake tolerance)."""
    max_attempts = max(1, int(attempts or 1))
    entries = sorted(get_rust_tutorial_directories(), key=lambda e: e[0])
    if not entries:
        print("No Rust tutorials discovered.")
        return 1

    failed = []
    for dir_name, rel_path in entries:
        display_name = f"Rust Tutorial {dir_name}"
        for attempt in range(1, max_attempts + 1):
            print(f"\n=== {display_name} (attempt {attempt}/{max_attempts}) ===", flush=True)
            rc = run_example_or_tutorial(PROJECT_ROOT, rel_path, display_name)
            if rc == 0:
                break
            print(f"FAILED: {display_name} exited {rc}", flush=True)
        else:
            failed.append(dir_name)

    print("\n=== Rust tutorial summary ===", flush=True)
    for dir_name, _ in entries:
        print(f"  {'FAIL' if dir_name in failed else 'ok  '} {dir_name}", flush=True)
    if failed:
        print(f"\n{len(failed)} tutorial(s) failed: {', '.join(failed)}", flush=True)
        return 1
    print(f"\nAll {len(entries)} Rust tutorials passed.", flush=True)
    return 0
