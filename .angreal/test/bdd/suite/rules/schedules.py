"""Rules of the Schedules row (CLOACINA-T-1111).

For each schedule of the standing workload, the checker computes the fire
times from the schedule's cron expression in `workload.py` (with
`suite/cron.py`), never from the server's own `next_run_at`, and compares
them with the fires the server recorded (`schedule_executions`) and the runs
they started.

Times that are compared were written by the server: a fire time, the start
of its run, the interval the run wrote. The database clock says whether a
fire time is due yet, with a margin.

The scenarios of `features/schedules/` show each promise once. These rules
judge them on every pass, for every tenant.
"""

from __future__ import annotations

import json

from .. import workload
from ..cron import Cron
from ..engine import broken, format_time, holds, parse_time, unknown
from . import _shared

CHAPTER = "schedules"

# The longest a run may start after its fire time and still be on time. The
# first full run (CLOACINA-T-1116) measures the real lateness; the feature
# states the same limit.
LATE_LIMIT_SECONDS = 60
# A fire time is due this long after it passes: the scheduler polls, and the
# pass reads the database a little later.
DUE_MARGIN_SECONDS = 180
# The window the rules read, in minutes.
WINDOW_MINUTES = 170
# The kinds of disturbance in which no scheduler may run.
SCHEDULER_AWAY = ("restart_server", "postgres_away")
# Where the data jobs write the interval they were given.
INTERVAL_KEYS = {"standing_rollup": "rollup_done", "standing_extract": "extract_loaded"}
BUSY_WORKFLOWS = ("standing_archive", workload.HOLD_WORKFLOW)

SCHEDULES_SQL = (
    "SELECT id, workflow_name, instance_name, trigger_name, cron_expression, timezone, "
    "catchup_policy, max_active_runs, enabled, paused, backfill, created_at "
    "FROM {schema}.schedules WHERE schedule_type = 'cron'"
)

FIRES_SQL = (
    "SELECT se.schedule_id, se.scheduled_time, se.started_at AS claimed_at, "
    "se.workflow_execution_id AS run_id, we.status, we.started_at, we.completed_at, "
    "c.value::jsonb -> 'standing_rollup_done' AS rollup_done, "
    "c.value::jsonb -> 'standing_extract_loaded' AS extract_loaded "
    "FROM {schema}.schedule_executions se "
    "LEFT JOIN {schema}.workflow_executions we ON we.id = se.workflow_execution_id "
    "LEFT JOIN {schema}.contexts c ON c.id = we.context_id "
    f"WHERE se.scheduled_time > now() - interval '{WINDOW_MINUTES} minutes' "
    "ORDER BY se.scheduled_time"
)

_BUSY_IN = ", ".join(f"'{name}'" for name in BUSY_WORKFLOWS)
BUSY_SQL = (
    "SELECT workflow_name, started_at, completed_at FROM {schema}.workflow_executions "
    f"WHERE workflow_name IN ({_BUSY_IN}) "
    f"AND started_at > now() - interval '{WINDOW_MINUTES + 60} minutes'"
)


def _object(value):
    if isinstance(value, dict):
        return value
    if isinstance(value, str) and value:
        try:
            parsed = json.loads(value)
        except ValueError:
            return {}
        return parsed if isinstance(parsed, dict) else {}
    return {}


def _true(value):
    return value is True or str(value).lower() in ("t", "true", "1")


def schedule_name(schedule):
    if schedule.kind == "instance":
        return f"{schedule.workflow}/{schedule.instance}"
    return f"{schedule.workflow} (its package's own schedule)"


