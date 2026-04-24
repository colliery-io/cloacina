"""Root-level registrar — imports each subpackage so its commands register.

Angreal auto-loads files matching `.angreal/task_*.py` at the top level; we
keep this file small and delegate all real logic to subpackages.
"""

# Test suites (unit, integration, macros, auth, all, coverage, metrics-format,
# e2e {cli, compiler, ws}, soak {daemon, server}).
import test  # noqa: F401

# Lint — fmt, clippy, credential-logging, all.
import lint  # noqa: F401

# CI local mirrors — fast, full.
import ci  # noqa: F401

# Demos — tutorials {rust, python}, features.
import demos  # noqa: F401

# Performance — simple, pipeline, parallel, computation-graph-bench, all, quick.
import performance  # noqa: F401
