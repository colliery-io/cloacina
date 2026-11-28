"""Rules of the Operations row (CLOACINA-T-1115).

What the people who run the system can see: an alert for each run that
failed for good, who did what in the audit record, the backlog and the lag
as gauges, and a history that retention keeps short.

The rules compare two accounts of the same thing:

- the alerts the suite's receiver got (`suite/alerts.py`) against the runs
  the database says failed, or failed once and then passed;
- the self-driving workflow's own record of what it did against the rows of
  `audit_events`;
- the server's gauges (its metrics listener) against the same figure read
  from the database at the same pass, so a gauge that stays at 0 while work
  waits is a fault, and not a pass;
- the age of the finished runs against the retention period.

A receiver or a metrics listener that is away is "cannot judge".
"""

from __future__ import annotations

import json

from .. import workload
from ..engine import broken, format_time, holds, parse_time, unknown
from . import _shared

CHAPTER = "operations"

# An alert leaves a few seconds after its run ends; a send that failed is
# tried again after 30 s. Judge a run this long after it ended.
ALERT_SETTLE_SECONDS = 180
FAILED_EVENT = "workflow_execution.failed"
# An audit row is written when the request is served; the driver writes its
# record a moment later. Allow this much between the two clocks.
AUDIT_SLACK_SECONDS = 600
# Judge a driver action this long after it.
AUDIT_SETTLE_SECONDS = 120
# The name of the self-driving workflow's key (`StandingWorld.ensure_driver_secret`).
DRIVER_KEY_NAME = "{tenant}-self-driving"
# The keys the driver makes for a tenant, by the end of their name
# (`PublishWorld.provision_tenants`, `StandingWorld`).
TENANT_KEY_SUFFIXES = ("-self-driving", "-publish", "-admin", "-checker")
# The retention job runs every 300 s; give it three passes.
RETENTION_SLACK_SECONDS = 900
RETENTION_SECONDS = workload.RUN_HISTORY_DAYS * 86400
FINISHED = ("Completed", "Failed", "Cancelled")
WAITING_GAUGE = "cloacina_waiting_tasks"
LAG_GAUGE = "cloacina_schedule_lag_seconds"

# The runs that ended Failed in the last three hours.
FAILED_RUNS_SQL = (
    "SELECT id, workflow_name, completed_at FROM {schema}.workflow_executions "
    "WHERE status = 'Failed' AND completed_at > now() - interval '3 hours'"
)

# The runs that ended Completed in the last three hours after an attempt failed.
RETRIED_RUNS_SQL = (
    "SELECT we.id, we.workflow_name, we.completed_at FROM {schema}.workflow_executions we "
    "WHERE we.status = 'Completed' AND we.completed_at > now() - interval '3 hours' "
    "AND EXISTS (SELECT 1 FROM {schema}.task_attempt_errors e "
    "WHERE e.workflow_execution_id = we.id)"
)

# The tasks that wait now, as the waiting gauge counts them
# (`TaskExecutionDAL::waiting_task_stats_by_pool_and_priority`).
WAITING_SQL = (
    "SELECT count(*) AS waiting FROM {schema}.task_executions "
    "WHERE status = 'Ready' AND (waiting_since <= now() "
    "OR (waiting_since IS NULL AND (retry_at IS NULL OR retry_at <= now())))"
)

# How late the most overdue cron schedule is now, as the lag gauge says it.
LAG_SQL = (
    "SELECT extract(epoch FROM now() - min(next_run_at)) AS lag_seconds, count(*) AS due "
    "FROM {schema}.schedules "
    "WHERE schedule_type = 'cron' AND enabled AND NOT paused AND next_run_at < now()"
)

# The retention facts of a tenant, in one row. The tenant's age is the age of
# its oldest schedule: its first runs are gone once retention works.
RETENTION_SQL = (
    "SELECT "
    "(SELECT count(*) FROM {schema}.workflow_executions "
    f"WHERE status IN ('Completed', 'Failed', 'Cancelled') AND completed_at < now() "
    f"- interval '{RETENTION_SECONDS + RETENTION_SLACK_SECONDS} seconds') AS old_finished, "
    "(SELECT min(started_at) FROM {schema}.workflow_executions "
    "WHERE status NOT IN ('Completed', 'Failed', 'Cancelled')) AS oldest_active, "
    "(SELECT count(*) FROM {schema}.task_executions te WHERE NOT EXISTS "
    "(SELECT 1 FROM {schema}.workflow_executions we WHERE we.id = te.workflow_execution_id)) "
    "AS orphan_tasks, "
    "(SELECT min(created_at) FROM {schema}.schedules) AS tenant_since"
)

