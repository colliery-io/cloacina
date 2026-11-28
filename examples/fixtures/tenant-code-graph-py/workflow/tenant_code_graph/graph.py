"""A graph whose node says where it ran (CLOACINA-T-1079, CLOACINA-A-0019).

The reactor `tenant_code_graph_rx` fires the graph. Its one node, `record`,
returns the facts of the process that ran it and writes them as a marked line
to stderr. The chapter reads the node's output from the reactor's recent
fires, and looks for the marked line in the server's log, where it must not
be. Names only: the program, its host name, and which credential names its
environment holds. Never a value.
"""
from __future__ import annotations

import json
import os
import socket
import sys

import cloaca

NODE_MARKER = "TENANT_CODE_GRAPH_MARK"

ORCHESTRATOR_CREDENTIALS = (
    "DATABASE_URL",
    "CLOACINA_BOOTSTRAP_KEY",
    "CLOACINA_SECRET_KEK",
    "PGPASSWORD",
)


def _where():
    try:
        with open("/proc/self/cmdline", "rb") as handle:
            process = os.path.basename(handle.read().split(b"\0")[0].decode("utf-8", "replace"))
    except Exception:  # noqa: BLE001
        process = os.path.basename(sys.argv[0]) if sys.argv and sys.argv[0] else "unknown"
    return {
        "moment": "graph",
        "process": process,
        "pid": os.getpid(),
        "hostname": socket.gethostname(),
        "orchestrator_credentials_in_environment": [
            name for name in ORCHESTRATOR_CREDENTIALS if os.environ.get(name)
        ],
    }


@cloaca.passthrough_accumulator
def tc_in(event):
    return event


@cloaca.reactor(name="tenant_code_graph_rx", accumulators=["tc_in"], mode="when_any")
class _Reactor:
    pass


with cloaca.ComputationGraphBuilder(
    "tenant_code_graph",
    reactor=_Reactor,
    graph={"record": {"inputs": ["tc_in"]}},
) as builder:

    @cloaca.node
    def record(tc_in):
        here = _where()
        try:
            print(f"{NODE_MARKER} {json.dumps(here, sort_keys=True)}", file=sys.stderr, flush=True)
        except Exception:  # noqa: BLE001
            pass
        return {"where": here}
