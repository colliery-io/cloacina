"""Rules of the chapter "Failure" (CLOACINA-T-1112).

An agent dies, a server replica restarts, Postgres goes away: does the work
survive? Each rule reads the disturbance record (CLOACINA-T-1109), finds every
run that was active when a disturbance began, and says how it ended. A window
is judged once it has closed (its `settle_until`); before that a run may still
be on its way. Inside the window a task may start again; after it, the rule
states what must be true.

The stated rule, from the docs (`explanation/service/execution-agent-fleet.md`
and `explanation/engine/cron-scheduling.md`, "Missed execution handling"):

- a task on an agent that dies runs again from its start on a live agent, at
  least once; an agent stopped with SIGTERM lets the task finish in its drain
  or hands it back;
- a task in flight when a server replica stops finishes once, on its first
  dispatch: no task is sent again;
- a fire time missed while the scheduler cannot run follows the schedule's
  catch-up policy: `skip` gives one run for the missed fire times, `run_all`
  one run for each; no fire time has two runs.

The docs name no rule for a task in flight while Postgres is away. The
chapter holds it to the rule of a server restart: it ends, and is not sent
again. A break there is a finding of the chapter.
"""

from __future__ import annotations

import math
import re

from .. import workload
from ..engine import broken, format_time, holds, parse_time, unknown
from . import _shared

CHAPTER = "failure"

TERMINAL = ("Completed", "Failed", "Cancelled")
# Hours of runs and fire times a pass reads. A window older than the judged
# span is not judged again: its runs may be out of the read.
LOOKBACK_HOURS = 6
JUDGED_SECONDS = (LOOKBACK_HOURS - 1) * 3600

# The runs of a tenant, each with what its tasks did: how many, the highest
# attempt, how many times a task was sent again (the stale-claim recovery),
# and the pod that finished the hold or archive job (the host it wrote into
# its result).
RUNS_SQL = (
    "SELECT we.id, we.workflow_name, we.status, we.started_at, we.completed_at, "
    "count(te.id) AS tasks, coalesce(max(te.attempt), 0) AS max_attempt, "
    "coalesce(sum(te.recovery_attempts), 0) AS recoveries, "
    "coalesce(c.value::jsonb -> 'standing_archive_done' ->> 'host', "
    "c.value::jsonb -> 'standing_hold_done' ->> 'host') AS finished_on "
    "FROM {schema}.workflow_executions we "
    "LEFT JOIN {schema}.task_executions te ON te.workflow_execution_id = we.id "
    "LEFT JOIN {schema}.contexts c ON c.id = we.context_id "
    f"WHERE we.started_at > now() - interval '{LOOKBACK_HOURS} hours' "
    "GROUP BY we.id, we.workflow_name, we.status, we.started_at, we.completed_at, c.value "
    "ORDER BY we.started_at"
)

# Each cron schedule of a tenant, with its fire times of the last hours and
# the status of the run each one started. A schedule with no fire time in
# that span is one row with no fire time.
FIRES_SQL = (
    "SELECT s.workflow_name, s.instance_name, s.catchup_policy, s.cron_expression, "
    "se.scheduled_time, we.status "
    "FROM {schema}.schedules s "
    "LEFT JOIN {schema}.schedule_executions se ON se.schedule_id = s.id "
    f"AND se.scheduled_time > now() - interval '{LOOKBACK_HOURS} hours' "
    "LEFT JOIN {schema}.workflow_executions we ON we.id = se.workflow_execution_id "
    "WHERE s.schedule_type = 'cron' AND s.enabled AND NOT s.paused "
    "ORDER BY s.workflow_name, s.instance_name, se.scheduled_time"
)

# The jobs whose runs each disturbance judges (workload.EVIDENCE).
AGENT_JOBS = tuple(workload.EVIDENCE["failure.agent_dies"])
SERVER_JOBS = tuple(workload.EVIDENCE["failure.server_restart"])
# The job that fails on purpose twice an hour.
FAILS_ON_PURPOSE = ("standing_flaky",)


