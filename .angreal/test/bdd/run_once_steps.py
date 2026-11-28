"""Step definitions of the chapter "Run once" (CLOACINA-T-1110).

Each scenario starts its own runs by hand on team A, with a tag in the
context (`run_once_tag`), so the marks the task bodies print name the run.
The marks are read from the agent pods' output with `kubectl logs`; the
attempts and the status come from the API. The line of a server replica that
stored a result for the replica that dispatched the task is read from the
server pods' output: the API has no field for it.

The world is the standing workload's (`standing_world.StandingWorld`).
"""

from __future__ import annotations

import json
import time
import uuid

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import workload
from .suite.rules.run_once import RESULT_STORED, TASK_ID, parse_mark
from .suite.sources import parse_log_lines

steps = StepRegistry()

CHAPTER = "run_once"
FEATURES = ("features/run_once/run_once.feature",)

TENANT = "team_a"
LONG_WORKFLOW = "standing_archive"
LONG_HOLD_SECONDS = 360
LONG_MIN_SECONDS = 300
# The slots of a tenant: its agents times the tasks each runs at once.
SLOTS = workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS
# Longer than the wait for every slot to be taken (300 s) and the check after.
FILL_HOLD_SECONDS = 420
WAIT_CHECK_SECONDS = 45
SHORT_HOLD_SECONDS = 20
SHORT_BATCH = 4
SHORT_ROUNDS = 6


def _wait(seconds, check, every=5):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _start(world, workflow, **context):
    """Start a run by hand with a new tag. Returns (execution id, tag)."""
    tag = f"run-once-{uuid.uuid4()}"
    context[workload.RUN_TAG_KEY] = tag
    _, body = world.api("POST", f"/v1/tenants/{TENANT}/workflows/{workflow}/execute",
                        key=world.keys[TENANT], body={"context": context}, expect=(200, 201, 202))
    execution = body.get("execution_id") if isinstance(body, dict) else None
    assert execution, f"starting {workflow} gave no execution id: {str(body)[:200]}"
    return execution, tag


def _status(world, execution):
    code, body = world.api("GET", f"/v1/tenants/{TENANT}/executions/{execution}",
                           key=world.keys[TENANT], expect=None)
    return body.get("status") if code == 200 and isinstance(body, dict) else None


def _tasks(world, execution):
    """The task rows of a run, from the API."""
    code, body = world.api("GET", f"/v1/tenants/{TENANT}/executions/{execution}/tasks",
                           key=world.keys[TENANT], expect=None)
    if code != 200:
        return []
    if isinstance(body, list):
        return body
    if isinstance(body, dict):
        return body.get("items") or body.get("tasks") or []
    return []


def _wait_ended(world, executions, seconds):
    """Wait until every run has ended. Returns execution id -> status."""
    def statuses():
        return {execution: _status(world, execution) for execution in executions}

    _wait(seconds, lambda: all(s in TERMINAL for s in statuses().values()), every=10)
    return statuses()


def _marks(world, tags=None):
    """tag -> the marks of team A's agents that carry it: [(pod, mark)]."""
    proc = world.kubectl("logs", "-n", workload.agent_namespace(TENANT), "-l",
                         workload.AGENT_SELECTOR, "--all-containers=true", "--prefix",
                         "--timestamps", "--tail=-1", "--since=3h", "--max-log-requests=20",
                         check=False)
    found = {}
    for row in parse_log_lines(proc.stdout or "", workload.MARK.strip()):
        mark = parse_mark(row["text"])
        tag = (mark or {}).get(workload.RUN_TAG_KEY)
        if tag and (tags is None or tag in tags):
            found.setdefault(tag, []).append((row["pod"], mark))
    return found


def _ready_server_pods(world):
    data = world.kubectl_json("get", "pods", "-n", world.ns, "-l", workload.SERVER_SELECTOR)
    ready = []
    for pod in (data or {}).get("items", []):
        statuses = pod.get("status", {}).get("containerStatuses", [])
        if statuses and all(status.get("ready") for status in statuses):
            ready.append(pod["metadata"]["name"])
    return ready


