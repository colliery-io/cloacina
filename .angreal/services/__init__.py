"""
Service tasks for Cloacina.
"""

import angreal  # type: ignore
from pathlib import Path

from utils import run_example_or_tutorial

# Project root for accessing services (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

services = angreal.command_group(name="services", about="run Cloacina service projects")

def get_service_directories():
    """Get all directories in examples folder that are services."""
    examples_dir = PROJECT_ROOT / "examples"
    return [
        d.name for d in examples_dir.iterdir()
        if d.is_dir() and d.name.startswith("service")
    ]

def create_service_command(command_group, dir_name):
    # Split on '-' and take the last part if it's a number
    parts = dir_name.split('-')
    if len(parts) > 1 and parts[0] == 'service' and parts[1].isdigit():
        command_name = parts[1]
    else:
        command_name = dir_name

    @command_group()
    @angreal.command(name=command_name, about=f"run the {dir_name} service project")
    def command():
        """Run the service project."""
        return run_example_or_tutorial(PROJECT_ROOT, f"examples/{dir_name}", f"{dir_name.title()} service")
    return command

# Dynamically create commands for services
service_commands = {}
for service_dir in get_service_directories():
    service_commands[service_dir] = create_service_command(services, service_dir)