# The audit record of the last three hours. A global table, in the schema
# `cloacina_global` (CLOACINA-T-1080).
AUDIT_SQL = (
    "SELECT id, created_at, tenant_id, actor_key_id, actor_key_name, action, target, "
    "outcome, detail FROM cloacina_global.audit_events "
    "WHERE created_at > now() - interval '3 hours' ORDER BY id"
)

# Which audit action a driver action writes.
AUDIT_ACTION = {
    "publish": "package.upload",
    "publish_broken": "package.upload",
    "step_back": "package.activate",
    "run_by_hand": "workflow.run",
    "start_holds": "workflow.run",
}


def _int(value):
    try:
        return int(float(value))
    except (TypeError, ValueError):
        return None


def detail_of(row):
    value = row.get("detail")
    if isinstance(value, str):
        try:
            value = json.loads(value)
        except ValueError:
            return {}
    return value if isinstance(value, dict) else {}


def alerts_by_run(records):
    """The alerts received, by execution id."""
    found = {}
    for record in records:
        body = record.get("body") if isinstance(record.get("body"), dict) else {}
        run = body.get("execution_id")
        if run:
            found.setdefault(str(run), []).append(body)
    return found


def audit_row_for(rows, tenant, record):
    """The audit rows that a driver action wrote: same tenant, action and
    target, close in time."""
    action = AUDIT_ACTION.get(record.get("action"))
    at = record.get("at_epoch")
    found = []
    for row in rows:
        if row.get("tenant_id") != tenant or row.get("action") != action:
            continue
        stamp = parse_time(row.get("created_at"))
        if stamp is None or abs(stamp - at) > AUDIT_SLACK_SECONDS:
            continue
        detail = detail_of(row)
        kind = record.get("action")
        if kind in ("publish", "publish_broken"):
            ok = (detail.get("package_name") == record.get("package")
                  and str(detail.get("version")) == str(record.get("version")))
        elif kind == "step_back":
            ok = (row.get("target") == record.get("package")
                  and str(detail.get("version")) == str(record.get("version")))
        elif kind == "run_by_hand":
            ok = str(detail.get("execution_id")) == str(record.get("execution_id"))
        else:  # start_holds
            ok = row.get("target") == workload.HOLD_WORKFLOW
        if ok:
            found.append(row)
    return found


def gauge_by_tenant(series):
    """The sum of a gauge's series for each tenant label."""
    sums = {}
    for labels, value in series:
        tenant = labels.get("tenant")
        if tenant:
            sums[tenant] = sums.get(tenant, 0.0) + value
    return sums


