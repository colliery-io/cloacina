"""
Python binding test tasks for Cloaca.

Uses composable functions from file_generation, build_operations, and file_operations
for clean, testable command implementations.
"""

import angreal  # type: ignore

# Define command group
cloaca = angreal.command_group(name="cloaca", about="commands for Python binding tests")

# Import task implementations

# Import command implementations
