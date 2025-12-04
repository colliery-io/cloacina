"""
Rust demo commands (examples and tutorials).
"""

import angreal  # type: ignore

from utils import run_example_or_tutorial

from .demos_utils import (
    PROJECT_ROOT,
    get_rust_tutorial_directories,
    get_rust_feature_directories,
    get_rust_performance_directories,
    normalize_command_name
)

# Define command group
demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")


def create_rust_tutorial_command(dir_name):
    """Create a command for a Rust tutorial."""
    command_name = normalize_command_name(dir_name)

    # Extract tutorial number
    parts = dir_name.split('-')
    if len(parts) > 0 and parts[0].isdigit():
        display_name = f"Rust Tutorial {parts[0]}"
    else:
        display_name = f"Rust {dir_name.title()}"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run {display_name}",
        when_to_use=[
            "Testing Cloacina Rust integration",
            "Learning workflow patterns in Rust",
            "Validating compiled workflow performance",
            "Demonstrating native Rust capabilities"
        ],
        when_not_to_use=[
            "Production deployment scenarios",
            "Memory-intensive workflows",
            "Cross-language integrations",
            "Debugging workflow internals"
        ]
    )
    def command():
        """Run the tutorial project."""
        return run_example_or_tutorial(
            PROJECT_ROOT,
            f"examples/tutorials/{dir_name}",
            display_name
        )

    command.__name__ = f"rust_tutorial_{command_name.replace('-', '_')}"
    return command


def create_rust_feature_command(dir_name):
    """Create a command for a Rust feature example."""
    command_name = dir_name.replace('_', '-')
    display_name = f"{dir_name.replace('-', ' ').replace('_', ' ').title()} Example"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run {display_name}",
        when_to_use=[
            "Testing Cloacina features",
            "Demonstrating specific capabilities",
            "Validating feature implementations"
        ],
        when_not_to_use=[
            "Production deployment",
            "Performance testing"
        ]
    )
    def command():
        """Run the feature example."""
        return run_example_or_tutorial(
            PROJECT_ROOT,
            f"examples/features/{dir_name}",
            display_name
        )

    command.__name__ = f"rust_feature_{command_name.replace('-', '_')}"
    return command


def create_rust_performance_command(dir_name):
    """Create a command for a Rust performance example."""
    command_name = f"perf-{dir_name}"
    display_name = f"Performance {dir_name.title()}"

    @demos()
    @angreal.command(
        name=command_name,
        about=f"run {display_name}",
        when_to_use=[
            "Performance benchmarking",
            "Testing execution speed",
            "Validating optimizations"
        ],
        when_not_to_use=[
            "Learning workflows",
            "Feature testing"
        ]
    )
    def command():
        """Run the performance example."""
        return run_example_or_tutorial(
            PROJECT_ROOT,
            f"examples/performance/{dir_name}",
            display_name
        )

    command.__name__ = f"rust_perf_{command_name.replace('-', '_')}"
    return command


# Create commands for all Rust tutorials
rust_tutorial_commands = {}
for tutorial_dir in get_rust_tutorial_directories():
    rust_tutorial_commands[tutorial_dir] = create_rust_tutorial_command(tutorial_dir)

# Create commands for all Rust feature examples
rust_feature_commands = {}
for feature_dir in get_rust_feature_directories():
    rust_feature_commands[feature_dir] = create_rust_feature_command(feature_dir)

# Create commands for all Rust performance examples
rust_performance_commands = {}
for perf_dir in get_rust_performance_directories():
    rust_performance_commands[perf_dir] = create_rust_performance_command(perf_dir)
