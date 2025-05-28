"""
Local development tasks for Cloacina.
"""

import angreal  # type: ignore
import subprocess
import sys
import shutil
import time
from pathlib import Path

from utils import docker_up, docker_down, docker_clean

# Define command groups
services = angreal.command_group(name="services", about="commands for managing backing services")
dev = angreal.command_group(name="dev", about="commands for development tools")


@services()
@angreal.command(name="up", about="start backing services for local development")
def up():
    """Start backing services for local development."""
    return docker_up()


@services()
@angreal.command(name="down", about="stop backing services")
@angreal.argument(
    name="volumes",
    long="volumes",
    help="Remove volumes",
    takes_value=False,
    is_flag=True
)
def down(volumes=False):
    """Stop backing services."""
    return docker_down(volumes)


@services()
@angreal.command(name="reset", about="reset local services (stop and restart)")
@angreal.argument(
    name="clean",
    long="clean",
    help="Clean volumes",
    takes_value=False,
    is_flag=True
)
def reset(clean=False):
    """Reset local services (stop and restart)."""
    exit_code = docker_down(clean)
    if exit_code != 0:
        return exit_code

    return docker_up()


@services()
@angreal.command(name="clean", about="stop and remove services including volumes")
def clean():
    """Stop and remove services including volumes."""
    # First clean docker resources
    exit_code = docker_clean()
    if exit_code != 0:
        return exit_code

    # Remove root target directory
    project_root = Path(angreal.get_root()).parent
    root_target = project_root / "target"
    if root_target.exists():
        shutil.rmtree(root_target)

    # Remove target directories in examples
    examples_dir = project_root / "examples"
    if examples_dir.exists():
        for example_dir in examples_dir.iterdir():
            if example_dir.is_dir():
                target_dir = example_dir / "target"
                if target_dir.exists():
                    shutil.rmtree(target_dir)

    return 0


@dev()
@angreal.command(name="docs", about="generate and serve Rust documentation")
def docs():
    """Generate and serve Rust documentation."""
    subprocess.run(["cargo", "doc", "--no-deps", "--open"])
    return 0


@dev()
@angreal.command(name="schema", about="generate `src/schema.rs` from current migrations")
def schema():
    """Generate schema.rs from current migrations."""
    try:
        # Start Docker services
        exit_code = docker_up()
        if exit_code != 0:
            return exit_code

        # Wait for services to be ready
        print("Waiting for services to be ready...")
        time.sleep(30)

        db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"

        # Run migrations
        subprocess.run(
            f"diesel migration run --database-url {db_url}",
            cwd=str(Path(angreal.get_root()).parent / "cloacina/src/database"),
            shell=True,
            check=True
        )

        # Generate schema
        subprocess.run(
            f"diesel print-schema --database-url {db_url} > schema.rs",
            cwd=str(Path(angreal.get_root()).parent / "cloacina/src/database"),
            shell=True,
            check=True
        )

        print("Schema generated successfully!")
        return 0
    except subprocess.CalledProcessError as e:
        print(f"Schema generation failed with error: {e}", file=sys.stderr)
        return e.returncode
    finally:
        # Stop Docker services
        docker_down()
