"""demos tutorials rust — run individual Rust tutorial examples."""

import angreal  # type: ignore

from utils import run_example_or_tutorial

from .._utils import (
    PROJECT_ROOT,
    get_rust_tutorial_directories,
)

demos = angreal.command_group(name="demos", about="run Cloacina demonstration projects")
tutorials = angreal.command_group(name="tutorials", about="run tutorial examples")
rust = angreal.command_group(name="rust", about="Rust tutorial examples")


def _register(dir_name, rel_path):
    parts = dir_name.split("-", 1)
    if parts[0].isdigit():
        number = parts[0]
        display_name = f"Rust Tutorial {number}"
        leaf = number
    else:
        display_name = f"Rust {dir_name.title()}"
        leaf = dir_name

    @demos()
    @tutorials()
    @rust()
    @angreal.command(
        name=leaf,
        about=f"run {display_name}",
        when_to_use=["learning Cloacina's Rust surface", "validating a tutorial change"],
        when_not_to_use=["production deployment", "performance benchmarking"],
    )
    def _cmd():
        return run_example_or_tutorial(PROJECT_ROOT, rel_path, display_name)

    _cmd.__name__ = f"rust_tutorial_{leaf}".replace("-", "_")
    return _cmd


_commands = {
    name: _register(name, path)
    for name, path in get_rust_tutorial_directories()
}
