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

# from cloaca.release import *


# from cloaca.tutorial import *
