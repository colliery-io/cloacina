"""
Run all tutorial projects.
"""

import angreal  # type: ignore
from . import tutorials, tutorial_commands

@tutorials()
@angreal.command(name="all", about="run all tutorial projects")
def all_tutorials():
    """Run all tutorial projects."""
    tutorials_to_run = [
        (name, cmd) for name, cmd in tutorial_commands.items()
    ]

    for name, tutorial_func in tutorials_to_run:
        print(f"\n=== Running {name} tutorial ===")
        result = tutorial_func()
        if result != 0:
            print(f"Tutorial {name} failed with exit code {result}")
            return result
        print(f"Tutorial {name} completed successfully!")

    print("\nAll tutorials completed successfully!")
    return 0
