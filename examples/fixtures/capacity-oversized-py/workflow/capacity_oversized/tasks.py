"""A job of the behaviour suite's "Capacity" chapter (CLOACINA-T-1114).

One task of the agent pool `sized` that asks for 2Gi of memory: more than the
chapter's agent of that pool reports. It runs only on an agent that reports
room for 2Gi. The request is what the server packs by; the task itself only
writes the host it ran on.
"""
from __future__ import annotations

import socket

import cloaca

WORKFLOW = "capacity_oversized"
RESULT_KEY = "capacity_oversized_done"


@cloaca.task(dependencies=[], retry_attempts=0, timeout_seconds=600,
             pool="sized", cpu="250m", memory="2Gi")
def oversized(context):
    context.set(RESULT_KEY, {"host": socket.gethostname()})
    return context
