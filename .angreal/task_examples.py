"""
Example tasks for Cloacina.
"""

import angreal  # type: ignore
from pathlib import Path

from utils import run_example_or_tutorial

# Project root for accessing examples (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

examples = angreal.command_group(name="examples", about="run Cloacina example projects")

def get_example_directories():
    """Get all directories in examples folder that are not tutorials and are intended for execution."""
    examples_dir = PROJECT_ROOT / "examples"
    # Exclude validation_failures as it has multiple binaries and is not meant to be executed directly
    excluded_examples = {"validation_failures"}
    return [
        d.name for d in examples_dir.iterdir()
        if d.is_dir() and not d.name.startswith("tutorial") and d.name not in excluded_examples
    ]

def create_example_command(command_group, dir_name):
    # Convert directory name to command name (e.g., multi_tenant -> multi-tenant)
    command_name = dir_name.replace('_', '-')

    @command_group()
    @angreal.command(name=command_name, about=f"run the {dir_name} example project")
    def command():
        """Run the example project."""
        return run_example_or_tutorial(PROJECT_ROOT, f"examples/{dir_name}", f"{dir_name.replace('_', ' ').title()} example")
    return command

# Dynamically create commands for examples
example_commands = {}
for example_dir in get_example_directories():
    example_commands[example_dir] = create_example_command(examples, example_dir)
