"""Team B's "ledger job" for the publishing behaviour spec (CLOACINA-T-1026)."""
from __future__ import annotations

import cloaca


@cloaca.task(dependencies=[])
def close_ledger(context):
    context.set("ledger_closed", True)
    return context
