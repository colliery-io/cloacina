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

class FileOperationError(Exception):
    """Raised when file operations fail."""
    pass

def write_file_safe(file_path: Path, content: str, backup: bool = True) -> None:
    """Safely write content to a file, optionally backing up existing content.

    Args:
        file_path: Path to write to
        content: Content to write
        backup: Whether to backup existing file
    """
    try:
        if backup and file_path.exists():
            backup_path = file_path.with_suffix(file_path.suffix + ".bak")
            file_path.rename(backup_path)

        file_path.write_text(content)
    except Exception as e:
        raise FileOperationError(f"Failed to write file {file_path}: {e}")

def get_workspace_version() -> str:
    """Get version from workspace's Cargo.toml.

    Returns:
        Version string from Cargo.toml

    Raises:
        FileOperationError: If Cargo.toml not found or version cannot be extracted
    """
    cargo_toml = Path("Cargo.toml")
    if not cargo_toml.exists():
        raise FileOperationError("Cargo.toml not found")

    try:
        content = cargo_toml.read_text()
        version_line = next(line for line in content.splitlines() if line.startswith("version ="))
        version = version_line.split("=")[1].strip().strip('"')
        return version
    except Exception as e:
        raise FileOperationError(f"Failed to extract version from Cargo.toml: {e}")

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
    
    # Tutorial-06 (multi-tenancy) needs PostgreSQL for the advanced admin demo
    needs_postgres = not is_tutorial or "tutorial-06" in example_dir

    if needs_postgres:
        # For examples and tutorial-06, check if Docker services are running
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
        # For most tutorials, SQLite is used - no Docker setup needed
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

def check_postgres_container_health() -> bool:
    """Check if PostgreSQL container is healthy.

    Returns:
        True if container is healthy, False otherwise
    """
    try:
        result = subprocess.run(
            ["docker", "ps", "--filter", "name=postgres", "--format", "{{.Status}}"],
            capture_output=True,
            text=True
        )
        return "healthy" in result.stdout
    except Exception:
        return False

def smart_postgres_reset() -> bool:
    """Reset PostgreSQL state.

    This function will:
    1. Try to reset using SQL if possible
    2. Fall back to container restart if needed

    Returns:
        True if reset was successful, False otherwise
    """
    try:
        # First try SQL reset
        result = subprocess.run(
            [
                "docker", "exec", "cloacina-postgres",
                "psql", "-U", "cloacina", "-d", "cloacina",
                "-c", "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
            ],
            capture_output=True,
            text=True
        )

        if result.returncode == 0:
            return True

        # If SQL reset fails, log the error and try container restart
        print(f"SQL reset failed with return code {result.returncode}")
        if result.stdout:
            print("STDOUT:", result.stdout)
        if result.stderr:
            print("STDERR:", result.stderr)
        print("Falling back to container restart...")

        docker_down()
        time.sleep(2)  # Wait for container to fully stop
        if docker_up() != 0:
            return False
        time.sleep(10)  # Wait for container to be ready
        return check_postgres_container_health()

    except Exception as e:
        print(f"Error during PostgreSQL reset: {e}")
        return False
