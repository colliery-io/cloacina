"""Python packaged POLL-TRIGGER workflow — the Python peer of the Rust
`packaged-triggers` example.

A `@cloaca.trigger(on=…, poll_interval=…)` polls on an interval and fires the
`file_processing_py` workflow automatically, injecting the discovered filename
via context — no `workflow run`. Tasks are declared with bare `@cloaca.task`
decorators (the packaged loader builds the workflow context from `workflow_name`
before importing this module, so they register on import — do NOT wrap them in a
`WorkflowBuilder`, that is for in-process runs only).

    validate -> transform -> archive

A real trigger would watch a directory / queue and `fire` only when a new file
arrives (returning `skip` otherwise). This demo fires every interval so
executions appear automatically — the point is to prove the packaged
Python-trigger path through the server gold path.
"""
from __future__ import annotations

import cloaca


@cloaca.task(id="validate", dependencies=[])
def validate(context):
    """what: Validate the incoming file named by the trigger.

    why: The trigger injects `filename`/`source_path`; validating first means a
    bad file fails here as its own observable step, not deep in transform.
    """
    filename = context.get("filename") or "unknown"
    context.set("validated", True)
    context.set("validated_file", filename)
    return context


@cloaca.task(id="transform", dependencies=["validate"])
def transform(context):
    """what: Transform the validated file. why: separated from validation so a
    transform bug is attributable and the step is independently retryable."""
    context.set("records_processed", 1500)
    return context


@cloaca.task(id="archive", dependencies=["transform"])
def archive(context):
    """what: Archive the processed file. why: a terminal step so a completed run
    is observably distinct from one that died mid-transform."""
    context.set("archived", True)
    return context


# Poll trigger: fires `file_processing_py` on a short interval, injecting the
# filename the workflow reads from context. `on` names the workflow in this same
# package; the decorator running at import IS the declaration (no triggers
# section in package.toml). The reconciler projects it into the host trigger
# registry, which drives the poll — executions appear automatically.
@cloaca.trigger(on="file_processing_py", poll_interval="3s")
def inbox_poll():
    ctx = cloaca.Context({
        "filename": "invoice-042.dat",
        "source_path": "/data/inbox/invoice-042.dat",
    })
    return cloaca.TriggerResult.fire(ctx)
