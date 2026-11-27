"""The behaviour suite's shared code (CLOACINA-T-1107, CLOACINA-T-1108).

Everything in this package needs the standard library only and never imports
angreal: the driver (an angreal task) and the checker (its own program, run
with the system Python) both use it.

- `promises`: the promises table of the suite.
- `workload`: the standing workload, described once.
- `engine`, `sources`, `report`, `rules/`, `check.py`: the checker.
"""