def _stored_elsewhere(world):
    """task id -> the server pod that stored its result for another replica."""
    proc = world.kubectl("logs", "-n", world.ns, "-l", workload.SERVER_SELECTOR,
                         "--all-containers=true", "--prefix", "--timestamps", "--tail=-1",
                         "--since=1h", check=False)
    found = {}
    for row in parse_log_lines(proc.stdout or "", RESULT_STORED):
        match = TASK_ID.search(row["text"])
        if match:
            found.setdefault(match.group(1).lower(), row["pod"])
    return found


def _one_start_each(world, tags):
    marks = _marks(world, set(tags))
    wrong = []
    for tag in tags:
        seen = marks.get(tag, [])
        world.note(f"{tag}: {len(seen)} start(s) on {', '.join(sorted({p for p, _ in seen})) or 'none'}")
        if len(seen) != 1:
            wrong.append(f"{tag}: the body started {len(seen)} times")
    assert not wrong, "; ".join(wrong)


# --- Background ---------------------------------------------------------------

@steps.given("a shared orchestrator runs in Kubernetes with two server replicas")
def bg_two_replicas(world):
    ready = _ready_server_pods(world)
    world.note(f"ready server replicas: {', '.join(ready) or 'none'}")
    assert len(ready) >= workload.SERVER_REPLICAS, (
        f"{len(ready)} ready server replica(s); the chapter needs {workload.SERVER_REPLICAS}")


@steps.given("team A has the packages of the standing workload in service")
def bg_packages(world):
    world.publish_workload(TENANT)


# --- Scenario 1: a long task --------------------------------------------------

@steps.when("team A starts the long job by hand, holding for six minutes")
def s1_start(world):
    execution, tag = _start(world, LONG_WORKFLOW, hold_seconds=LONG_HOLD_SECONDS)
    world.scratch["long"] = (execution, tag)
    world.note(f"long job run {execution}, tag {tag}")


@steps.then("the run of the long job completes")
def s1_completes(world):
    execution, _tag = world.scratch["long"]
    status = _wait_ended(world, [execution], LONG_HOLD_SECONDS + 600)[execution]
    assert status == "Completed", f"the long job ended {status}"


@steps.then("the server recorded one attempt of the long task")
def s1_one_attempt(world):
    execution, _tag = world.scratch["long"]
    tasks = _tasks(world, execution)
    world.note("tasks: " + ", ".join(f"{t.get('task_name')} attempt {t.get('attempt')} "
                                     f"{t.get('status')}" for t in tasks))
    assert len(tasks) == 1, f"{len(tasks)} task rows"
    assert tasks[0].get("attempt") == 1, f"the server recorded attempt {tasks[0].get('attempt')}"


@steps.then("the body of the long task started one time")
def s1_one_start(world):
    _one_start_each(world, [world.scratch["long"][1]])


@steps.then("the long task held for more than five minutes")
def s1_held(world):
    execution, _tag = world.scratch["long"]
    done = world.execution_context(TENANT, execution).get("standing_archive_done") or {}
    world.note(f"held {done.get('held_seconds')} s on {done.get('host')}")
    held = done.get("held_seconds")
    assert held is not None and float(held) > LONG_MIN_SECONDS, f"the task held {held} s"


# --- Scenario 2: no free agent ------------------------------------------------

@steps.given("every agent slot of team A is taken by a held task")
def s2_fill(world):
    runs = [_start(world, workload.HOLD_WORKFLOW, hold_seconds=FILL_HOLD_SECONDS)
            for _ in range(SLOTS)]
    world.scratch["fill"] = runs
    tags = {tag for _execution, tag in runs}
    # Other work of the tenant (the scheduled jobs, the self-driving
    # workflow's holds) can hold slots for a while.
    _wait(300, lambda: len(_marks(world, tags)) >= SLOTS, every=10)
    started = len(_marks(world, tags))
    world.note(f"{started} of {SLOTS} held tasks started")
    assert started >= SLOTS, f"{started} of {SLOTS} held tasks started; the slots are not all taken"


@steps.when("team A starts two more held tasks")
def s2_more(world):
    world.scratch["more"] = [_start(world, workload.HOLD_WORKFLOW, hold_seconds=SHORT_HOLD_SECONDS)
                             for _ in range(2)]
    world.scratch["more_started"] = time.time()


