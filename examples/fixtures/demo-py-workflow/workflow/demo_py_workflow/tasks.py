"""Demo Python task workflow (CLOACI-I-0117 / T-0664).

A minimal two-task Python workflow that the reconciler loads via PyO3 — proves
the Python packaging path end-to-end in the demo (Workflows + Executions).
"""
from __future__ import annotations

import cloaca


@cloaca.task(dependencies=[])
def prepare(context):
    context.set("demo_py_prepare", True)
    return context


@cloaca.task(dependencies=["prepare"])
def finish(context):
    context.set("demo_py_workflow_ran", True)
    return context
