"""Python packaged CONDITIONAL-SKIP workflow — proves `trigger_rules` gate a
Python task into the real `Skipped` state on the server gold path.

    gate ──┬─▶ process ─▶ record
           └─▶ audit (SKIPPED)

`gate` sets `do_audit = False`. `audit` is gated by
`context_value("do_audit", "Equals", True)`, so its rule never fires and the
planner must Skip it. `audit` RAISES if it is ever executed — so the ONLY way
the run reaches `Completed` is if the skip was honored (if `trigger_rules` were
ignored, `audit` would run, raise, and fail the execution). `record` fans in on
`process` + `audit`; a Skipped dependency counts as resolved, so `record` still
runs — and it asserts `process` actually ran, catching the opposite bug
(everything wrongly skipped).

This is the Python mirror of the Rust trigger-rule gating in `demo-cron-rust`,
lifted onto the features gold path.
"""
from __future__ import annotations

import cloaca


@cloaca.task(id="gate", dependencies=[])
def gate(context):
    """what: Decide the audit branch is OFF for this run.

    why: Gating on a context value the trigger rule reads is how cloacina models
    conditional branches — no imperative if/else in the DAG.
    """
    context.set("do_audit", False)
    context.set("gated", True)
    return context


@cloaca.task(id="process", dependencies=["gate"])
def process(context):
    """what: The always-on work path. why: gives `record` something to assert so
    a run that wrongly skips everything is caught, not just one that under-skips."""
    context.set("records", 10)
    return context


@cloaca.task(
    id="audit",
    dependencies=["gate"],
    # Rule wants do_audit == True, which never holds → this task must Skip.
    trigger_rules=cloaca.context_value("do_audit", "Equals", True),
)
def audit(context):
    """what: The gated-off branch. why: it RAISES if executed, so a completed run
    is proof the trigger rule skipped it rather than running it."""
    raise RuntimeError(
        "audit ran even though its trigger_rules gate (do_audit == True) is False "
        "— trigger_rules skipping was NOT honored"
    )


@cloaca.task(id="record", dependencies=["process", "audit"])
def record(context):
    """what: Fan-in terminal step over the run path + the skipped path.

    why: proves a Skipped dependency counts as resolved (the fan-in still fires)
    AND that the run path executed (asserts `process`'s output is present).
    """
    records = context.get("records")
    if records != 10:
        raise RuntimeError(
            f"expected process to have run (records == 10), got {records!r} "
            "— the run path was wrongly skipped"
        )
    context.set("recorded", True)
    return context