def register(registry):
    _shared.need_clock(registry)
    _shared.need_driver_runs(registry)
    registry.need_db("operations_failed_runs", FAILED_RUNS_SQL)
    registry.need_db("operations_retried_runs", RETRIED_RUNS_SQL)
    registry.need_db("operations_waiting", WAITING_SQL)
    registry.need_db("operations_lag", LAG_SQL)
    registry.need_db("operations_retention", RETENTION_SQL)
    registry.need_admin_db("operations_audit", AUDIT_SQL)

    @registry.rule("operations.alert_per_failed_run", "operations.alert", CHAPTER,
                   tolerates=("postgres_away", "restart_server"),
                   about="each run that failed for good while the receiver was up has an alert "
                         "with its tenant, its workflow and its id")
    def alert_per_failed_run(snapshot):
        now = _shared.clock(snapshot)
        body = snapshot.alerts()
        up = parse_time(body.get("started_at")) or now
        by_run = alerts_by_run(body.get("records") or [])
        wrong = []
        judged = 0
        repeats = 0
        for tenant in _shared.workload_tenants(snapshot):
            for row in snapshot.db(tenant, "operations_failed_runs"):
                ended = parse_time(row.get("completed_at"))
                if ended is None or ended < up or now - ended < ALERT_SETTLE_SECONDS:
                    continue
                judged += 1
                run = str(row.get("id"))
                name = f"{tenant}/{row.get('workflow_name')} run {run}"
                got = by_run.get(run) or []
                if not got:
                    wrong.append(f"{name}, failed at {format_time(ended)}: no alert")
                    continue
                repeats += len(got) - 1
                alert = got[0]
                for field, wanted in (("tenant", tenant), ("workflow", row.get("workflow_name")),
                                      ("event", FAILED_EVENT)):
                    if alert.get(field) != wanted:
                        wrong.append(f"{name}: the alert's {field} is {alert.get(field)!r}, "
                                     f"not {wanted!r}")
        if wrong:
            return broken(f"{len(wrong)} faults in the alerts of {judged} failed runs", wrong)
        if not judged:
            return unknown("no run failed while the receiver was up, long enough ago to judge")
        extra = f"; {repeats} alerts came twice (delivery is at least once)" if repeats else ""
        return holds(f"{judged} failed runs, each with its alert{extra}")

    @registry.rule("operations.no_alert_after_retry", "operations.alert", CHAPTER,
                   about="a run that failed an attempt and then completed raised no alert")
    def no_alert_after_retry(snapshot):
        now = _shared.clock(snapshot)
        body = snapshot.alerts()
        up = parse_time(body.get("started_at")) or now
        by_run = alerts_by_run(body.get("records") or [])
        wrong = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            for row in snapshot.db(tenant, "operations_retried_runs"):
                ended = parse_time(row.get("completed_at"))
                if ended is None or ended < up or now - ended < ALERT_SETTLE_SECONDS:
                    continue
                judged += 1
                if by_run.get(str(row.get("id"))):
                    wrong.append(f"{tenant}/{row.get('workflow_name')} run {row.get('id')} "
                                 "completed after a retry and raised an alert")
        if wrong:
            return broken(f"{len(wrong)} alerts for runs that completed", wrong)
        if not judged:
            return unknown("no run completed after a failed attempt while the receiver was up")
        return holds(f"{judged} runs completed after a failed attempt; none raised an alert")

    @registry.rule("operations.audit_of_driver", "operations.audit", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each publish, version change and run by hand of the self-driving "
                         "workflow has its audit row, written by its own key in its tenant")
    def audit_of_driver(snapshot):
        now = _shared.clock(snapshot)
        rows = snapshot.db(None, "operations_audit")
        wrong = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            key_name = DRIVER_KEY_NAME.format(tenant=tenant)
            for record in _shared.driver_actions(snapshot, tenant):
                if record.get("action") not in AUDIT_ACTION:
                    continue
                # A burst records the status of each start in `statuses`.
                if record.get("action") != "start_holds" and not _shared.accepted(record):
                    continue
                age = now - record["at_epoch"]
                if age < AUDIT_SETTLE_SECONDS or age > 3 * 3600 - AUDIT_SLACK_SECONDS:
                    continue
                name = f"{tenant}: {record.get('action')} at {format_time(record['at_epoch'])}"
                found = audit_row_for(rows, tenant, record)
                if record.get("action") == "start_holds":
                    started = sum(1 for status in record.get("statuses") or []
                                  if isinstance(status, int) and 200 <= status < 300)
                    if not started:
                        continue
                    judged += 1
                    mine = [row for row in found if row.get("actor_key_name") == key_name]
                    if len(mine) < started:
                        wrong.append(f"{name}: {started} runs started, {len(mine)} audit rows "
                                     f"by {key_name}")
                    continue
                judged += 1
                if not found:
                    wrong.append(f"{name}: no {AUDIT_ACTION[record['action']]} row")
                elif not any(row.get("actor_key_name") == key_name and row.get("actor_key_id")
                             for row in found):
                    wrong.append(f"{name}: the row names the key {found[0].get('actor_key_name')!r}, "
                                 f"not {key_name!r}")
        if wrong:
            return broken(f"{len(wrong)} of {judged} actions of the self-driving workflow have no "
                          "right audit row", wrong)
        if not judged:
            return unknown("no action of the self-driving workflow is old enough to judge")
        return holds(f"{judged} actions of the self-driving workflow, each with its audit row")

    @registry.rule("operations.audit_rows_named", "operations.audit", CHAPTER,
                   about="each audit row of a tenant names the key that acted and the tenant; "
                         "no key of one tenant did anything in another")
    def audit_rows_named(snapshot):
        rows = snapshot.db(None, "operations_audit")
        tenants = _shared.workload_tenants(snapshot)
        wrong = []
        judged = 0
        for row in rows:
            key_name = str(row.get("actor_key_name") or "")
            owner = next((t for t in tenants for suffix in TENANT_KEY_SUFFIXES
                          if key_name == f"{t}{suffix}"), None)
            if row.get("tenant_id") not in tenants and owner is None:
                continue
            judged += 1
            name = f"audit row {row.get('id')} ({row.get('action')}, {row.get('outcome')})"
            if not row.get("actor_key_id") or not key_name:
                wrong.append(f"{name} names no key")
            if owner is not None and row.get("outcome") == "ok" and row.get("tenant_id") != owner:
                wrong.append(f"{name}: the key {key_name} of {owner} acted in "
                             f"{row.get('tenant_id') or 'no tenant'}")
        if wrong:
            return broken(f"{len(wrong)} faults in {judged} audit rows", wrong)
        if not judged:
            return unknown("no audit row of the workload's tenants in the window")
        return holds(f"{judged} audit rows of the workload's tenants, each with its key and tenant")

    @registry.rule("operations.backlog_gauge", "operations.backlog", CHAPTER,
                   tolerates=("postgres_away", "restart_server"),
                   about="the waiting gauge of each tenant shows its waiting tasks while they wait, "
                         "and goes back to 0 when none waits")
    def backlog_gauge(snapshot):
        sums = gauge_by_tenant(snapshot.metrics(WAITING_GAUGE))
        wrong = []
        moved = []
        seen = []
        for tenant in _shared.workload_tenants(snapshot):
            rows = snapshot.db(tenant, "operations_waiting")
            waiting = _int(rows[0].get("waiting")) if rows else None
            gauge = sums.get(tenant)
            if waiting is None or gauge is None:
                continue
            seen.append(f"{tenant} {waiting}/{int(gauge)}")
            if waiting >= 2 and gauge == 0:
                wrong.append(f"{tenant}: {waiting} tasks wait, and the gauge says 0")
            elif waiting == 0 and gauge >= 2:
                wrong.append(f"{tenant}: no task waits, and the gauge says {int(gauge)}")
            elif waiting > 0 and gauge > 0:
                moved.append(tenant)
        if wrong:
            return broken("; ".join(wrong), wrong)
        if not seen:
            return unknown("the waiting gauge has no series of the workload's tenants")
        if not moved:
            return unknown("no task waits now (waiting/gauge: " + ", ".join(seen) + ")")
        return holds("the gauge shows the waiting tasks (waiting/gauge): " + ", ".join(seen))

    @registry.rule("operations.lag_gauge", "operations.backlog", CHAPTER,
                   tolerates=("postgres_away", "restart_server"),
                   about="the schedule lag gauge of each tenant shows how late its most overdue "
                         "schedule is, and goes back to 0 when none is due")
    def lag_gauge(snapshot):
        sums = gauge_by_tenant(snapshot.metrics(LAG_GAUGE))
        wrong = []
        moved = []
        seen = []
        for tenant in _shared.workload_tenants(snapshot):
            rows = snapshot.db(tenant, "operations_lag")
            lag = max(0.0, float(rows[0].get("lag_seconds") or 0)) if rows else 0.0
            gauge = sums.get(tenant)
            if gauge is None:
                continue
            seen.append(f"{tenant} {int(lag)}/{int(gauge)} s")
            if lag >= 180 and gauge < lag - 120:
                wrong.append(f"{tenant}: a schedule is {int(lag)} s late, and the gauge says {int(gauge)}")
            elif lag < 30 and gauge >= 180:
                wrong.append(f"{tenant}: no schedule is late, and the gauge says {int(gauge)}")
            elif lag >= 60 and gauge >= 30:
                moved.append(tenant)
        if wrong:
            return broken("; ".join(wrong), wrong)
        if not seen:
            return unknown("the lag gauge has no series of the workload's tenants")
        if not moved:
            return unknown("no schedule is late now (database/gauge: " + ", ".join(seen) + ")")
        return holds("the gauge shows the late schedules (database/gauge): " + ", ".join(seen))

    @registry.rule("operations.retention", "operations.retention", CHAPTER,
                   tolerates=("postgres_away",),
                   about=f"no finished run is older than the retention period "
                         f"({workload.RUN_HISTORY_DAYS} days), and the active runs stay")
    def retention(snapshot):
        now = _shared.clock(snapshot)
        wrong = []
        judged = []
        kept = 0
        for tenant in _shared.workload_tenants(snapshot):
            rows = snapshot.db(tenant, "operations_retention")
            row = rows[0] if rows else {}
            since = parse_time(row.get("tenant_since"))
            if since is None or now - since < RETENTION_SECONDS + RETENTION_SLACK_SECONDS:
                continue
            judged.append(tenant)
            old = _int(row.get("old_finished")) or 0
            orphans = _int(row.get("orphan_tasks")) or 0
            if old:
                wrong.append(f"{tenant}: {old} finished runs are older than "
                             f"{workload.RUN_HISTORY_DAYS} days")
            if orphans:
                wrong.append(f"{tenant}: {orphans} task rows have no run")
            active = parse_time(row.get("oldest_active"))
            if active is not None and now - active > RETENTION_SECONDS:
                kept += 1
        if wrong:
            return broken("; ".join(wrong), wrong)
        if not judged:
            return unknown(f"no tenant is older than the retention period of "
                           f"{workload.RUN_HISTORY_DAYS} days yet")
        return holds(f"no finished run older than {workload.RUN_HISTORY_DAYS} days in "
                     f"{', '.join(judged)}; {kept} of them keep an active run older than that")
