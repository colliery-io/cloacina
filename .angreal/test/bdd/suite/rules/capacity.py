"""Rules of the Capacity row (CLOACINA-T-1114).

The self-driving workflow of each tenant starts more holds at once than the
tenant's agents can take, once an hour (`workload.SLOT_ACTIONS`). For a few
minutes the tenant is full. These rules judge, on every pass, what happened
while a tenant was full: the held work ran, nothing failed for lack of
room, the other tenants' schedules kept their time, waiting work went in
priority order, and every task in flight is on an agent that fits it.

When a tenant is full is read from the rows, not from the driver's record:
a tenant is full while one of its holds waits for an agent (created, and not
started for more than `WAIT_SECONDS`).

The scenarios of `features/capacity/` judge the same promises once, step by
step, with work they start themselves.
"""

from __future__ import annotations

import json

from .. import workload
from ..engine import broken, format_time, holds, parse_time, unknown
from . import _shared

CHAPTER = "capacity"

# A hold that did not start this long after it was made waited for an agent.
WAIT_SECONDS = 10
# How long a burst of holds can take to drain, with every slot of the tenant
# busy: one hold length for each round of slots, one more, and two minutes.
_ROUNDS = -(-workload.HOLD_BURST // (workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS))
SETTLE_SECONDS = workload.HOLD_SECONDS * (_ROUNDS + 1) + 120
# A scheduled run of a tenant that is not full starts its first task within
# this long of its fire time.
ON_TIME_SECONDS = 60
# Waiting work ages one priority step for each this many seconds (the
# server's default, `--priority-aging-interval-s`).
AGING_SECONDS = 120
# Two dispatches of one pass start within this long of each other.
ORDER_SLACK_SECONDS = 5
# The pool of a task that names none (`cloacina_api_types::DEFAULT_AGENT_POOL`).
DEFAULT_POOL = "default"
# An error with this text is a planned failure of the workload
# (`standing-flaky-py`), not a failure for lack of room.
PLANNED_FAILURE = "planned failure"

# Each hold task of the last three hours, with its run and its priority.
HOLD_TASKS_SQL = (
    "SELECT we.id AS run, we.status AS run_status, we.priority AS priority, "
    "te.id AS task, te.status, te.attempt, te.pool, te.created_at, te.started_at, "
    "te.completed_at, "
    "(SELECT count(*) FROM {schema}.task_attempt_errors e WHERE e.task_execution_id = te.id) "
    "AS attempt_errors "
    "FROM {schema}.workflow_executions we "
    "JOIN {schema}.task_executions te ON te.workflow_execution_id = we.id "
    f"WHERE we.workflow_name = '{workload.HOLD_WORKFLOW}' "
    "AND we.started_at > now() - interval '3 hours' "
    "ORDER BY te.created_at"
)

# Each task that failed, and each attempt that failed, in the last three
# hours. The error is cut: it is evidence, not a log.
FAILURES_SQL = (
    "SELECT we.workflow_name AS workflow, te.task_name AS task, 'task' AS kind, "
    "te.completed_at AS at, left(coalesce(te.last_error, te.error_details, ''), 300) AS error "
    "FROM {schema}.task_executions te "
    "JOIN {schema}.workflow_executions we ON we.id = te.workflow_execution_id "
    "WHERE te.status = 'Failed' AND te.completed_at > now() - interval '3 hours' "
    "UNION ALL "
    "SELECT we.workflow_name, te.task_name, 'attempt', e.failed_at, left(e.message, 300) "
    "FROM {schema}.task_attempt_errors e "
    "JOIN {schema}.task_executions te ON te.id = e.task_execution_id "
    "JOIN {schema}.workflow_executions we ON we.id = e.workflow_execution_id "
    "WHERE e.failed_at > now() - interval '3 hours' "
    "ORDER BY 4"
)

# Each fire of a package's own schedule in the last three hours, with the
# start of the run's first task.
SCHEDULED_RUNS_SQL = (
    "SELECT s.workflow_name AS workflow, se.scheduled_time, "
    "min(te.started_at) AS first_task_started "
    "FROM {schema}.schedule_executions se "
    "JOIN {schema}.schedules s ON s.id = se.schedule_id "
    "LEFT JOIN {schema}.task_executions te ON te.workflow_execution_id = se.workflow_execution_id "
    "WHERE s.instance_name IS NULL AND s.schedule_type = 'cron' "
    "AND se.scheduled_time > now() - interval '3 hours' "
    "GROUP BY 1, 2 ORDER BY 2"
)

# The tasks on an agent now: placed, with a claim held, not ended
# (`TaskExecutionDAL::fleet_placements`).
IN_FLIGHT_SQL = (
    "SELECT te.id AS task, te.task_name, te.status, te.pool, te.cpu_request_millis, "
    "te.memory_request_bytes, te.fleet_agent_id "
    "FROM {schema}.task_executions te "
    "WHERE te.fleet_agent_id IS NOT NULL AND te.claimed_by IS NOT NULL "
    "AND te.status IN ('Ready', 'Running')"
)

# The live agents, with their pool (a `pool:<name>` capability) and totals.
# A global table, in the schema `cloacina_global` (CLOACINA-T-1080).
LIVE_AGENTS_SQL = (
    "SELECT agent_id, tenant_id, capabilities, max_concurrency, available_capacity, "
    "cpu_millis, memory_bytes, last_heartbeat_at "
    "FROM cloacina_global.fleet_agents "
    "WHERE last_heartbeat_at > now() - interval '2 minutes'"
)


def _int(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def pool_of_agent(capabilities):
    """The pool of an agent: its `pool:<name>` capability, else `default`.
    `capabilities` is a JSON string array, or a list."""
    if isinstance(capabilities, str):
        try:
            capabilities = json.loads(capabilities)
        except ValueError:
            capabilities = []
    for item in capabilities or []:
        if isinstance(item, str) and item.startswith("pool:") and item[5:]:
            return item[5:]
    return DEFAULT_POOL


def charge(request, total, max_concurrency):
    """What a task counts for on an agent in one resource: its request, or
    one slot's share of the agent's total; nothing when the agent reports no
    total (`agent_registry::charge`)."""
    if total is None:
        return 0
    if request is not None:
        return request
    return total // max(1, max_concurrency or 1)


def full_windows(rows, now):
    """When a tenant was full: for each hold that waited more than
    WAIT_SECONDS, from when it was made to when it started (or now, while it
    waits). Merged and sorted: [(start, end)]."""
    spans = []
    for row in rows:
        made = parse_time(row.get("created_at"))
        started = parse_time(row.get("started_at"))
        if made is None:
            continue
        end = started if started is not None else (now if row.get("status") == "Ready" else None)
        if end is not None and end - made > WAIT_SECONDS:
            spans.append((made, end))
    spans.sort()
    merged = []
    for start, end in spans:
        if merged and start <= merged[-1][1]:
            merged[-1] = (merged[-1][0], max(merged[-1][1], end))
        else:
            merged.append((start, end))
    return merged


def _inside(stamp, windows):
    return stamp is not None and any(start <= stamp <= end for start, end in windows)


def aged(priority, waiting_since, at):
    """The priority of waiting work at time `at`: one step better for each
    AGING_SECONDS it has waited, never better than 0."""
    steps = max(0, int((at - waiting_since) // AGING_SECONDS))
    return max(0, priority - steps)


def priority_inversions(rows):
    """Each start of a hold while a hold of the same tenant and pool, with a
    better priority after ageing, still waited. `rows` are the hold rows of
    one tenant. Returns text, one line for each."""
    tasks = []
    for row in rows:
        made = parse_time(row.get("created_at"))
        priority = _int(row.get("priority"))
        if made is None or priority is None:
            continue
        tasks.append({"made": made, "started": parse_time(row.get("started_at")),
                      "priority": priority, "pool": row.get("pool") or DEFAULT_POOL,
                      "task": row.get("task")})
    found = []
    for late in tasks:
        at = late["started"]
        if at is None:
            continue
        for early in tasks:
            if early is late or early["pool"] != late["pool"] or early["made"] > at:
                continue
            still_waiting = early["started"] is None or early["started"] > at + ORDER_SLACK_SECONDS
            # Work of one priority is not judged for its order here; ageing
            # lets worse work go once it has aged level.
            if not still_waiting or early["priority"] >= late["priority"]:
                continue
            if aged(early["priority"], early["made"], at) < aged(late["priority"], late["made"], at):
                found.append(f"a hold at priority {late['priority']} started at {format_time(at)} "
                             f"while one at priority {early['priority']}, waiting since "
                             f"{format_time(early['made'])}, still waited")
                break
    return found


def register(registry):
    _shared.need_clock(registry)
    registry.need_db("capacity_hold_tasks", HOLD_TASKS_SQL)
    registry.need_db("capacity_failures", FAILURES_SQL)
    registry.need_db("capacity_scheduled_runs", SCHEDULED_RUNS_SQL)
    registry.need_db("capacity_in_flight", IN_FLIGHT_SQL)
    registry.need_admin_db("capacity_live_agents", LIVE_AGENTS_SQL)

    def windows_of(snapshot):
        now = _shared.clock(snapshot)
        return {tenant: full_windows(snapshot.db(tenant, "capacity_hold_tasks"), now)
                for tenant in _shared.workload_tenants(snapshot)}

    @registry.rule("capacity.held_work_runs", "capacity.waits", CHAPTER,
                   tolerates=("kill_agent", "restart_server", "postgres_away"),
                   about="each hold of a full tenant completed once it could run: one attempt, "
                         "no failed attempt, and none still waiting after the burst should be over")
    def held_work_runs(snapshot):
        now = _shared.clock(snapshot)
        wrong = []
        judged = 0
        waited = 0
        for tenant in _shared.workload_tenants(snapshot):
            for row in snapshot.db(tenant, "capacity_hold_tasks"):
                made = parse_time(row.get("created_at"))
                if made is None or now - made < SETTLE_SECONDS:
                    continue
                judged += 1
                started = parse_time(row.get("started_at"))
                if started is not None and started - made > WAIT_SECONDS:
                    waited += 1
                name = f"{tenant}: the hold made at {format_time(made)}"
                if row.get("run_status") != "Completed":
                    wrong.append(f"{name} is {row.get('run_status')} "
                                 f"(task {row.get('status')}), {int(now - made)} s after it was made")
                elif (_int(row.get("attempt")) or 0) > 1 or (_int(row.get("attempt_errors")) or 0):
                    wrong.append(f"{name} took {row.get('attempt')} attempts "
                                 f"({row.get('attempt_errors')} failed)")
        if wrong:
            return broken(f"{len(wrong)} of {judged} holds did not run once to the end", wrong)
        if not judged:
            return unknown("no hold is old enough to judge yet")
        if not waited:
            return unknown(f"{judged} holds completed, but none waited for an agent: "
                           "no tenant was full in the window")
        return holds(f"{judged} holds completed once each; {waited} of them waited for an agent first")

    @registry.rule("capacity.no_failure_while_full", "capacity.waits", CHAPTER,
                   tolerates=("kill_agent", "restart_server", "postgres_away"),
                   about="no task or attempt of a tenant failed while the tenant was full, "
                         "apart from the planned failures of the workload")
    def no_failure_while_full(snapshot):
        windows = windows_of(snapshot)
        wrong = []
        count = 0
        for tenant, spans in windows.items():
            count += len(spans)
            for row in snapshot.db(tenant, "capacity_failures"):
                if PLANNED_FAILURE in str(row.get("error") or ""):
                    continue
                at = parse_time(row.get("at"))
                if _inside(at, spans):
                    wrong.append(f"{tenant}: {row.get('kind')} of {row.get('workflow')}/"
                                 f"{row.get('task')} failed at {format_time(at)}, while the tenant "
                                 f"was full: {str(row.get('error') or '')[:200]}")
        if wrong:
            return broken(f"{len(wrong)} failures while a tenant was full", wrong)
        if not count:
            return unknown("no tenant was full in the window")
        return holds(f"{count} times a tenant was full; no task failed in them")

    @registry.rule("capacity.others_on_time", "capacity.waits", CHAPTER,
                   tolerates=("kill_agent", "restart_server", "postgres_away"),
                   about="while one tenant was full, each scheduled run of every other tenant "
                         f"started its first task within {ON_TIME_SECONDS} s of its fire time")
    def others_on_time(snapshot):
        now = _shared.clock(snapshot)
        windows = windows_of(snapshot)
        late = []
        judged = 0
        for tenant in windows:
            others = [span for other, spans in windows.items() if other != tenant for span in spans]
            own = windows[tenant]
            for row in snapshot.db(tenant, "capacity_scheduled_runs"):
                fired = parse_time(row.get("scheduled_time"))
                if not _inside(fired, others) or _inside(fired, own):
                    continue
                first = parse_time(row.get("first_task_started"))
                if first is None and now - fired <= ON_TIME_SECONDS:
                    continue  # not due yet
                judged += 1
                if first is None:
                    late.append(f"{tenant}/{row.get('workflow')}: the fire of {format_time(fired)} "
                                f"has started no task, {int(now - fired)} s later")
                elif first - fired > ON_TIME_SECONDS:
                    late.append(f"{tenant}/{row.get('workflow')}: the fire of {format_time(fired)} "
                                f"started its first task {int(first - fired)} s late")
        if late:
            return broken(f"{len(late)} of {judged} runs of other tenants were late while one "
                          "tenant was full", late)
        if not judged:
            return unknown("no run of another tenant fired while a tenant was full")
        return holds(f"{judged} runs of other tenants started on time while a tenant was full")

    @registry.rule("capacity.priority_order", "capacity.waits", CHAPTER,
                   tolerates=("kill_agent", "restart_server", "postgres_away"),
                   about="no hold started while a hold of the same tenant and pool with a better "
                         f"priority, after ageing one step each {AGING_SECONDS} s, still waited")
    def priority_order(snapshot):
        wrong = []
        mixed = 0
        for tenant in _shared.workload_tenants(snapshot):
            rows = snapshot.db(tenant, "capacity_hold_tasks")
            waited = [row for row in rows if (parse_time(row.get("started_at")) or 0)
                      - (parse_time(row.get("created_at")) or 0) > WAIT_SECONDS]
            if len({_int(row.get("priority")) for row in waited}) > 1:
                mixed += 1
            wrong.extend(f"{tenant}: {line}" for line in priority_inversions(rows))
        if wrong:
            return broken(f"{len(wrong)} holds started out of priority order", wrong)
        if not mixed:
            return unknown("no tenant had waiting holds of more than one priority")
        return holds(f"waiting holds of {mixed} tenants started in priority order")

    @registry.rule("capacity.fits_in_flight", "capacity.fits", CHAPTER,
                   tolerates=("kill_agent",),
                   about="each task on an agent now is on a live agent of its tenant and pool, and "
                         "no agent holds more tasks, CPU or memory than it has")
    def fits_in_flight(snapshot):
        agents = {row.get("agent_id"): row for row in snapshot.db(None, "capacity_live_agents")}
        wrong = []
        placed = 0
        load = {}
        for tenant in _shared.workload_tenants(snapshot):
            for row in snapshot.db(tenant, "capacity_in_flight"):
                agent = agents.get(row.get("fleet_agent_id"))
                if agent is None:
                    continue  # the agent is gone; `standing.agents` judges that
                placed += 1
                pool = row.get("pool") or DEFAULT_POOL
                name = f"{tenant}: {row.get('task_name')}"
                if agent.get("tenant_id") != tenant:
                    wrong.append(f"{name} is on {row.get('fleet_agent_id')}, an agent of "
                                 f"{agent.get('tenant_id')}")
                if pool_of_agent(agent.get("capabilities")) != pool:
                    wrong.append(f"{name} names the pool {pool} and is on "
                                 f"{row.get('fleet_agent_id')}, of the pool "
                                 f"{pool_of_agent(agent.get('capabilities'))}")
                slots = _int(agent.get("max_concurrency")) or 1
                entry = load.setdefault(row.get("fleet_agent_id"), [0, 0, 0])
                entry[0] += 1
                entry[1] += charge(_int(row.get("cpu_request_millis")),
                                   _int(agent.get("cpu_millis")), slots)
                entry[2] += charge(_int(row.get("memory_request_bytes")),
                                   _int(agent.get("memory_bytes")), slots)
        for agent_id, (tasks, cpu, memory) in sorted(load.items()):
            agent = agents[agent_id]
            slots = _int(agent.get("max_concurrency")) or 1
            cpu_total = _int(agent.get("cpu_millis"))
            memory_total = _int(agent.get("memory_bytes"))
            if tasks > slots:
                wrong.append(f"{agent_id} holds {tasks} tasks and has {slots} slots")
            if cpu_total is not None and cpu > cpu_total:
                wrong.append(f"{agent_id} holds {cpu}m of CPU and has {cpu_total}m")
            if memory_total is not None and memory > memory_total:
                wrong.append(f"{agent_id} holds {memory} bytes of memory and has {memory_total}")
        if wrong:
            return broken(f"{len(wrong)} placements that do not fit", wrong)
        if not placed:
            return unknown("no task is on an agent right now")
        return holds(f"{placed} tasks in flight on {len(load)} agents, each on an agent of its "
                     "tenant and pool with room for it")
