"""
Service management tasks for Cloacina.
"""

import angreal  # type: ignore
import shutil
from pathlib import Path

from utils import docker_up, docker_down, docker_clean

# Define command group
services = angreal.command_group(name="services", about="commands for managing backing services")


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
