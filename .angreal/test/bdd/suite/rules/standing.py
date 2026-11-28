"""Rules of the standing workload itself (CLOACINA-T-1107, CLOACINA-T-1108).

These are not promises of the product. They say whether the workload is
alive: the agents, the schedules, the self-driving workflow. When one of them
does not hold, the other rows of the report have little work to judge.
"""

from __future__ import annotations

from .. import workload
from ..engine import broken, format_time, holds, parse_time, unknown
from . import _shared

CHAPTER = "standing"

# Agents that sent a heartbeat in the last two minutes, for each tenant. A
# global table, in the schema `cloacina_global` (CLOACINA-T-1080).
AGENTS_SQL = (
    "SELECT tenant_id, count(*) AS agents, max(last_heartbeat_at) AS last_heartbeat "
    "FROM cloacina_global.fleet_agents "
    "WHERE last_heartbeat_at > now() - interval '2 minutes' "
    "GROUP BY tenant_id"
)

# For each workflow of a tenant: its runs of the last three hours.
LAST_RUNS_SQL = (
    "SELECT workflow_name, count(*) AS runs, max(started_at) AS last_started "
    "FROM {schema}.workflow_executions "
    "WHERE started_at > now() - interval '3 hours' "
    "GROUP BY workflow_name"
)

# The first run a tenant ever had: one row, also for a tenant with no run.
FIRST_RUN_SQL = (
    "SELECT min(started_at) AS first_run, count(*) AS runs "
    "FROM {schema}.workflow_executions"
)

# A run may start this long after its fire time before the rule calls the
# schedule late. The "Schedules" chapter judges lateness itself; this rule
# only asks whether the schedule is alive.
SLACK_SECONDS = 120


def _first_run(snapshot, tenant):
    """When the tenant's first run started, or None when it has none."""
    rows = snapshot.db(tenant, "first_run")
    return parse_time(rows[0].get("first_run")) if rows else None


def register(registry):
    _shared.need_clock(registry)
    _shared.need_driver_runs(registry)
    registry.need_admin_db("agents", AGENTS_SQL)
    registry.need_db("last_runs", LAST_RUNS_SQL)
    registry.need_db("first_run", FIRST_RUN_SQL)

    @registry.rule("standing.agents", "workload.agents", CHAPTER,
                   tolerates=("kill_agent", "postgres_away", "restart_server"),
                   about="each tenant has its agents, with a heartbeat in the last two minutes")
    def agents(snapshot):
        rows = {row.get("tenant_id"): row for row in snapshot.db(None, "agents")}
        short = []
        seen = []
        for tenant in _shared.workload_tenants(snapshot):
            count = int((rows.get(tenant) or {}).get("agents") or 0)
            seen.append(f"{tenant} {count}")
            if count < workload.AGENTS_PER_TENANT:
                short.append(f"{tenant} has {count} of {workload.AGENTS_PER_TENANT} agents")
        if short:
            return broken("; ".join(short), short)
        return holds("agents with a recent heartbeat: " + ", ".join(seen))

    @registry.rule("standing.schedules", "workload.schedules", CHAPTER,
                   tolerates=("postgres_away", "restart_server"),
                   about="each schedule of the workload started a run within two of its periods")
    def schedules(snapshot):
        now = _shared.clock(snapshot)
        late = []
        judged = 0
        tenants = _shared.workload_tenants(snapshot)
        firsts = {tenant: _first_run(snapshot, tenant) for tenant in tenants}
        known = [stamp for stamp in firsts.values() if stamp is not None]
        oldest = min(known) if known else None
        for tenant in tenants:
            rows = {row.get("workflow_name"): row for row in snapshot.db(tenant, "last_runs")}
            first = firsts[tenant]
            if first is None:
                # No run at all. That is a fault once another tenant of the
                # same deployment has been running for a while.
                if oldest is not None and now - oldest > 2 * workload.SLOT_SECONDS + SLACK_SECONDS:
                    judged += 1
                    late.append(f"{tenant}: no run at all, while other tenants run")
                continue
            for workflow, period in sorted(workload.scheduled_workflows().items()):
                limit = 2 * period + SLACK_SECONDS
                last = parse_time((rows.get(workflow) or {}).get("last_started"))
                if last is None and now - first < limit:
                    continue  # the tenant is younger than the limit: not due yet
                judged += 1
                if last is None:
                    late.append(f"{tenant}/{workflow}: no run in the last three hours")
                elif now - last > limit:
                    late.append(f"{tenant}/{workflow}: last run started {format_time(last)}, "
                                f"{int(now - last)} s ago (limit {limit} s)")
        if late:
            return broken(f"{len(late)} of {judged} schedules have no recent run", late)
        if not judged:
            return unknown("no tenant of the workload has a run yet")
        return holds(f"{judged} schedules each started a run within two of their periods")

    @registry.rule("standing.self_driving", "workload.self_driving", CHAPTER,
                   tolerates=("postgres_away", "restart_server", "kill_agent"),
                   about="the self-driving workflow of each tenant finished a run with actions "
                         "in the last two periods")
    def self_driving(snapshot):
        now = _shared.clock(snapshot)
        limit = 2 * workload.SLOT_SECONDS + SLACK_SECONDS
        quiet = []
        seen = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            runs = snapshot.db(tenant, "driver_runs")
            if not runs:
                first = _first_run(snapshot, tenant)
                if first is None or now - first < limit:
                    continue  # too young to have a finished run
                judged += 1
                quiet.append(f"{tenant}: no run of the self-driving workflow in three hours")
                continue
            judged += 1
            actions = _shared.driver_actions(snapshot, tenant)
            recent = [record for record in actions if now - record["at_epoch"] <= limit]
            faults = [record for record in recent
                      if record.get("action") in ("read_key", "configure") and record.get("error")]
            if faults:
                quiet.append(f"{tenant}: the self-driving workflow cannot act: "
                             f"{faults[-1].get('action')} {faults[-1].get('error')}")
            elif not recent:
                last = actions[-1]["at_epoch"] if actions else None
                quiet.append(f"{tenant}: no action since {format_time(last) or 'ever'}")
            else:
                seen.append(f"{tenant} {len(recent)}")
        if quiet:
            return broken("; ".join(quiet), quiet)
        if not judged:
            return unknown("no tenant has a run of the self-driving workflow yet")
        return holds("actions in the last two periods: " + ", ".join(seen))
