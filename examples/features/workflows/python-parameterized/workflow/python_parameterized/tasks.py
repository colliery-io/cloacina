"""Parameterized Python workflow — one template, many differently-bound runs.

`@cloaca.workflow_params(...)` declares the configurable surface (the Python
parity of Rust's `#[workflow(params(...))]`): a bare type is required, a
`(type, default)` tuple is optional with that default. The compiler parses this
into typed input slots; the server validates every run's `--context` values
against them and delivers the bound values as top-level context keys.

    plan_sync -> execute_sync -> report
"""
from __future__ import annotations

import cloaca


@cloaca.workflow_params(
    source=str,           # required
    dst=str,              # required
    mode=(str, "copy"),   # optional, default "copy"
    max_files=(int, 100), # optional, default 100
)
@cloaca.task(id="plan_sync", dependencies=[])
def plan_sync(context):
    """what: Turn the bound params into a sync plan.

    why: Bound values arrive as top-level context keys; making the plan explicit
    means a bad parameterization is visible here, not deep in the transfer.
    """
    source = context.get("source")
    dst = context.get("dst")
    mode = context.get("mode") or "copy"
    max_files = context.get("max_files") or 100
    if mode not in ("copy", "move"):
        raise ValueError(f"mode must be 'copy' or 'move', got {mode!r}")
    context.set("sync_plan", {"source": source, "dst": dst, "mode": mode, "max_files": max_files})
    return context


@cloaca.task(id="execute_sync", dependencies=["plan_sync"])
def execute_sync(context):
    """what: Simulate the transfer the plan describes."""
    plan = context.get("sync_plan") or {}
    transferred = min(int(plan.get("max_files", 0)), 42)
    context.set("sync_result", {"transferred": transferred, "plan": plan})
    return context


@cloaca.task(id="report", dependencies=["execute_sync"])
def report(context):
    """what: Report what this particular parameterization did."""
    context.set("sync_report", context.get("sync_result"))
    return context
