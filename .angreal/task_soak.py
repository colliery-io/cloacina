"""
Soak test task — server (containerized) or daemon (local).

Server mode: builds cloacina server + workflows in Docker, starts postgres,
uploads workflows, and runs the soak test against real HTTP executions.

Daemon mode: builds packages locally, starts the daemon with SQLite,
registers workflows, schedules cron, and verifies executions.

Python packages are built using `cloacinactl package build` (pure Rust).
"""

import os
import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = PROJECT_ROOT / "deploy" / "docker-compose.soak.yml"


@angreal.command(
    name="soak",
    about="run fully containerized soak test (build + server + workflows + load)",
    when_to_use=[
        "validating server or daemon end-to-end",
        "running real workflow executions under load",
        "pre-release smoke test",
    ],
    when_not_to_use=[
        "quick unit tests — use angreal cloacina unit",
        "integration tests — use angreal cloacina integration",
    ],
)
@angreal.argument(
    name="mode",
    long="mode",
    short="m",
    help="test mode: server (Docker + postgres) or daemon (local + SQLite)",
    default="server",
)
@angreal.argument(
    name="duration",
    long="duration",
    short="d",
    help="test duration (e.g. 30s, 2m, 1h)",
    default="2m",
)
@angreal.argument(
    name="concurrency",
    long="concurrency",
    short="c",
    help="number of concurrent worker threads (server mode only)",
    default="2",
)
@angreal.argument(
    name="profile",
    long="profile",
    short="p",
    help="load profile: light, medium, heavy (server mode only)",
    default="medium",
)
@angreal.argument(
    name="rebuild",
    long="rebuild",
    help="force rebuild of all containers (server mode only)",
    takes_value=False,
    is_flag=True,
)
def soak(mode="server", duration="2m", concurrency="2", profile="medium", rebuild=False):
    """Run the soak test in server or daemon mode."""

    if mode == "server":
        return _soak_server(duration, concurrency, profile, rebuild)
    elif mode == "daemon":
        return _soak_daemon(duration)
    else:
        print(f"ERROR: Unknown mode '{mode}'. Use 'server' or 'daemon'.", file=sys.stderr)
        return 1


# ---------------------------------------------------------------------------
# Server mode (Docker)
# ---------------------------------------------------------------------------

def _soak_server(duration, concurrency, profile, rebuild):
    """Run the containerized server soak test."""

    compose_cmd = _docker_compose_cmd()

    # Tear down any previous run
    print("Cleaning up previous soak run...")
    subprocess.run(
        compose_cmd + ["down", "-v", "--remove-orphans"],
        cwd=str(PROJECT_ROOT),
        capture_output=True,
    )

    # Build Docker images
    build_cmd = compose_cmd + ["build"]
    if rebuild:
        build_cmd.append("--no-cache")
    print("Building containers (this may take a while on first run)...")
    rc = subprocess.run(build_cmd, cwd=str(PROJECT_ROOT)).returncode
    if rc != 0:
        print("ERROR: Container build failed", file=sys.stderr)
        return rc

    # Run with abort-on-container-exit
    print(f"\nRunning server soak: duration={duration}, concurrency={concurrency}, profile={profile}")
    print(f"  PROJECT_ROOT: {PROJECT_ROOT}")
    print("=" * 70)
    env = dict(os.environ)
    env["SOAK_DURATION"] = str(duration)
    env["SOAK_CONCURRENCY"] = str(concurrency or "2")
    env["SOAK_PROFILE"] = str(profile or "medium")

    run_cmd = compose_cmd + ["up", "--abort-on-container-exit", "--exit-code-from", "soak"]
    try:
        rc = subprocess.run(run_cmd, cwd=str(PROJECT_ROOT), env=env).returncode
    finally:
        _cleanup_server(compose_cmd)
    return rc


# ---------------------------------------------------------------------------
# Daemon mode (local)
# ---------------------------------------------------------------------------

def _soak_daemon(duration):
    """Run the daemon soak test locally with SQLite."""

    # Ensure cloacinactl is built
    ctl = PROJECT_ROOT / "target" / "debug" / "cloacinactl"
    if not ctl.exists():
        print("Building cloacinactl...", end=" ", flush=True)
        result = subprocess.run(
            ["cargo", "build", "-p", "cloacinactl"],
            cwd=str(PROJECT_ROOT),
            capture_output=True, text=True, timeout=300,
        )
        if result.returncode != 0:
            print(f"FAILED\n{result.stderr[:500]}")
            return 1
        print("OK")

    # Run daemon soak test
    print(f"\nRunning daemon soak: duration={duration}")
    print("=" * 70)

    cmd = [
        sys.executable,
        str(PROJECT_ROOT / "tests" / "soak" / "daemon_soak_test.py"),
        "--build",
        f"--duration={duration}",
        f"--cloacinactl={ctl}",
    ]

    rc = subprocess.run(cmd, cwd=str(PROJECT_ROOT)).returncode
    return rc


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _docker_compose_cmd():
    """Return the correct docker compose command."""
    return ["docker", "compose", "-f", str(COMPOSE_FILE)]


def _cleanup_server(compose_cmd):
    """Tear down soak containers."""
    print("\nCleaning up soak containers...")
    subprocess.run(
        compose_cmd + ["down", "-v", "--remove-orphans"],
        cwd=str(PROJECT_ROOT),
        capture_output=True,
    )