def _short(ident):
    return str(ident or "")[:8]


def runs_of(snapshot, tenant):
    return runs_from_rows(snapshot.db(tenant, "failure_runs"), tenant)


def runs_from_rows(rows, tenant):
    """The rows of RUNS_SQL, each with its tenant and its times as epochs."""
    out = []
    for row in rows:
        row = dict(row)
        row["tenant"] = tenant
        row["start_epoch"] = parse_time(row.get("started_at"))
        row["end_epoch"] = parse_time(row.get("completed_at"))
        if row["start_epoch"] is not None:
            out.append(row)
    return out


def active_at(runs, at, jobs=None):
    """The runs that had started and not ended at `at`."""
    return [run for run in runs
            if (jobs is None or run.get("workflow_name") in jobs)
            and run["start_epoch"] <= at
            and (run["end_epoch"] is None or run["end_epoch"] >= at)]


def judged_windows(snapshot, kind, now):
    """(closed windows to judge, windows still settling) of a kind."""
    closed, settling = [], []
    for window in snapshot.windows(kind):
        if now - window.start > JUDGED_SECONDS:
            continue
        if window.closes is None or now < window.closes:
            settling.append(window)
        else:
            closed.append(window)
    return closed, settling


def describe(run):
    ended = format_time(run["end_epoch"]) or "not ended"
    return (f"{run['tenant']} {run.get('workflow_name')} {_short(run.get('id'))}: "
            f"{run.get('status')} ({ended}), tasks {run.get('tasks')}, "
            f"attempt {run.get('max_attempt')}, sent again {run.get('recoveries')}")


def sent_again(run):
    return int(run.get("recoveries") or 0) > 0


def _verdict(kind, closed, settling, judged, faults, details):
    pending = f"; {len(settling)} window(s) still settling" if settling else ""
    if faults:
        return broken(f"{len(faults)} of {judged} runs active at a {kind} did not survive{pending}",
                      faults + details)
    if not closed:
        return unknown(f"no {kind} window has closed in the last {LOOKBACK_HOURS - 1} h{pending}")
    if not judged:
        return unknown(f"{len(closed)} {kind} window(s), and no judged run was active in one{pending}",
                       details)
    return holds(f"{judged} runs active at {len(closed)} {kind} window(s) ended as the rule says"
                 f"{pending}", details)


# --- fire times -----------------------------------------------------------

_EVERY = re.compile(r"^\*/(\d+) \* \* \* \*$")
_AT_MINUTE = re.compile(r"^(\d+) \* \* \* \*$")


def fire_times(cron, begin, end):
    """The fire times of a cron expression in [begin, end], for the forms the
    workload uses (`*/N * * * *`, `M * * * *`, UTC). None for another form."""
    cron = " ".join(str(cron or "").split())
    every = _EVERY.match(cron)
    if every:
        period, offset = int(every.group(1)) * 60, 0
    else:
        minute = _AT_MINUTE.match(cron)
        if not minute:
            return None
        period, offset = 3600, int(minute.group(1)) * 60
    first = math.ceil((begin - offset) / period) * period + offset
    return [float(t) for t in range(int(first), int(end) + 1, period)]


def schedules_of(snapshot, tenant):
    return schedules_from_rows(snapshot.db(tenant, "failure_fires"))


def schedules_from_rows(rows):
    """The rows of FIRES_SQL as (workflow, instance) -> {policy, cron,
    fires: [(epoch, status)]}."""
    found = {}
    for row in rows:
        key = (row.get("workflow_name"), row.get("instance_name"))
        entry = found.setdefault(key, {"policy": (row.get("catchup_policy") or "skip"),
                                       "cron": row.get("cron_expression"), "fires": []})
        stamp = parse_time(row.get("scheduled_time"))
        if stamp is not None:
            entry["fires"].append((stamp, row.get("status")))
    return found


