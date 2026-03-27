"""
Python tutorial demo commands.

NOTE: Python tutorials require the cloaca module which is now embedded
in cloacina core via PyO3. These tutorials cannot run in a standalone
Python venv — they need a cloacina-powered runtime. The Python integration
is tested via `angreal cloaca smoke` which runs the Rust-side Python tests.
"""

import angreal  # type: ignore

from .demos_utils import (
    get_python_tutorial_files,
    normalize_command_name,
)

# Define command group
demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")


def create_python_tutorial_command(tutorial_file):
    """Create a command for a Python tutorial."""
    parts = tutorial_file.replace(".py", "").split("_")
    if len(parts) >= 1 and parts[0].isdigit():
        tutorial_num = parts[0]
        command_name = f"python-tutorial-{tutorial_num}"
    else:
        command_name = normalize_command_name(tutorial_file)
        tutorial_num = "??"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run Python Tutorial {tutorial_num}",
        when_to_use=[
            "Testing Cloacina functionality with Python examples",
        ],
        when_not_to_use=[
            "These tutorials are currently being reworked for native Python support",
        ],
    )
    def command():
        """Run the Python tutorial.

        NOTE: Python tutorials are being reworked for native Python in core.
        The cloaca module is now embedded in cloacina via PyO3 and cannot
        be installed in a standalone Python venv.

        Use `angreal cloaca smoke` to verify Python integration works.
        """
        print(f"Python Tutorial {tutorial_num}: {tutorial_file}")
        print()
        print("NOTE: Python tutorials are being reworked for native Python support.")
        print("The cloaca module is now embedded in cloacina core via PyO3.")
        print("It cannot be installed in a standalone Python venv.")
        print()
        print("To verify Python integration:")
        print("  angreal cloaca smoke    — runs Rust-side Python integration tests")
        print("  angreal cloaca test     — comprehensive Python tests")
        print()
        print("See CLOACI-T-0271 for the rework plan.")

    command.__name__ = f"python_tutorial_{tutorial_num}"
    return command


# Create commands for all Python tutorials
python_tutorial_commands = {}
for tutorial_file in get_python_tutorial_files():
    python_tutorial_commands[tutorial_file] = create_python_tutorial_command(tutorial_file)
