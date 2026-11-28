"""Rules of the Run once row (CLOACINA-T-1110).

"A task runs one time" needs a count the server does not keep. Each task body
of the workload prints one mark when it starts (`workload.MARK`), and the
checker reads those marks from the agent pods' output and keeps them. These
rules compare, for each task that has ended, the number of its marks with the
number of attempts the server recorded for it. More marks than attempts is a
task that ran again; no mark is a task that never ran.

A mark names its run: a scheduled run by `schedule_id` and `scheduled_time`,
a run by hand by `run_once_tag`. A task of a run that has neither (a run by
hand with no tag) is not counted.

The scenarios of `features/run_once/` show each promise once, step by step.
These rules judge them on every pass, for every tenant.
"""

from __future__ import annotations

import json
import re

from .. import workload
from ..engine import BROKEN, broken, holds, parse_time, unknown
from . import _shared

CHAPTER = "run_once"

LONG_WORKFLOW = "standing_archive"
LONG_TASK = "archive"
# The long task holds for six minutes; the promise is about a task that runs
# longer than five, past the push of work and the heartbeat periods.
LONG_TASK_MIN_SECONDS = 300

# The first task of each workflow the row reads: a task that waits for an
# agent waits from its row's creation, not from the end of a task before it.
FIRST_TASKS = {
    "standing_archive": "archive",
    "standing_hold": "hold",
    "standing_rollup": "read_interval",
    "standing_extract": "extract",
}
WORKFLOWS = tuple(sorted(FIRST_TASKS))
# A first task that started this long after its row was made waited for an agent.
WAIT_SECONDS = 60
# A task is judged this long after it ended, so the marks of its last start
# have been read by a pass.
SETTLE_SECONDS = 150
# A result in the inbox this old was not taken by the replica that waits.
INBOX_STALE_SECONDS = 600
TERMINAL = ("Completed", "Failed")

RESULT_STORED = "stored for the replica that does"
TASK_ID = re.compile(r"task_id[\"']?\s*[=:]\s*[\"']?([0-9a-fA-F-]{36})")

_IN = ", ".join(f"'{name}'" for name in WORKFLOWS)
# The tasks of the row's workflows of the last 170 minutes (the marks are kept
# for three hours), with what names their run and what the long task wrote.
TASKS_SQL = (
    "SELECT te.id AS task_id, we.id AS run_id, we.workflow_name, te.task_name, te.status, "
    "te.attempt, te.created_at, te.started_at, te.completed_at, te.waiting_since, "
    "c.value::jsonb ->> 'schedule_id' AS schedule_id, "
    "c.value::jsonb ->> 'scheduled_time' AS scheduled_time, "
    f"c.value::jsonb ->> '{workload.RUN_TAG_KEY}' AS run_once_tag, "
    "c.value::jsonb -> 'standing_archive_done' ->> 'held_seconds' AS held_seconds "
    "FROM {schema}.task_executions te "
    "JOIN {schema}.workflow_executions we ON we.id = te.workflow_execution_id "
    "LEFT JOIN {schema}.contexts c ON c.id = we.context_id "
    f"WHERE we.workflow_name IN ({_IN}) "
    "AND we.started_at > now() - interval '170 minutes' "
    "ORDER BY te.created_at"
)

# Results stored for another replica and not taken. The inbox of a tenant's
# schema, and the one in public for a server that writes there.
INBOX_SQL = (
    "SELECT task_execution_id, created_at FROM {schema}.fleet_result_inbox "
    f"WHERE created_at < now() - interval '{INBOX_STALE_SECONDS} seconds'"
)
PUBLIC_INBOX_SQL = INBOX_SQL.replace("{schema}", "public")


def parse_mark(text):
    """The JSON object of a mark line, or None. The agent may put its own
    prefix before what the task printed."""
    at = text.find(workload.MARK) if isinstance(text, str) else -1
    if at < 0:
        return None
    try:
        value = json.loads(text[at + len(workload.MARK):].strip())
    except ValueError:
        return None
    return value if isinstance(value, dict) else None


def run_key(record):
    """What names a run, from a mark or a task row: the tag of a run by hand,
    or the schedule and the fire time. None when there is neither."""
    tag = record.get(workload.RUN_TAG_KEY)
    if tag:
        return f"tag {tag}"
    if record.get("schedule_id") and record.get("scheduled_time"):
        return f"schedule {record['schedule_id']} at {record['scheduled_time']}"
    return None


