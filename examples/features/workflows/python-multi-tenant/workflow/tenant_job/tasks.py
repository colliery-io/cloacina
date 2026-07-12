"""Python packaged MULTI-TENANCY workflow — a trivial job used to demonstrate
the tenant isolation boundary.

The feature under test is not the task logic but the tenancy boundary: the same
package is deployed into two tenants and run in both, and the harness asserts
their executions never cross (and that a tenant without the package can't see
the workflow). See the package.toml and the `python-multi-tenant` lane.

    stamp -> finish
"""
from __future__ import annotations

import cloaca


@cloaca.task(id="stamp", dependencies=[])
def stamp(context):
    """what: Record that the job ran. why: a minimal observable side effect so a
    per-tenant execution reaches Completed and is countable in that tenant."""
    context.set("stamped", True)
    return context


@cloaca.task(id="finish", dependencies=["stamp"])
def finish(context):
    """what: Terminal step. why: a two-task DAG keeps the example a real workflow
    rather than a single node, matching the other packaged peers."""
    context.set("finished", True)
    return context
