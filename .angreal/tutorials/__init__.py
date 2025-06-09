"""
Tutorial tasks for Cloacina.
"""

import angreal  # type: ignore
from pathlib import Path

from utils import run_example_or_tutorial

# Project root for accessing tutorials (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

tutorials = angreal.command_group(name="tutorials", about="run Cloacina tutorial projects")

def get_tutorial_directories():
    """Get all directories in examples folder that are tutorials."""
    examples_dir = PROJECT_ROOT / "examples"
    return [
        d.name for d in examples_dir.iterdir()
        if d.is_dir() and d.name.startswith("tutorial")
    ]

def create_tutorial_command(command_group, dir_name):
    # Split on '-' and take the last part if it's a number
    parts = dir_name.split('-')
    if len(parts) > 1 and parts[0] == 'tutorial' and parts[1].isdigit():
        command_name = parts[1]
    else:
        command_name = dir_name

    @command_group()
    @angreal.command(name=command_name, about=f"run the {dir_name} tutorial project")
    def command():
        """Run the tutorial project."""
        return run_example_or_tutorial(PROJECT_ROOT, f"examples/{dir_name}", f"{dir_name.title()} tutorial")
    return command

# Dynamically create commands for tutorials
tutorial_commands = {}
for tutorial_dir in get_tutorial_directories():
    tutorial_commands[tutorial_dir] = create_tutorial_command(tutorials, tutorial_dir)
