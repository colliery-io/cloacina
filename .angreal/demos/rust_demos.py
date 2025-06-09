"""
Rust demo commands (examples and tutorials).
"""

import angreal  # type: ignore

from utils import run_example_or_tutorial

from .demos_utils import (
    PROJECT_ROOT,
    get_rust_example_directories,
    get_rust_tutorial_directories,
    normalize_command_name
)

# Define command group
demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")


def create_rust_demo_command(dir_name, demo_type):
    """Create a command for a Rust demo (example or tutorial)."""
    command_name = normalize_command_name(dir_name)

    # Determine the display name based on type
    if demo_type == "tutorial":
        # Extract tutorial number
        parts = dir_name.split('-')
        if len(parts) > 1 and parts[1].isdigit():
            display_name = f"Rust Tutorial {parts[1]}"
        else:
            display_name = f"Rust {dir_name.title()}"
    else:
        # Example
        display_name = f"{dir_name.replace('_', ' ').title()} Example"

    @demos()
    @angreal.command(name=command_name, about=f"run {display_name}")
    def command():
        """Run the demo project."""
        return run_example_or_tutorial(
            PROJECT_ROOT,
            f"examples/{dir_name}",
            display_name
        )

    # Store the function with a unique name to avoid conflicts
    command.__name__ = f"rust_demo_{command_name.replace('-', '_')}"
    return command


# Create commands for all Rust examples
rust_example_commands = {}
for example_dir in get_rust_example_directories():
    rust_example_commands[example_dir] = create_rust_demo_command(example_dir, "example")

# Create commands for all Rust tutorials
rust_tutorial_commands = {}
for tutorial_dir in get_rust_tutorial_directories():
    rust_tutorial_commands[tutorial_dir] = create_rust_demo_command(tutorial_dir, "tutorial")
