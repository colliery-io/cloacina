"""The checker's rules, one file for each chapter (CLOACINA-T-1108).

A chapter adds a file here, named after the chapter, with a `register(registry)`
function. The checker finds the file by its name: there is no list to edit.
A file whose name starts with `_` holds shared helpers and is not a chapter.

A rules file names the sources it needs (`registry.need_api`,
`registry.need_db`) and its rules (`@registry.rule(...)`). See `standing.py`.
"""

from __future__ import annotations

import importlib
import pkgutil

from ..engine import Registry


def chapter_modules():
    """The names of the rules files, in order."""
    return sorted(info.name for info in pkgutil.iter_modules(__path__)
                  if not info.name.startswith("_"))


def load(registry=None):
    """Import every rules file and let it register. Returns the registry."""
    registry = registry or Registry()
    for name in chapter_modules():
        module = importlib.import_module(f"{__name__}.{name}")
        if not hasattr(module, "register"):
            raise ValueError(f"rules file {name}.py has no register(registry) function")
        module.register(registry)
    return registry
