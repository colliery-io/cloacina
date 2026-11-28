"""A graph fed by a polling accumulator (CLOACINA-T-1079, CLOACINA-A-0019).

The poll function `tc_poll` is tenant code that the reactive layer would call
on a timer. A server without single trust refuses the package, and the
function must never run in the server: it writes a marked line to stderr each
time it runs, with names only (the program and which credential names its
environment holds).
"""
from __future__ import annotations

import json
import os
import sys

import cloaca

POLL_MARKER = "TENANT_CODE_ACCUMULATOR_MARK"

ORCHESTRATOR_CREDENTIALS = (
    "DATABASE_URL",
    "CLOACINA_BOOTSTRAP_KEY",
    "CLOACINA_SECRET_KEK",
    "PGPASSWORD",
)


@cloaca.polling_accumulator(interval="5s")
def tc_poll():
    here = {
        "moment": "accumulator",
        "process": os.path.basename(sys.argv[0]) if sys.argv and sys.argv[0] else "unknown",
        "orchestrator_credentials_in_environment": [
            name for name in ORCHESTRATOR_CREDENTIALS if os.environ.get(name)
        ],
    }
    try:
        print(f"{POLL_MARKER} {json.dumps(here, sort_keys=True)}", file=sys.stderr, flush=True)
    except Exception:  # noqa: BLE001
        pass
    return {"polled": True}


@cloaca.reactor(name="tenant_code_polling_rx", accumulators=["tc_poll"], mode="when_any")
class _Reactor:
    pass


with cloaca.ComputationGraphBuilder(
    "tenant_code_polling",
    reactor=_Reactor,
    graph={"consume": {"inputs": ["tc_poll"]}},
) as builder:

    @cloaca.node
    def consume(tc_poll):
        return {"seen": tc_poll is not None}
