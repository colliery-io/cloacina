"""A job of the behaviour suite's "Capacity" chapter (CLOACINA-T-1114).

One task that names the agent pool `sized` and asks for 250m of CPU and
400Mi of memory. It sleeps for `hold_seconds` (default 30) and writes the
host it ran on, so the chapter can say which agent ran it. It fills a slot
and a share of the agent's memory by request, not by use.
"""
from __future__ import annotations

import socket
import time

import cloaca

WORKFLOW = "capacity_sized"
RESULT_KEY = "capacity_sized_done"
HOLD_SECONDS = 30


@cloaca.task(dependencies=[], retry_attempts=0, timeout_seconds=600,
             pool="sized", cpu="250m", memory="400Mi")
def sized(context):
    seconds = context.get("hold_seconds", None)
    seconds = float(HOLD_SECONDS if seconds is None else seconds)
    started = time.time()
    while time.time() - started < seconds:
        time.sleep(min(2.0, max(0.0, seconds - (time.time() - started))))
    context.set(RESULT_KEY, {"host": socket.gethostname(), "started": started,
                             "ended": time.time()})
    return context
