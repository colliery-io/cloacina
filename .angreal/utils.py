"""
Utility functions for Cloacina development.
"""

import subprocess
import sys
import time
from pathlib import Path
import logging

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root())
DOCKER_COMPOSE_FILE = PROJECT_ROOT / "docker-compose.yaml"

# Set up logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger(__name__)

def docker_up():
    """Start docker containers for local development."""
    try:
        # Try docker compose first (newer), then fall back to docker-compose
        try:
            subprocess.run(
                ["docker", "compose", "-f", str(DOCKER_COMPOSE_FILE), "up", "-d"],
                check=True
            )
        except (subprocess.CalledProcessError, FileNotFoundError):
            subprocess.run(
                ["docker-compose", "-f", str(DOCKER_COMPOSE_FILE), "up", "-d"],
                check=True
            )
        print("Docker services started successfully.")
    except subprocess.CalledProcessError as e:
        print(f"Error starting Docker services: {e}", file=sys.stderr)
        return 1
    return 0


def docker_down(remove_volumes=False):
    """Stop docker containers for local development."""
    try:
        # Try docker compose first (newer), then fall back to docker-compose
        cmd_docker = ["docker", "compose", "-f", str(DOCKER_COMPOSE_FILE), "down"]
        cmd_docker_compose = ["docker-compose", "-f", str(DOCKER_COMPOSE_FILE), "down"]
        if remove_volumes:
            cmd_docker.append("-v")
            cmd_docker_compose.append("-v")

        try:
            subprocess.run(cmd_docker, check=True)
        except (subprocess.CalledProcessError, FileNotFoundError):
            subprocess.run(cmd_docker_compose, check=True)
        print("Docker services stopped successfully.")
    except subprocess.CalledProcessError as e:
        print(f"Error stopping Docker services: {e}", file=sys.stderr)
        return 1
    return 0


def docker_clean():
    """Remove docker volumes for clean restart."""
    try:
        # Try docker compose first (newer), then fall back to docker-compose
        try:
            subprocess.run(
                ["docker", "compose", "-f", str(DOCKER_COMPOSE_FILE), "down", "-v"],
                check=True
            )
        except (subprocess.CalledProcessError, FileNotFoundError):
            subprocess.run(
                ["docker-compose", "-f", str(DOCKER_COMPOSE_FILE), "down", "-v"],
                check=True
            )
        print("Docker services and volumes cleaned successfully.")
    except subprocess.CalledProcessError as e:
        print(f"Error cleaning Docker volumes: {e}", file=sys.stderr)
        return 1
    return 0

def run_cargo_command(cwd, command_args, check=True):
    """Run a cargo command in the specified directory.

    Args:
        cwd: Working directory to run the command in
        command_args: List of arguments to pass to cargo
        check: Whether to check the return code (default: True)

    Returns:
        The return code from the command
    """
    try:
        result = subprocess.run(
            ["cargo"] + command_args,
            cwd=str(cwd),
            check=check
        )
        return result.returncode
    except subprocess.CalledProcessError as e:
        print(f"Command failed with error: {e}", file=sys.stderr)
        return e.returncode

def run_example_or_tutorial(project_root, example_dir, name, is_test=False, binary=None):
    """Run an example or tutorial with consistent setup.

    Args:
        project_root: Root directory of the project
        example_dir: Directory containing the example/tutorial
        name: Name of the example/tutorial for logging
        is_test: Whether to run as a test (default: False)
        binary: Specific binary to run (default: None)

    Returns:
        The return code from the command
    """
    # Check if this is a tutorial (SQLite-based) or other example (potentially PostgreSQL-based)
    is_tutorial = "tutorial" in example_dir
    
    if not is_tutorial:
        # For non-tutorial examples, check if Docker services are running
        try:
            # Try docker compose first (newer), then fall back to docker-compose
            try:
                result = subprocess.run(
                    ["docker", "compose", "-f", str(DOCKER_COMPOSE_FILE), "ps", "--services", "--filter", "status=running"],
                    check=True,
                    capture_output=True,
                    text=True
                )
            except (subprocess.CalledProcessError, FileNotFoundError):
                result = subprocess.run(
                    ["docker-compose", "-f", str(DOCKER_COMPOSE_FILE), "ps", "--services", "--filter", "status=running"],
                    check=True,
                    capture_output=True,
                    text=True
                )
            # If we get here, the command succeeded, but we need to check if any services are actually running
            services_running = bool(result.stdout.strip())
        except subprocess.CalledProcessError:
            services_running = False

        # Only restart Docker if services aren't running
        if not services_running:
            # Clean restart
            exit_code = docker_down(True)
            if exit_code != 0:
                return exit_code

            # Start Docker services
            exit_code = docker_up()
            if exit_code != 0:
                return exit_code

            # Wait for services to be ready
            print("Waiting for services to be ready...")
            time.sleep(10)
    else:
        # For tutorials, SQLite is used - no Docker setup needed
        print(f"Running {name} (SQLite-based, no database setup required)")

    # Run the example/tutorial
    if is_test:
        return run_cargo_command(
            project_root / example_dir,
            ["test", name, "--", "--nocapture"]
        )
    else:
        cmd = ["run"]
        if binary:
            cmd.extend(["--bin", binary])
        return run_cargo_command(
            project_root / example_dir,
            cmd
        )
