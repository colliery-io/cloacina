"""Team A's "orders job" for the publishing behaviour spec (CLOACINA-T-1026).

Two tasks, one after the other. Each one writes the version of the code that
ran it into the context. The first one can hold the execution open for
`hold_seconds` (from the execution context), so the spec can publish a new
version while the execution runs and then see which version ran the second
task.
"""
from __future__ import annotations

import time

import cloaca

# The spec driver replaces this string when it stages version 0.2.0.
PACKAGE_VERSION = "0.1.0"


@cloaca.task(dependencies=[])
def take_orders(context):
    context.set("first_task_version", PACKAGE_VERSION)
    hold = context.get("hold_seconds", 0) or 0
    if hold:
        time.sleep(float(hold))
    return context


@cloaca.task(dependencies=["take_orders"])
def ship_orders(context):
    context.set("second_task_version", PACKAGE_VERSION)
    return context