def _name(tenant, key):
    workflow, instance = key
    return f"{tenant}/{workflow}" + (f"/{instance}" if instance else "")


def fire_faults(tenant, key, entry, window):
    """What is wrong with one schedule's fire times around one window, and
    what was seen."""
    begin, end = window.start, window.end
    due = fire_times(entry["cron"], begin, end)
    name = _name(tenant, key)
    if due is None:
        return [], [f"{name}: cron {entry['cron']!r} not read by the rule"]
    if not due:
        return [], []
    faults = []
    counts = {t: 0 for t in due}
    for stamp, _status in entry["fires"]:
        for t in due:
            if abs(stamp - t) < 1.0:
                counts[t] += 1
    doubled = [format_time(t) for t, n in counts.items() if n > 1]
    if doubled:
        faults.append(f"{name}: two runs for the fire time(s) {', '.join(doubled)}")
    seen = (f"{name} ({entry['policy']}): fire times {', '.join(format_time(t) for t in due)}, "
            f"runs {', '.join(str(counts[t]) for t in due)}")
    if entry["policy"] == "run_all":
        lost = [format_time(t) for t, n in counts.items() if n == 0]
        if lost:
            faults.append(f"{name}: run_all, and no run for the fire time(s) {', '.join(lost)}")
    else:
        in_window = [s for s, _ in entry["fires"] if begin - 1.0 <= s <= end + 1.0]
        if not in_window:
            faults.append(f"{name}: skip, and no run for the fire times of the window "
                          f"{format_time(due[0])} to {format_time(due[-1])}")
    unended = [format_time(s) for s, status in entry["fires"]
               if begin - 1.0 <= s <= end + 1.0 and status not in TERMINAL]
    if unended:
        faults.append(f"{name}: the run of the fire time(s) {', '.join(unended)} has not ended")
    return faults, [seen]