class Judged:
    """One schedule of the workload in one tenant: its rows and its fires."""

    def __init__(self, tenant, schedule, rows, fires):
        self.tenant = tenant
        self.schedule = schedule
        self.rows = rows
        ids = {str(row.get("id")) for row in rows}
        self.fires = [fire for fire in fires if str(fire.get("schedule_id")) in ids]
        zone = next((row.get("timezone") for row in rows if row.get("timezone")), None)
        self.cron = Cron(schedule.cron, zone or "UTC")
        made = [parse_time(row.get("created_at")) for row in rows]
        self.created = min((stamp for stamp in made if stamp is not None), default=None)

    @property
    def name(self):
        return f"{self.tenant}/{schedule_name(self.schedule)}"

    @property
    def catches_up(self):
        return self.schedule.catchup == "run_all"

    def by_fire_time(self):
        """fire time (minute) -> the fires the server recorded for it."""
        found = {}
        for fire in self.fires:
            stamp = parse_time(fire.get("scheduled_time"))
            if stamp is not None:
                found.setdefault(int(stamp) // 60 * 60, []).append(fire)
        return found


def judged_schedules(snapshot, tenant):
    """Each schedule of the workload in a tenant, with its rows. A schedule
    with no row is in the second list."""
    rows = [row for row in snapshot.db(tenant, "schedules")
            if not row.get("backfill") and _true(row.get("enabled"))]
    fires = snapshot.db(tenant, "fires")
    found, missing = [], []
    for schedule in workload.SCHEDULES:
        instance = schedule.instance if schedule.kind == "instance" else None
        mine = [row for row in rows if row.get("workflow_name") == schedule.workflow
                and (row.get("instance_name") or None) == instance]
        if mine:
            found.append(Judged(tenant, schedule, mine, fires))
        else:
            missing.append(f"{tenant}/{schedule_name(schedule)}")
    return found, missing


def scheduler_away(snapshot, fire_time, period):
    """True when a scheduler could not run at a fire time: a restart of the
    server or Postgres away, from one period before it to its close."""
    return any(window.overlaps(fire_time - period, fire_time)
               for kind in SCHEDULER_AWAY for window in snapshot.windows(kind))


def register(registry):
    _shared.need_clock(registry)
    registry.need_db("schedules", SCHEDULES_SQL)
    registry.need_db("fires", FIRES_SQL)
    registry.need_db("busy_runs", BUSY_SQL)

    @registry.rule("schedules.every_fire", "schedules.on_time", CHAPTER,
                   tolerates=SCHEDULER_AWAY,
                   about="each schedule of the workload has exactly one run for each fire time "
                         "its cron expression gives, and none for any other time")
    def every_fire(snapshot):
        now = _shared.clock(snapshot)
        faults = []
        judged = 0
        excused = 0
        for tenant in _shared.workload_tenants(snapshot):
            found, missing = judged_schedules(snapshot, tenant)
            faults.extend(f"{name}: no schedule is registered" for name in missing)
            for item in found:
                if len(item.rows) > 1:
                    faults.append(f"{item.name}: registered {len(item.rows)} times "
                                  f"({', '.join(sorted(str(r.get('id')) for r in item.rows))})")
                start = max(now - WINDOW_MINUTES * 60 + 60, item.created or now)
                expected = item.cron.between(start, now - DUE_MARGIN_SECONDS)
                fired = item.by_fire_time()
                newest = max(fired) if fired else None
                for fire_time in expected:
                    judged += 1
                    runs = fired.get(int(fire_time), [])
                    if len(runs) > 1:
                        faults.append(f"{item.name}: {len(runs)} runs for the fire time "
                                      f"{format_time(fire_time)}")
                    elif not runs:
                        if item.catches_up and (newest is None or fire_time > newest):
                            continue  # still waiting in its backlog
                        if not item.catches_up and scheduler_away(snapshot, fire_time,
                                                                  item.schedule.period_s):
                            excused += 1
                            continue
                        faults.append(f"{item.name}: no run for the fire time "
                                      f"{format_time(fire_time)}")
                wanted = {int(stamp) for stamp in expected}
                for stamp in sorted(fired):
                    if stamp >= start and stamp not in wanted and not item.cron.matches(stamp):
                        faults.append(f"{item.name}: a run for {format_time(stamp)}, which is not "
                                      f"a fire time of {item.schedule.cron}")
        if faults:
            return broken(f"{len(faults)} faults in {judged} fire times", faults)
        if not judged:
            return unknown("no fire time of a schedule of the workload is due in the window yet")
        note = f" ({excused} fire times while no scheduler could run not counted)" if excused else ""
        return holds(f"{judged} fire times, each with exactly one run{note}")

    @registry.rule("schedules.on_time", "schedules.on_time", CHAPTER,
                   tolerates=SCHEDULER_AWAY,
                   about=f"each run of a schedule started within {LATE_LIMIT_SECONDS} s of its "
                         "fire time, also while the long job and held tasks ran")
    def on_time(snapshot):
        now = _shared.clock(snapshot)
        late = []
        judged = 0
        busy_count = 0
        worst = 0.0
        for tenant in _shared.workload_tenants(snapshot):
            busy = []
            for run in snapshot.db(tenant, "busy_runs"):
                begin = parse_time(run.get("started_at"))
                if begin is not None:
                    busy.append((begin, parse_time(run.get("completed_at")) or now))
            found, _missing = judged_schedules(snapshot, tenant)
            for item in found:
                if item.catches_up:
                    continue  # a backlog starts late by design; see backlog_order
                for fire in item.fires:
                    fire_time = parse_time(fire.get("scheduled_time"))
                    started = parse_time(fire.get("started_at")) or parse_time(fire.get("claimed_at"))
                    if fire_time is None or started is None or now - fire_time < DUE_MARGIN_SECONDS:
                        continue
                    if scheduler_away(snapshot, fire_time, item.schedule.period_s):
                        continue
                    judged += 1
                    delay = started - fire_time
                    worst = max(worst, delay)
                    if any(begin <= fire_time <= end for begin, end in busy):
                        busy_count += 1
                    if delay > LATE_LIMIT_SECONDS:
                        late.append(f"{item.name}: the run for {format_time(fire_time)} started "
                                    f"{int(delay)} s after it (limit {LATE_LIMIT_SECONDS} s)")
        if late:
            return broken(f"{len(late)} of {judged} runs started late", late)
        if not judged:
            return unknown("no run of a schedule of the workload is in the window yet")
        return holds(f"{judged} runs started on time, {busy_count} of them while the long job or "
                     f"held tasks ran; the latest started {int(worst)} s after its fire time "
                     f"(limit {LATE_LIMIT_SECONDS} s)")

    @registry.rule("schedules.interval", "schedules.interval", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each completed run of the data jobs wrote the interval of its fire time: "
                         "from the fire time before it to its own")
    def interval(snapshot):
        wrong = []
        judged = 0
        for tenant in _shared.workload_tenants(snapshot):
            found, _missing = judged_schedules(snapshot, tenant)
            for item in found:
                key = INTERVAL_KEYS.get(item.schedule.workflow)
                if key is None:
                    continue
                for fire in item.fires:
                    if fire.get("status") != "Completed":
                        continue
                    fire_time = parse_time(fire.get("scheduled_time"))
                    if fire_time is None:
                        continue
                    judged += 1
                    written = _object(fire.get(key))
                    start = parse_time(written.get("interval_start"))
                    end = parse_time(written.get("interval_end"))
                    expected_start = item.cron.previous_before(fire_time)
                    where = f"{item.name}: the run for {format_time(fire_time)}"
                    if start is None or end is None:
                        wrong.append(f"{where} wrote no interval")
                    elif int(end) != int(fire_time) or int(start) != int(expected_start):
                        wrong.append(f"{where} wrote {format_time(start)} to {format_time(end)}, "
                                     f"not {format_time(expected_start)} to {format_time(fire_time)}")
        if wrong:
            return broken(f"{len(wrong)} of {judged} runs wrote the wrong interval", wrong)
        if not judged:
            return unknown("no completed run of a data job is in the window yet")
        return holds(f"{judged} runs each wrote the interval of their fire time")

    @registry.rule("schedules.backlog_order", "schedules.backlog", CHAPTER,
                   tolerates=("postgres_away",),
                   about="the runs of a schedule that runs every missed fire time start in the "
                         "order of their fire times, with no more active at once than its limit")
    def backlog_order(snapshot):
        now = _shared.clock(snapshot)
        wrong = []
        judged = 0
        behind = 0
        for tenant in _shared.workload_tenants(snapshot):
            found, _missing = judged_schedules(snapshot, tenant)
            for item in found:
                if not item.catches_up:
                    continue
                limit = item.schedule.max_active_runs or 1
                runs = []
                for fire in item.fires:
                    fire_time = parse_time(fire.get("scheduled_time"))
                    started = parse_time(fire.get("started_at"))
                    if fire_time is None or started is None:
                        continue
                    ended = parse_time(fire.get("completed_at")) or now
                    runs.append((fire_time, started, ended))
                runs.sort()
                judged += len(runs)
                behind += sum(1 for fire_time, started, _ in runs
                              if started - fire_time > LATE_LIMIT_SECONDS)
                for (fire_a, start_a, _), (fire_b, start_b, _) in zip(runs, runs[1:]):
                    if start_b < start_a:
                        wrong.append(f"{item.name}: the run for {format_time(fire_b)} started "
                                     f"before the run for {format_time(fire_a)}")
                for fire_time, started, _ended in runs:
                    active = sum(1 for _f, begin, end in runs if begin <= started < end)
                    if active > limit:
                        wrong.append(f"{item.name}: {active} runs active when the run for "
                                     f"{format_time(fire_time)} started (limit {limit})")
        if wrong:
            return broken(f"{len(wrong)} faults in the order of {judged} runs", wrong)
        if not judged:
            return unknown("no run of a schedule that catches up is in the window yet")
        return holds(f"{judged} runs in the order of their fire times, within their limit; "
                     f"{behind} of them started late, from a backlog")
