"""Demo Python task workflow (CLOACI-I-0117 / T-0664).

A branching Python workflow that the reconciler loads via PyO3 — proves the
Python packaging path end-to-end in the demo (Workflows + Executions). Python
tasks have no trigger-rule gating, so this fans out + fans in (non-linear) but
does not skip — the skipped-node demos are the Rust fixtures.

    prepare ──┬─▶ transform ─▶ finish
              └─▶ validate ───┘
"""
from __future__ import annotations

import cloaca


# CLOACI-I-0128 / T-0760: declare typed workflow params (Python parity for
# Rust's #[workflow(params(...))]). The compiler parses this from source at build
# time into JSON-Schema-typed input slots; at runtime it's a no-op pass-through.
@cloaca.workflow_params(
    source_id=str,
    batch_size=(int, 500),
)
@cloaca.task(dependencies=[])
def prepare(context):
    """what: Stage the demo batch — seed the context the downstream fan-out reads.

    why: Every branch keys off the prepared flags; a bad seed fails the whole run,
    so preparation is its own observable step. (CLOACI-T-0754: demonstrates Python
    what/why docstrings surfacing in the UI like Rust doc comments.)
    """
    context.set("demo_py_prepare", True)
    return context


@cloaca.task(dependencies=["prepare"])
def transform(context):
    """what: Apply the demo transformation to the prepared batch.

    why: Runs in parallel with validate to demonstrate the fan-out shape.
    """
    context.set("demo_py_transform", True)
    return context


@cloaca.task(dependencies=["prepare"])
def validate(context):
    context.set("demo_py_validate", True)
    return context


@cloaca.task(dependencies=["transform", "validate"])
def finish(context):
    context.set("demo_py_workflow_ran", True)
    return context