@steps.then("those two tasks wait for a free agent")
def s2_wait(world):
    time.sleep(max(0.0, world.scratch["more_started"] + WAIT_CHECK_SECONDS - time.time()))
    tags = {tag for _execution, tag in world.scratch["more"]}
    early = _marks(world, tags)
    statuses = {execution: _status(world, execution) for execution, _tag in world.scratch["more"]}
    world.note(f"after {WAIT_CHECK_SECONDS} s: {len(early)} started; statuses {statuses}")
    assert not early, f"{len(early)} of the two tasks started while every slot was taken"
    assert not any(s in TERMINAL for s in statuses.values()), f"a waiting run ended: {statuses}"


@steps.then("each of those two tasks completes once an agent is free")
def s2_complete(world):
    executions = [execution for execution, _tag in world.scratch["more"]]
    statuses = _wait_ended(world, executions, FILL_HOLD_SECONDS + 600)
    wrong = {e: s for e, s in statuses.items() if s != "Completed"}
    assert not wrong, f"runs that did not complete: {wrong}"


@steps.then("the body of each of those two tasks started one time")
def s2_one_start(world):
    _one_start_each(world, [tag for _execution, tag in world.scratch["more"]])


@steps.then("neither of those two tasks used up an attempt while it waited")
def s2_attempts(world):
    wrong = []
    for execution, _tag in world.scratch["more"]:
        for task in _tasks(world, execution):
            if task.get("attempt") != 1:
                wrong.append(f"{execution} {task.get('task_name')}: attempt {task.get('attempt')}")
    assert not wrong, "; ".join(wrong)


# --- Scenario 3: the other replica --------------------------------------------

@steps.when("team A runs short held tasks until a result reaches the replica that did not "
            "dispatch its task")
def s3_until(world):
    for round_ in range(1, SHORT_ROUNDS + 1):
        runs = [_start(world, workload.HOLD_WORKFLOW, hold_seconds=SHORT_HOLD_SECONDS)
                for _ in range(SHORT_BATCH)]
        _wait_ended(world, [execution for execution, _tag in runs], 600)
        stored = _stored_elsewhere(world)
        for execution, tag in runs:
            for task in _tasks(world, execution):
                pod = stored.get(str(task.get("id")).lower())
                if pod:
                    world.scratch["elsewhere"] = (execution, tag, task, pod)
                    world.note(f"round {round_}: the result of task {task.get('id')} was stored "
                               f"by {pod} for the replica that dispatched it")
                    return
    raise AssertionError(f"in {SHORT_ROUNDS} rounds of {SHORT_BATCH} runs, no result reached "
                         "a replica that did not dispatch its task")


@steps.then("a server replica stored that task's result for the replica that dispatched it")
def s3_stored(world):
    assert world.scratch.get("elsewhere"), "no result reached the other replica"


@steps.then("that task completed")
def s3_completed(world):
    execution, _tag, task, _pod = world.scratch["elsewhere"]
    now = {t.get("id"): t for t in _tasks(world, execution)}.get(task.get("id")) or task
    assert now.get("status") == "Completed", f"the task is {now.get('status')}"
    assert _status(world, execution) == "Completed", f"the run is {_status(world, execution)}"


@steps.then("the body of that task started one time")
def s3_one_start(world):
    _one_start_each(world, [world.scratch["elsewhere"][1]])


@steps.then("no stored result of that task is left untaken")
def s3_inbox(world):
    _execution, _tag, task, _pod = world.scratch["elsewhere"]
    task_id = str(task.get("id"))
    # Read from postgres: the API has no view of the inbox.
    counts = {}
    for schema in (TENANT, "public"):
        raw = world.psql(f"SELECT count(*) FROM {schema}.fleet_result_inbox "
                         f"WHERE task_execution_id = '{task_id}';")
        counts[schema] = raw
    world.note("inbox rows of the task: " + json.dumps(counts))
    left = {schema: raw for schema, raw in counts.items() if raw not in ("0", "")}
    assert not left, f"stored results of the task are left: {left}"
