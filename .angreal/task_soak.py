"""
Soak test task — fully containerized.

Builds cloacina server + example workflow in Docker, starts postgres,
uploads the workflow, and runs the soak test against real executions.
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
        "validating server end-to-end",
        "running real workflow executions under load",
        "pre-release smoke test",
    ],
    when_not_to_use=[
        "quick unit tests — use angreal cloacina unit",
        "integration tests — use angreal cloacina integration",
    ],
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
    help="number of concurrent worker threads",
    default="2",
)
@angreal.argument(
    name="profile",
    long="profile",
    short="p",
    help="load profile: light, medium, heavy",
    default="medium",
)
@angreal.argument(
    name="rebuild",
    long="rebuild",
    help="force rebuild of all containers",
    takes_value=False,
    is_flag=True,
)
def soak(duration="2m", concurrency="2", profile="medium", rebuild=False):
    """Run the fully containerized soak test."""

    compose_cmd = _docker_compose_cmd()

    # Tear down any previous run
    print("Cleaning up previous soak run...")
    subprocess.run(
        compose_cmd + ["down", "-v", "--remove-orphans"],
        cwd=str(PROJECT_ROOT),
        capture_output=True,
    )

    # Build
    build_cmd = compose_cmd + ["build"]
    if rebuild:
        build_cmd.append("--no-cache")
    print("Building containers (this may take a while on first run)...")
    rc = subprocess.run(build_cmd, cwd=str(PROJECT_ROOT)).returncode
    if rc != 0:
        print("ERROR: Container build failed", file=sys.stderr)
        return rc

    # Run everything with abort-on-container-exit (soak container exit stops the stack)
    print(f"\nRunning soak test: duration={duration}, concurrency={concurrency}, profile={profile}")
    print("=" * 70)
    env = dict(os.environ)
    env["SOAK_DURATION"] = duration
    env["SOAK_CONCURRENCY"] = concurrency
    env["SOAK_PROFILE"] = profile
    run_cmd = compose_cmd + ["up", "--abort-on-container-exit", "--exit-code-from", "soak"]
    rc = subprocess.run(run_cmd, cwd=str(PROJECT_ROOT), env=env).returncode

    # Cleanup
    _cleanup(compose_cmd)
    return rc


def _docker_compose_cmd():
    """Return the correct docker compose command."""
    return ["docker", "compose", "-f", str(COMPOSE_FILE)]


def _cleanup(compose_cmd):
    """Tear down soak containers."""
    print("\nCleaning up soak containers...")
    subprocess.run(
        compose_cmd + ["down", "-v", "--remove-orphans"],
        cwd=str(PROJECT_ROOT),
        capture_output=True,
    )
