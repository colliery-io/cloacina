"""A job of the behaviour suite's standing workload (CLOACINA-T-1107) that
holds an agent and does nothing else.

The self-driving workflow starts more of these at once than its tenant's
agents can take, so later work of the tenant has to wait. It sleeps: it fills
a slot of an agent, not a CPU. `hold_seconds` in the context sets the hold.
"""
from __future__ import annotations

import json
import os
import socket
import time

import cloaca

WORKFLOW = "standing_hold"
RESULT_KEY = "standing_hold_done"
HOLD_SECONDS = 120


def _mark(context, task):
    """One marked line for each start of a task body. `schedule_id` and
    `scheduled_time` name a scheduled run, `run_once_tag` a run by hand."""
    line = "STANDING_MARK " + json.dumps({
        "workflow": WORKFLOW, "task": task, "scheduled_time": context.get("scheduled_time", None),
        "schedule_id": context.get("schedule_id", None),
        "run_once_tag": context.get("run_once_tag", None),
        "host": socket.gethostname(), "pid": os.getpid(), "at": time.time()}, sort_keys=True)
    print(line, flush=True)


@cloaca.task(dependencies=[], retry_attempts=0, timeout_seconds=900)
def hold(context):
    _mark(context, "hold")
    seconds = context.get("hold_seconds", None)
    seconds = float(HOLD_SECONDS if seconds is None else seconds)
    started = time.time()
    while time.time() - started < seconds:
        time.sleep(min(5.0, max(0.0, seconds - (time.time() - started))))
    context.set(RESULT_KEY, {
        "held_seconds": round(time.time() - started, 1),
        "host": socket.gethostname(),
    })
    return context
