"""A package whose import leaves a mark (CLOACINA-T-1079, CLOACINA-A-0019).

The top level of this module runs in every process that imports it. It writes
one marked line to stderr and keeps the same facts in `IMPORT_FACTS`. The task
copies those facts into the run's context, so the chapter can see the mark in
the process that ran the task (the control), and look for it in the logs of
the server and the compiler (where it must not be).

The facts are names only: the process, its host name, and which credential
names its environment holds. Never a value.
"""
from __future__ import annotations

import json
import os
import socket
import sys

import cloaca

IMPORT_MARKER = "TENANT_CODE_IMPORT_MARK"
TASK_MARKER = "TENANT_CODE_TASK_MARK"

# Names that give their holder the orchestrator's own reach.
ORCHESTRATOR_CREDENTIALS = (
    "DATABASE_URL",
    "CLOACINA_BOOTSTRAP_KEY",
    "CLOACINA_SECRET_KEK",
    "PGPASSWORD",
)


def _process_name():
    try:
        with open("/proc/self/cmdline", "rb") as handle:
            parts = handle.read().split(b"\0")
        return os.path.basename(parts[0].decode("utf-8", "replace")), (
            len(parts) > 1 and parts[1] == b"describe")
    except Exception:  # noqa: BLE001
        name = os.path.basename(sys.argv[0]) if sys.argv and sys.argv[0] else "unknown"
        return name, False


def where(moment):
    """Which process runs this code, and which credential names it holds."""
    process, describe_mode = _process_name()
    return {
        "moment": moment,
        "process": process,
        "pid": os.getpid(),
        "describe_mode": describe_mode,
        "hostname": socket.gethostname(),
        "orchestrator_credentials_in_environment": [
            name for name in ORCHESTRATOR_CREDENTIALS if os.environ.get(name)
        ],
    }


def _emit(marker, facts):
    try:
        print(f"{marker} {json.dumps(facts, sort_keys=True)}", file=sys.stderr, flush=True)
    except Exception:  # noqa: BLE001 - the mark must never break the import
        pass


try:
    IMPORT_FACTS = where("import")
    _emit(IMPORT_MARKER, IMPORT_FACTS)
except Exception as exc:  # noqa: BLE001
    IMPORT_FACTS = {"moment": "import", "error": exc.__class__.__name__}


@cloaca.task(dependencies=[])
def record_where(context):
    here = where("task")
    _emit(TASK_MARKER, here)
    context.set("task_mark", here)
    context.set("import_mark_in_this_process", IMPORT_FACTS)
    return context
