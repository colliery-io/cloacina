"""Run the unit tests of the behaviour suite's shared code and checker.

    /usr/bin/python3 -I .angreal/test/bdd/suite/tests/run.py

Needs the standard library only: no server, no cluster, no database.
"""

from __future__ import annotations

import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
# The folder that holds the `suite` package.
sys.path.insert(0, os.path.dirname(os.path.dirname(HERE)))


def main():
    names = sorted(name[:-3] for name in os.listdir(HERE)
                   if name.startswith("test_") and name.endswith(".py"))
    suite = unittest.defaultTestLoader.loadTestsFromNames([f"suite.tests.{name}" for name in names])
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    sys.exit(main())
