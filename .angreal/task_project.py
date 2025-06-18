"""
Unified task project file that imports commands from organized folders.

This replaces the individual task_*.py files with a cleaner structure where
each task group has its own folder with individual command files.
"""

# Import all command groups
from cloaca.generate import *  # noqa: F403
from cloaca.scrub import *  # noqa: F403
from cloaca.smoke import *  # noqa: F403
from cloaca.test import *  # noqa: F403
from cloaca.package import * # noqa: F403
from cloaca.release import * # noqa: F403

# Import all command modules to register the commands
from cloacina import unit  # noqa: F401
from cloacina import integration  # noqa: F401
from cloacina import macros  # noqa: F401
from cloacina import all  # noqa: F401

# Import demos module
from demos import rust_demos  # noqa: F401
from demos import python_demos  # noqa: F401
# from demos import all as demos_all  # noqa: F401  # all.py removed

# Import performance module
import performance  # noqa: F401