def marks_by_task(snapshot, tenant):
    """(run key, workflow, task) -> the kept mark lines of that task."""
    found = {}
    for row in snapshot.logs(tenant, "marks"):
        mark = parse_mark(row.get("text"))
        if mark is None:
            continue
        key = run_key(mark)
        if key is None:
            continue
        found.setdefault((key, mark.get("workflow"), mark.get("task")), []).append(row)
    return found


class Count:
    """One ended task, with its marks."""

    def __init__(self, tenant, row, marks):
        self.tenant = tenant
        self.row = row
        self.key = run_key(row)
        self.marks = marks
        self.starts = len(marks)
        self.attempts = max(1, int(row.get("attempt") or 1))
        self.pods = sorted({mark.get("pod") or "?" for mark in marks})

    @property
    def once(self):
        return self.starts == self.attempts

    def name(self):
        return (f"{self.tenant}/{self.row.get('workflow_name')} run {self.row.get('run_id')} "
                f"({self.key}), task {self.row.get('task_name')}")

    def fault(self):
        if self.starts == 0:
            return (f"{self.name()}: never ran (no mark), the server recorded "
                    f"{self.attempts} attempt(s) and {self.row.get('status')}")
        return (f"{self.name()}: its body started {self.starts} times on {', '.join(self.pods)}, "
                f"the server recorded {self.attempts} attempt(s) and {self.row.get('status')}")

    def span(self):
        begin = parse_time(self.row.get("created_at"))
        end = parse_time(self.row.get("completed_at")) or begin
        return begin, end

    def waited(self):
        """True when the task waited for an agent before its body started."""
        if self.row.get("waiting_since"):
            return True
        made = parse_time(self.row.get("created_at"))
        if made is None:
            return False
        first = min((parse_time(mark.get("at")) for mark in self.marks
                     if parse_time(mark.get("at")) is not None), default=None)
        started = first if first is not None else parse_time(self.row.get("started_at"))
        return started is None or started - made >= WAIT_SECONDS


def ended_tasks(snapshot, tenant, only=None):
    """The counts of the ended, settled tasks of a tenant that name their run."""
    now = _shared.clock(snapshot)
    marks = marks_by_task(snapshot, tenant)
    found = []
    for row in snapshot.db(tenant, "run_once_tasks"):
        if row.get("status") not in TERMINAL or run_key(row) is None:
            continue
        if only is not None and not only(row):
            continue
        ended = parse_time(row.get("completed_at"))
        if ended is None or now - ended < SETTLE_SECONDS:
            continue
        key = (run_key(row), row.get("workflow_name"), row.get("task_name"))
        found.append(Count(tenant, row, marks.get(key, [])))
    return found


def excused(snapshot, count):
    """A task whose agent was killed on purpose may start again on another
    agent, and the marks of a killed pod read after its death are lost."""
    begin, end = count.span()
    if begin is None:
        return False
    return any(window.overlaps(begin, end) for window in snapshot.windows("kill_agent"))


def judge_counts(snapshot, counts, what):
    """A verdict for counts of one promise."""
    if not counts:
        return unknown(f"no {what} has ended in the window yet")
    kept = sum(len(snapshot.logs(tenant, "marks")) for tenant in _shared.workload_tenants(snapshot))
    if not kept:
        # Not one mark of any task in any tenant: the marks are not read, or
        # not printed. That says nothing about the tasks.
        return unknown(f"no mark of any task is kept, for {len(counts)} ended {what}s: the "
                       "agents' output is not read, or the tasks print no mark")
    faults = []
    spared = 0
    for count in counts:
        if count.once:
            continue
        if excused(snapshot, count):
            spared += 1
            continue
        faults.append(count.fault())
    if faults:
        return broken(f"{len(faults)} of {len(counts)} {what}s did not run one time", faults)
    note = f" ({spared} next to a killed agent not counted)" if spared else ""
    return holds(f"{len(counts) - spared} {what}s each ran one time{note}")


