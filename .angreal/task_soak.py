"""
Soak test task — fully containerized.

Builds cloacina server + example workflows in Docker, starts postgres,
uploads the workflows, and runs the soak test against real executions.

Rust packages are built inside the Docker container.
Python packages are built locally (via cloaca) and mounted in.
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

    # Build Python package locally (platform-independent, no Docker needed)
    python_package = _build_python_package()

    # Build Docker images
    build_cmd = compose_cmd + ["build"]
    if rebuild:
        build_cmd.append("--no-cache")
    print("Building containers (this may take a while on first run)...")
    rc = subprocess.run(build_cmd, cwd=str(PROJECT_ROOT)).returncode
    if rc != 0:
        print("ERROR: Container build failed", file=sys.stderr)
        return rc

    # Run everything with abort-on-container-exit
    print(f"\nRunning soak test: duration={duration}, concurrency={concurrency}, profile={profile}")
    print("=" * 70)
    env = dict(os.environ)
    env["SOAK_DURATION"] = duration
    env["SOAK_CONCURRENCY"] = concurrency
    env["SOAK_PROFILE"] = profile

    run_cmd = compose_cmd + ["up", "--abort-on-container-exit", "--exit-code-from", "soak"]

    # Mount the Python package into the soak container if it was built
    if python_package:
        # Use docker compose run with a volume mount for the Python package
        run_cmd = compose_cmd + [
            "run", "--rm", "--no-deps",
            "-v", f"{python_package}:/opt/soak/python-workflow.cloacina:ro",
            "-e", f"SOAK_DURATION={duration}",
            "-e", f"SOAK_CONCURRENCY={concurrency}",
            "-e", f"SOAK_PROFILE={profile}",
            "soak",
            "--url=http://cloacina:8080",
            "--package=/opt/soak/simple-packaged-demo.cloacina",
            "--bootstrap",
            "--cloacinactl=/usr/local/bin/cloacinactl",
            "--database-url=postgres://cloacina:cloacina@postgres:5432/cloacina",
            f"--duration={duration}",
            f"--profile={profile}",
            f"--concurrency={concurrency}",
        ]

        # Start dependencies first
        print("Starting postgres + cloacina server...")
        dep_cmd = compose_cmd + ["up", "-d", "postgres", "cloacina"]
        rc = subprocess.run(dep_cmd, cwd=str(PROJECT_ROOT), env=env).returncode
        if rc != 0:
            print("ERROR: Failed to start services", file=sys.stderr)
            _cleanup(compose_cmd)
            return rc

        # Wait for health
        print("Waiting for server health check...")
        import time
        for i in range(30):
            check = subprocess.run(
                compose_cmd + ["exec", "-T", "cloacina", "curl", "-sf",
                               "http://localhost:8080/health"],
                cwd=str(PROJECT_ROOT),
                capture_output=True,
            )
            if check.returncode == 0:
                print(f"Server healthy after {i+1}s")
                break
            time.sleep(1)
        else:
            print("ERROR: Server failed to become healthy", file=sys.stderr)
            _cleanup(compose_cmd)
            return 1

        rc = subprocess.run(run_cmd, cwd=str(PROJECT_ROOT), env=env).returncode
    else:
        # No Python package — run the standard compose up
        run_cmd = compose_cmd + ["up", "--abort-on-container-exit", "--exit-code-from", "soak"]
        rc = subprocess.run(run_cmd, cwd=str(PROJECT_ROOT), env=env).returncode

    # Cleanup
    _cleanup(compose_cmd)
    return rc


def _build_python_package():
    """Build a Python .cloacina package locally using the cloaca test harness.

    Returns the absolute path to the built package, or None if build fails.
    """
    print("Building Python workflow package locally...")

    try:
        from cloaca.cloaca_utils import _build_and_install_cloaca_unified
    except ImportError:
        print("  SKIP: cloaca_utils not available (run from project root with angreal)")
        return None

    venv_name = "soak-cloaca-venv"
    venv_path = PROJECT_ROOT / venv_name

    try:
        # Build and install cloaca
        venv, python_exe, pip_exe = _build_and_install_cloaca_unified(venv_name)

        # Install additional deps for cloaca build
        subprocess.run(
            [str(pip_exe), "install", "click", "pydantic"],
            check=True, capture_output=True,
        )

        # Build the Python workflow package
        output_dir = PROJECT_ROOT / "soak-packages"
        output_dir.mkdir(exist_ok=True)

        result = subprocess.run(
            [str(python_exe), "-c",
             "from cloaca.cli.build import build; "
             f"build(['-o', '{output_dir}'], standalone_mode=False)"],
            cwd=str(PROJECT_ROOT / "examples" / "features" / "python-workflow"),
            capture_output=True, text=True, timeout=60,
        )

        if result.returncode != 0:
            print(f"  WARNING: Python package build failed: {result.stderr[:200]}")
            return None

        # Find the built package
        packages = list(output_dir.glob("*.cloacina"))
        if packages:
            pkg = str(packages[0].resolve())
            print(f"  OK: {packages[0].name}")
            return pkg
        else:
            print("  WARNING: No .cloacina file produced")
            return None

    except Exception as e:
        print(f"  WARNING: Python package build failed: {e}")
        return None
    finally:
        # Clean up venv
        import shutil
        if venv_path.exists():
            shutil.rmtree(venv_path, ignore_errors=True)


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
    # Clean up soak packages dir
    import shutil
    soak_pkg_dir = PROJECT_ROOT / "soak-packages"
    if soak_pkg_dir.exists():
        shutil.rmtree(soak_pkg_dir, ignore_errors=True)
