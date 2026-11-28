"""Helpers that more than one rules file uses. Not a chapter."""

from __future__ import annotations

import json

from .. import workload
from ..engine import Missing, parse_time

# The clock of the database, so times the server wrote are compared with a
# time from the same clock.
CLOCK_SQL = "SELECT now() AS now"

# The runs of the self-driving workflow of the last three hours, with the
# context each one left. The list of what a run did is in that context.
DRIVER_RUNS_SQL = (
    "SELECT we.id, we.status, we.started_at, we.completed_at, c.value AS context "
    "FROM {schema}.workflow_executions we "
    "LEFT JOIN {schema}.contexts c ON c.id = we.context_id "
    f"WHERE we.workflow_name = '{workload.DRIVER_WORKFLOW}' "
    "AND we.started_at > now() - interval '3 hours' "
    "ORDER BY we.started_at"
)

# How long after a publish its build must be over: the compiler's build time
# limit is 600 s.
SETTLE_SECONDS = 900


def need_clock(registry):
    registry.need_admin_db("clock", CLOCK_SQL)


def need_driver_runs(registry):
    registry.need_db("driver_runs", DRIVER_RUNS_SQL)


def clock(snapshot):
    """The time of the pass, by the database's clock when it answered."""
    try:
        rows = snapshot.db(None, "clock")
    except Missing:
        return snapshot.now
    stamp = parse_time(rows[0].get("now")) if rows else None
    return stamp if stamp is not None else snapshot.now


def workload_tenants(snapshot):
    """The tenants of the workload that this snapshot holds."""
    found = [tenant for tenant in workload.TENANTS if tenant in snapshot.tenants]
    if not found:
        raise Missing("no tenant of the workload is in the snapshot")
    return found


def _context(row):
    value = row.get("context")
    if isinstance(value, dict):
        return value
    if isinstance(value, str) and value:
        try:
            parsed = json.loads(value)
        except ValueError:
            return {}
        return parsed if isinstance(parsed, dict) else {}
    return {}


def driver_actions(snapshot, tenant):
    """Everything the self-driving workflow of a tenant did in the window,
    oldest first. Each record gets `at_epoch`."""
    actions = []
    for row in snapshot.db(tenant, "driver_runs"):
        for record in _context(row).get(workload.DRIVER_ACTIONS_KEY) or []:
            if not isinstance(record, dict):
                continue
            stamp = parse_time(record.get("at")) or parse_time(row.get("started_at"))
            if stamp is None:
                continue
            record = dict(record)
            record["at_epoch"] = stamp
            actions.append(record)
    actions.sort(key=lambda record: record["at_epoch"])
    return actions


def accepted(record):
    """True when the server accepted the call the record is about."""
    status = record.get("status")
    return isinstance(status, int) and 200 <= status < 300


def versions(snapshot, tenant, kind):
    """The versions list of a target package: version -> row. An unknown
    package (404) is an empty list."""
    status, body = snapshot.api(tenant, f"versions_target_{kind}")
    if status == 404:
        return {}
    if status != 200 or not isinstance(body, dict):
        raise Missing(f"the versions list of {workload.TARGETS[kind]} in {tenant} answered {status}")
    rows = {}
    # Newest upload first: keep the first row of a version.
    for row in body.get("items") or []:
        rows.setdefault(str(row.get("version")), row)
    return rows


def kind_of(package):
    for kind, name in workload.TARGETS.items():
        if name == package:
            return kind
    return None