def register(registry):
    _shared.need_clock(registry)
    registry.need_db("failure_runs", RUNS_SQL)
    registry.need_db("failure_fires", FIRES_SQL)

    @registry.rule("failure.agent_death", "failure.agent_dies", CHAPTER, tolerates=("kill_agent",),
                   about="each run of a held or long job that was active when an agent pod of its "
                         "tenant was killed ended Completed, and not on the pod that was killed")
    def agent_death(snapshot):
        now = _shared.clock(snapshot)
        closed, settling = judged_windows(snapshot, "kill_agent", now)
        faults, details, judged = [], [], 0
        runs_by_tenant = {}
        for window in closed:
            tenant = window.extra.get("tenant")
            outcome = str(window.extra.get("outcome") or "")
            if tenant not in snapshot.tenants or not outcome.startswith("pod deleted"):
                details.append(f"{window.label()}: not judged ({outcome or 'no outcome'})")
                continue
            if tenant not in runs_by_tenant:
                runs_by_tenant[tenant] = runs_of(snapshot, tenant)
            pod, mode = window.extra.get("pod"), window.extra.get("mode")
            for run in active_at(runs_by_tenant[tenant], window.start, AGENT_JOBS):
                judged += 1
                text = f"{describe(run)}; finished on {run.get('finished_on') or 'unknown'}"
                details.append(f"{window.label()} ({mode}): {text}")
                if run.get("status") != "Completed":
                    faults.append(f"{text}: not Completed after the window")
                elif (mode == "force" and pod and run.get("finished_on") == pod
                      and (run["end_epoch"] or 0) > (window.end or window.start)):
                    faults.append(f"{text}: finished on the pod that was killed, after it was killed")
        return _verdict("kill_agent", closed, settling, judged, faults, details)

    @registry.rule("failure.server_restart", "failure.server_restart", CHAPTER,
                   tolerates=("restart_server",),
                   about="each run that was active when a server replica restarted ended "
                         "Completed, and none of its tasks was sent again")
    def server_restart(snapshot):
        now = _shared.clock(snapshot)
        closed, settling = judged_windows(snapshot, "restart_server", now)
        faults, details, judged = [], [], 0
        tenants = _shared.workload_tenants(snapshot)
        runs = {tenant: runs_of(snapshot, tenant) for tenant in tenants} if closed else {}
        for window in closed:
            replicas = window.extra.get("replicas")
            label = f"{window.label()} ({window.extra.get('mode')}, {replicas} replicas)"
            for tenant in tenants:
                for run in active_at(runs[tenant], window.start, SERVER_JOBS):
                    judged += 1
                    details.append(f"{label}: {describe(run)}")
                    if run.get("status") != "Completed":
                        faults.append(f"{describe(run)}: not Completed after the window")
                    elif sent_again(run):
                        faults.append(f"{describe(run)}: a task was sent again across the restart")
        return _verdict("restart_server", closed, settling, judged, faults, details)

    @registry.rule("failure.postgres_fire_times", "failure.postgres_away", CHAPTER,
                   tolerates=("postgres_away",),
                   about="after Postgres came back, each fire time of the window has a run as its "
                         "schedule's catch-up policy says, and no fire time has two")
    def postgres_fire_times(snapshot):
        now = _shared.clock(snapshot)
        closed, settling = judged_windows(snapshot, "postgres_away", now)
        faults, details, judged = [], [], 0
        tenants = _shared.workload_tenants(snapshot)
        schedules = {tenant: schedules_of(snapshot, tenant) for tenant in tenants} if closed else {}
        for window in closed:
            if window.end is None:
                continue
            for tenant in tenants:
                for key, entry in sorted(schedules[tenant].items(), key=lambda item: str(item[0])):
                    found, seen = fire_faults(tenant, key, entry, window)
                    if seen and not seen[0].endswith("not read by the rule"):
                        judged += 1
                    faults.extend(f"{window.label()}: {fault}" for fault in found)
                    details.extend(f"{window.label()}: {line}" for line in seen)
        if faults:
            return broken(f"{len(faults)} faults in the fire times of {len(closed)} postgres_away "
                          f"window(s)", faults + details)
        if not closed:
            return unknown(f"no postgres_away window has closed in the last {LOOKBACK_HOURS - 1} h")
        if not judged:
            return unknown("no schedule had a fire time in a closed postgres_away window", details)
        return holds(f"{judged} schedules over {len(closed)} postgres_away window(s): each fire time "
                     "has its runs as the catch-up policy says", details)

    @registry.rule("failure.postgres_runs_end", "failure.postgres_away", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each run that was active when Postgres went away has ended, and none of "
                         "its tasks was sent again")
    def postgres_runs_end(snapshot):
        now = _shared.clock(snapshot)
        closed, settling = judged_windows(snapshot, "postgres_away", now)
        faults, details, judged = [], [], 0
        tenants = _shared.workload_tenants(snapshot)
        runs = {tenant: runs_of(snapshot, tenant) for tenant in tenants} if closed else {}
        for window in closed:
            for tenant in tenants:
                for run in active_at(runs[tenant], window.start):
                    judged += 1
                    details.append(f"{window.label()}: {describe(run)}")
                    fault = postgres_run_fault(run)
                    if fault:
                        faults.append(fault)
        return _verdict("postgres_away", closed, settling, judged, faults, details)


def postgres_run_fault(run):
    """What is wrong with a run that was active when Postgres went away, once
    its window has closed. None when it ended as the chapter holds."""
    status = run.get("status")
    if status not in TERMINAL:
        return f"{describe(run)}: has not ended after the window"
    if status != "Completed" and run.get("workflow_name") not in FAILS_ON_PURPOSE:
        return f"{describe(run)}: ended {status}"
    if sent_again(run):
        return f"{describe(run)}: a task was sent again"
    return None