def register(registry):
    _shared.need_clock(registry)
    registry.need_db("run_once_tasks", TASKS_SQL)
    registry.need_db("run_once_inbox", INBOX_SQL)
    registry.need_admin_db("run_once_public_inbox", PUBLIC_INBOX_SQL)
    registry.need_logs("marks", "agents", workload.MARK.strip())
    registry.need_logs("results_elsewhere", "server", RESULT_STORED)

    @registry.rule("run_once.long_task", "run_once.long_task", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each run of the long job held for more than five minutes, ended "
                         "completed, and its body started once for each attempt")
    def long_task(snapshot):
        counts = []
        short = []
        for tenant in _shared.workload_tenants(snapshot):
            for count in ended_tasks(snapshot, tenant,
                                     lambda row: row.get("workflow_name") == LONG_WORKFLOW
                                     and row.get("task_name") == LONG_TASK):
                counts.append(count)
                held = count.row.get("held_seconds")
                if count.row.get("status") != "Completed":
                    short.append(f"{count.name()}: ended {count.row.get('status')}")
                elif held is None or float(held) < LONG_TASK_MIN_SECONDS:
                    short.append(f"{count.name()}: held {held} s, not more than "
                                 f"{LONG_TASK_MIN_SECONDS}")
        verdict = judge_counts(snapshot, counts, "long task")
        if short and verdict.state != BROKEN:
            return broken(f"{len(short)} of {len(counts)} long tasks did not complete a long hold",
                          short)
        if short:
            verdict.details.extend(short)
        return verdict

    @registry.rule("run_once.no_free_agent", "run_once.no_free_agent", CHAPTER,
                   tolerates=("postgres_away",),
                   about="each task that waited for a free agent of its tenant then ran, and its "
                         "body started once for each attempt")
    def no_free_agent(snapshot):
        counts = []
        for tenant in _shared.workload_tenants(snapshot):
            for count in ended_tasks(snapshot, tenant,
                                     lambda row: FIRST_TASKS.get(row.get("workflow_name"))
                                     == row.get("task_name")):
                if count.waited():
                    counts.append(count)
        return judge_counts(snapshot, counts, "task that waited for an agent")

    @registry.rule("run_once.other_replica", "run_once.other_replica", CHAPTER,
                   tolerates=("postgres_away",),
                   about="with two server replicas, each task whose result reached the replica "
                         "that did not dispatch it completed, its body started once for each "
                         "attempt, and no stored result was left untaken")
    def other_replica(snapshot):
        replicas = snapshot.log_pods(None, "results_elsewhere")
        if len(replicas) < workload.SERVER_REPLICAS:
            return unknown(f"the server has {len(replicas)} ready replica(s); the promise needs "
                           f"{workload.SERVER_REPLICAS}")
        stored = {}
        for row in snapshot.logs(None, "results_elsewhere"):
            match = TASK_ID.search(row.get("text") or "")
            if match:
                stored.setdefault(match.group(1).lower(), row.get("pod"))
        counts = []
        rows = {}
        for tenant in _shared.workload_tenants(snapshot):
            for row in snapshot.db(tenant, "run_once_tasks"):
                rows[str(row.get("task_id")).lower()] = (tenant, row)
            counts.extend(count for count in ended_tasks(snapshot, tenant)
                          if str(count.row.get("task_id")).lower() in stored)
        left = []
        inbox = list(snapshot.db(None, "run_once_public_inbox"))
        for tenant in _shared.workload_tenants(snapshot):
            inbox.extend(snapshot.db(tenant, "run_once_inbox"))
        for item in inbox:
            task_id = str(item.get("task_execution_id")).lower()
            tenant, row = rows.get(task_id, (None, None))
            if row is not None and row.get("status") != "Completed":
                left.append(f"{tenant}/{row.get('workflow_name')} task {row.get('task_name')} "
                            f"({task_id}): a result stored at {item.get('created_at')} was not "
                            f"taken, the task is {row.get('status')}")
        verdict = judge_counts(snapshot, counts, "task whose result reached the other replica")
        if left:
            return broken(f"{len(left)} stored results were not taken", left + verdict.details)
        for count in counts:
            if count.row.get("status") != "Completed" and count.once:
                return broken(f"{count.name()}: its result reached the other replica and the "
                              f"task is {count.row.get('status')}", [count.name()])
        return verdict
