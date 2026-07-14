"""Canonical Python packaged workflow — a three-task data pipeline.

The Python peer of the Rust `simple-packaged` example. Tasks are declared with
bare `@cloaca.task` decorators; the packaged loader builds the workflow context
from `workflow_name` (in package.toml) before importing this module, so tasks
register on import — do NOT wrap them in a `WorkflowBuilder` (that is for
in-process runs only).

    collect_data -> process_data -> generate_report
"""
from __future__ import annotations

import cloaca


@cloaca.task(id="collect_data", dependencies=[])
def collect_data(context):
    """what: Gather the input batch and stage it on the context.

    why: Every downstream task keys off this data; making collection its own
    observable step means a bad source fails here, not deep in processing.
    """
    context.set("raw_records", 1000)
    context.set("source", "demo_database")
    return context


@cloaca.task(id="process_data", dependencies=["collect_data"])
def process_data(context):
    """what: Validate and transform the collected records.

    why: Separated from collection so a transform bug is attributable and the
    step is independently retryable.
    """
    raw = context.get("raw_records") or 0
    context.set("processed_records", raw)
    context.set("valid", raw > 0)
    return context


@cloaca.task(id="generate_report", dependencies=["process_data"])
def generate_report(context):
    """what: Summarize the processed batch into a report.

    why: The terminal step downstream consumers read — keeps reporting distinct
    from the transformation that produced the numbers.
    """
    context.set(
        "report",
        {
            "records": context.get("processed_records"),
            "source": context.get("source"),
            "ok": bool(context.get("valid")),
        },
    )
    return context
