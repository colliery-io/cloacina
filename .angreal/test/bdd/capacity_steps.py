"""Step definitions of the "Capacity" chapter (CLOACINA-T-1114).

Team A fills its own agents and the steps watch what the orchestrator does
with the work that has no room: it waits, it runs when room comes, in
priority order, and a task that names a pool and a size runs only where it
fits. The checker's rules (`suite/rules/capacity.py`) judge the same
promises on every pass, against the bursts of the standing workload.

Channels that are not the public API, and are the harness's own:

- the task rows, the attempt errors and the agent roster are read from
  postgres (`world.psql`): the API shows no agent and no wait of a task;
- the chapter's extra agents are copies of the tenant's agent Deployment,
  applied with `kubectl`, with the pool and the totals set by the agent's
  own environment (`CLOACINA_AGENT_POOL`, `CLOACINA_AGENT_CPU`,
  `CLOACINA_AGENT_MEMORY`);
- the metrics listener is read through `kubectl port-forward`.

A chapter's steps file names its chapter and its feature files; see
`chapters.py`.
"""

from __future__ import annotations

import copy
import json
import re
import socket
import subprocess
import time
import urllib.request

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import workload
from .suite.engine import parse_time
from .suite.rules import capacity as rules

steps = StepRegistry()

CHAPTER = "capacity"
FEATURES = ("features/capacity/capacity.feature",)

TEAM = workload.TENANTS[0]
OTHER_TEAMS = workload.TENANTS[1:]
# The slots of a tenant: its agents times the tasks each runs at once.
SLOTS = workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS
FIVE_MINUTE_JOBS = ("standing_rollup", "standing_extract", "standing_flaky")

SIZED_PACKAGE = "capacity-sized-py"
SIZED_WORKFLOW = "capacity_sized"
OVERSIZED_PACKAGE = "capacity-oversized-py"
OVERSIZED_WORKFLOW = "capacity_oversized"
POOL = "sized"
# The chapter's agent of the pool: 1 CPU and 1Gi. A sized task asks for 400Mi,
# so two fit at once; the oversized task asks for 2Gi and fits only on the
# roomy agent, which reports 4Gi.
SIZED_AGENT = "cloacina-agent-sized"
ROOMY_AGENT = "cloacina-agent-roomy"
MIB = 1024 * 1024
SIZED_AGENT_MEMORY = 1024 * MIB
SIZED_TASK_MEMORY = 400 * MIB
ROOMY_AGENT_MEMORY = 4096 * MIB
SIZED_RUNS = 4

PRIORITY_INSTANCE = "capacity_priority"
UUID = re.compile(r"^[0-9a-fA-F-]{36}$")


# --- helpers ------------------------------------------------------------------

def _wait(seconds, check, every=5):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _rows(world, sql):
    """The rows of one SELECT in the deployment's postgres, as dictionaries."""
    raw = world.psql(f"SELECT coalesce(json_agg(q), '[]'::json) FROM ({sql}) q;")
    try:
        return json.loads(raw) if raw else []
    except json.JSONDecodeError as exc:
        raise AssertionError(f"postgres did not answer with JSON: {raw[:300]}") from exc


def _id_list(ids):
    for ident in ids:
        assert UUID.match(str(ident)), f"not an execution id: {ident!r}"
    return ", ".join(f"'{ident}'" for ident in ids)


def _execute(world, workflow, context, priority=None):
    """Start one run of a workflow of team A with its write key. Returns the answer."""
    body = {"context": context}
    if priority is not None:
        body["priority"] = priority
    _, answer = world.api("POST", f"/v1/tenants/{TEAM}/workflows/{workflow}/execute",
                          key=world.keys[TEAM], body=body, expect=(200, 201, 202))
    assert isinstance(answer, dict) and answer.get("execution_id"), f"no execution id: {answer}"
    return answer


def _start_holds(world, count, seconds, priority=None):
    answers = [_execute(world, workload.HOLD_WORKFLOW, {"hold_seconds": seconds}, priority)
               for _ in range(count)]
    world.scratch.setdefault("holds", []).extend(a["execution_id"] for a in answers)
    world.scratch.setdefault("answers", []).extend(answers)
    world.scratch.setdefault("burst_started", time.time())
    return answers


def _run_rows(world, ids, extra=""):
    """One row for each task of the runs `ids` of team A."""
    return _rows(world, (
        "SELECT we.id AS run, we.status AS run_status, we.priority AS priority, "
        "te.id AS task, te.status, te.attempt, te.pool, te.created_at, te.started_at, "
        "te.completed_at, te.fleet_agent_id, "
        f"(SELECT count(*) FROM {TEAM}.task_attempt_errors e WHERE e.task_execution_id = te.id) "
        f"AS attempt_errors{extra} "
        f"FROM {TEAM}.workflow_executions we "
        f"JOIN {TEAM}.task_executions te ON te.workflow_execution_id = we.id "
        f"LEFT JOIN {TEAM}.contexts c ON c.id = we.context_id "
        f"WHERE we.id IN ({_id_list(ids)}) ORDER BY te.created_at"))


def _wait_ended(world, ids, timeout_s):
    def pending():
        rows = _run_rows(world, ids)
        ended = {row["run"] for row in rows if row.get("run_status") in TERMINAL}
        return [ident for ident in ids if ident not in ended]

    _wait(timeout_s, lambda: not pending(), every=10)
    still = pending()
    assert not still, f"{len(still)} of {len(ids)} runs had not ended after {timeout_s} s"
    return _run_rows(world, ids)


def _drain_seconds(count, seconds):
    rounds = -(-count // SLOTS)
    return seconds * (rounds + 1) + 180


def _most_at_once(rows):
    """The most tasks of `rows` that ran at the same time."""
    edges = []
    for row in rows:
        start, end = parse_time(row.get("started_at")), parse_time(row.get("completed_at"))
        if start is not None and end is not None:
            edges.append((start, 1))
            edges.append((end, -1))
    most = now = 0
    for _stamp, step in sorted(edges, key=lambda edge: (edge[0], edge[1])):
        now += step
        most = max(most, now)
    return most


def _context(row):
    value = row.get("context")
    if isinstance(value, str):
        try:
            value = json.loads(value)
        except ValueError:
            value = {}
    return value if isinstance(value, dict) else {}


def _failures_since(world, since):
    """Each task and attempt of team A that failed since `since`, apart from
    the planned failures of the workload."""
    stamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(since))
    sql = rules.FAILURES_SQL.replace("{schema}", TEAM).replace(
        "now() - interval '3 hours'", f"'{stamp}'::timestamptz")
    return [row for row in _rows(world, sql) if rules.PLANNED_FAILURE not in str(row.get("error"))]


# -- agents --

def _set_agent_limit(world, count):
    world.api("POST", f"/v1/tenants/{TEAM}/limits", key=world.bootstrap_key,
              body={"max_agents": count}, expect=(200, 201))


def _scale_to(world, count):
    """Bring team A to `count` agents of its own Deployment, up or down."""
    if not world.chart_compilers:
        world.kubectl("scale", "deploy/cloacina-agent", "-n", world.tenant_namespace(TEAM),
                      f"--replicas={count}")
        return
    for _ in range(8):
        _, fleet = world.api("GET", f"/v1/tenants/{TEAM}/fleet", key=world.bootstrap_key,
                             expect=(200,))
        desired = int((fleet or {}).get("desired_count") or 0)
        if desired == count:
            return
        verb = "provision" if desired < count else "deprovision"
        world.api("POST", f"/v1/tenants/{TEAM}/fleet/{verb}", key=world.bootstrap_key,
                  expect=(200, 201))
    raise AssertionError(f"{TEAM} did not reach a desired count of {count} agents")


def _live_agents(world):
    return _rows(world, rules.LIVE_AGENTS_SQL + f" AND tenant_id = '{TEAM}'")


def _clone_agent(world, name, pool, cpu, memory_mib):
    """A second agent Deployment of team A, made from its own: the same image,
    key, hardening and network, with the pool and totals given here."""
    ns = world.tenant_namespace(TEAM)
    if world.kubectl_json("get", "deploy", name, "-n", ns):
        return
    source = world.kubectl_json("get", "deploy", "cloacina-agent", "-n", ns)
    assert source, f"{TEAM} has no agent Deployment to copy in {ns}"
    template = copy.deepcopy(source["spec"]["template"])
    labels = dict(template.get("metadata", {}).get("labels") or {})
    # Its own name, so the tenant's Deployment does not count its pods. The
    # other labels stay: the server's network policy admits the agent by them.
    labels["app.kubernetes.io/name"] = name
    template["metadata"] = {"labels": labels}
    container = template["spec"]["containers"][0]
    overrides = {"CLOACINA_AGENT_POOL": pool, "CLOACINA_AGENT_CPU": cpu,
                 "CLOACINA_AGENT_MEMORY": f"{memory_mib}Mi"}
    container["env"] = [item for item in container.get("env") or []
                        if item.get("name") not in overrides]
    container["env"] += [{"name": key, "value": value} for key, value in overrides.items()]
    manifest = {
        "apiVersion": "apps/v1", "kind": "Deployment",
        "metadata": {"name": name, "namespace": ns,
                     "labels": {"app.kubernetes.io/name": name,
                                "cloacina-bdd-chapter": CHAPTER}},
        "spec": {"replicas": 1, "selector": {"matchLabels": {"app.kubernetes.io/name": name}},
                 "template": template},
    }
    proc = subprocess.run(["kubectl", "apply", "-f", "-"], input=json.dumps(manifest),
                          env=world._kube_env(), text=True, capture_output=True, check=False)
    assert proc.returncode == 0, f"could not apply the agent {name}: {proc.stderr}"


def _remove_agent(world, name):
    world.kubectl("delete", "deploy", name, "-n", world.tenant_namespace(TEAM),
                  "--ignore-not-found=true", "--wait=false", check=False)


def _wait_pool_agent(world, memory_bytes, timeout_s=240):
    """The live agent of team A in the pool with this memory total."""
    def found():
        return [row for row in _live_agents(world)
                if rules.pool_of_agent(row.get("capabilities")) == POOL
                and int(row.get("memory_bytes") or 0) == memory_bytes]

    agents = _wait(timeout_s, found, every=5)
    assert agents, f"no live agent of {TEAM} in the pool {POOL} with {memory_bytes} bytes"
    return agents[0]


# -- metrics --

def _free_port():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def _metrics_text(world):
    """/metrics of a server replica, through a port-forward of the metrics Service."""
    port = _free_port()
    service = f"svc/{world.release}-cloacina-server-metrics"
    forward = subprocess.Popen(["kubectl", "port-forward", service, f"{port}:9090", "-n", world.ns],
                               env=world._kube_env(), stdout=subprocess.DEVNULL,
                               stderr=subprocess.DEVNULL)
    try:
        deadline = time.time() + 20
        while True:
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/metrics", timeout=5) as answer:
                    return answer.read().decode("utf-8", "replace")
            except OSError:
                if time.time() >= deadline:
                    raise AssertionError(f"no answer from {service} on port 9090 through a port-forward")
                time.sleep(1)
    finally:
        forward.terminate()


def gauge(text, name, labels):
    """The value of the series of `name` whose labels include `labels`, or None."""
    for line in text.splitlines():
        if not line.startswith(name + "{"):
            continue
        head, _, value = line.rpartition(" ")
        found = dict(re.findall(r'(\w+)="([^"]*)"', head))
        if all(found.get(key) == str(wanted) for key, wanted in labels.items()):
            try:
                return float(value)
            except ValueError:
                return None
    return None


# --- Background ---------------------------------------------------------------

@steps.given("the standing workload is in place for each team")
def bg_workload(world):
    world.ensure_workload()
    counts = world.agent_counts()
    assert counts.get(TEAM, 0) >= workload.AGENTS_PER_TENANT, (
        f"{TEAM} has {counts.get(TEAM, 0)} agents, the chapter needs {workload.AGENTS_PER_TENANT}")


# --- Work waits, and runs when room comes -------------------------------------

@steps.when("team A starts more holds at once than its agents can take")
def w_burst(world):
    _start_holds(world, workload.HOLD_BURST, workload.HOLD_SECONDS)
    world.note(f"{TEAM}: started {workload.HOLD_BURST} holds of {workload.HOLD_SECONDS} s "
               f"on {SLOTS} slots")


@steps.then("some of team A's holds wait for a free agent")
def t_some_wait(world):
    time.sleep(30)
    rows = _run_rows(world, world.scratch["holds"])
    waiting = [row for row in rows if row.get("status") == "Ready" and not row.get("started_at")]
    running = [row for row in rows if row.get("started_at")]
    world.note(f"30 s after the burst: {len(running)} holds started, {len(waiting)} wait")
    assert waiting, f"no hold waits: {len(running)} of {len(rows)} started at once"


@steps.then("no task of team A fails while its agents are full")
def t_no_failure(world):
    ids = world.scratch["holds"]
    _wait_ended(world, ids, _drain_seconds(len(ids), workload.HOLD_SECONDS))
    failed = _failures_since(world, world.scratch["burst_started"] - 5)
    for row in failed:
        world.note(f"failed: {row.get('kind')} {row.get('workflow')}/{row.get('task')} at "
                   f"{row.get('at')}: {str(row.get('error'))[:200]}")
    assert not failed, f"{len(failed)} tasks or attempts of {TEAM} failed while it was full"


@steps.then("every hold of team A completed")
def t_completed(world):
    ids = world.scratch["holds"]
    rows = _wait_ended(world, ids, _drain_seconds(len(ids), workload.HOLD_SECONDS))
    wrong = [f"{row['run']} is {row.get('run_status')}" for row in rows
             if row.get("run_status") != "Completed"]
    assert not wrong, "; ".join(wrong)


@steps.then("each hold of team A ran one time")
def t_once(world):
    rows = _run_rows(world, world.scratch["holds"])
    wrong = [f"{row['run']}: attempt {row.get('attempt')}, {row.get('attempt_errors')} failed attempts"
             for row in rows if int(row.get("attempt") or 0) != 1 or int(row.get("attempt_errors") or 0)]
    waited = [row for row in rows if (parse_time(row.get("started_at")) or 0)
              - (parse_time(row.get("created_at")) or 0) > rules.WAIT_SECONDS]
    world.note(f"{len(rows)} holds, {len(waited)} waited for an agent, each with one attempt"
               if not wrong else "; ".join(wrong))
    assert not wrong, "; ".join(wrong)


@steps.given("team A may have one agent more than it has")
def g_limit_up(world):
    _set_agent_limit(world, workload.MAX_AGENTS_PER_TENANT + 1)


@steps.when("team A gets one more agent")
def w_more_agent(world):
    _scale_to(world, workload.AGENTS_PER_TENANT + 1)
    have = _wait(240, lambda: len(_live_agents(world)) > workload.AGENTS_PER_TENANT, every=5)
    assert have, f"{TEAM} did not get a live agent more within 240 s"
    world.note(f"{TEAM} has {len(_live_agents(world))} live agents")


@steps.then("more holds of team A ran at once than its first agents have slots")
def t_more_at_once(world):
    ids = world.scratch["holds"]
    rows = _wait_ended(world, ids, _drain_seconds(len(ids), workload.HOLD_SECONDS))
    most = _most_at_once(rows)
    world.note(f"at most {most} holds ran at once; the first agents have {SLOTS} slots")
    assert most > SLOTS, f"at most {most} holds ran at once: the new agent took none"


@steps.then("team A goes back to its own number of agents")
def t_limit_back(world):
    _scale_to(world, workload.AGENTS_PER_TENANT)
    _set_agent_limit(world, workload.MAX_AGENTS_PER_TENANT)


# --- A full team does not slow another team -----------------------------------

@steps.when("team A's agents are full across a fire time of the five-minute jobs")
def w_full_across_fire(world):
    now = time.time()
    fire = (int(now) // workload.SLOT_SECONDS + 1) * workload.SLOT_SECONDS
    if fire - now < 30:
        fire += workload.SLOT_SECONDS
    world.note(f"waiting {int(fire - 20 - now)} s, to fill {TEAM} 20 s before the fire time")
    time.sleep(max(0.0, fire - 20 - now))
    world.scratch["fire"] = fire
    _start_holds(world, workload.HOLD_BURST, workload.HOLD_SECONDS)


@steps.then("the five-minute jobs of team B and team C started on time at that fire time")
def t_others_on_time(world):
    fire = world.scratch["fire"]
    stamp = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(fire))
    time.sleep(max(0.0, fire + rules.ON_TIME_SECONDS + 30 - time.time()))
    still_full = _run_rows(world, world.scratch["holds"])
    waiting = [row for row in still_full if not row.get("started_at")
               or parse_time(row["started_at"]) > fire]
    world.note(f"holds of {TEAM} that had not started at the fire time: {len(waiting)}")
    assert waiting, f"{TEAM} was not full at the fire time: the scenario judges nothing"
    wrong = []
    for tenant in OTHER_TEAMS:
        sql = rules.SCHEDULED_RUNS_SQL.replace("{schema}", tenant).replace(
            "se.scheduled_time > now() - interval '3 hours'", f"se.scheduled_time = '{stamp}'::timestamptz")
        rows = {row.get("workflow"): row for row in _rows(world, sql)}
        for workflow in FIVE_MINUTE_JOBS:
            row = rows.get(workflow)
            if row is None:
                wrong.append(f"{tenant}/{workflow}: no fire at {stamp}")
                continue
            first = parse_time(row.get("first_task_started"))
            late = None if first is None else int(first - fire)
            world.note(f"{tenant}/{workflow}: first task {late} s after the fire time")
            if late is None or late > rules.ON_TIME_SECONDS:
                wrong.append(f"{tenant}/{workflow}: first task {late} s after {stamp}")
    assert not wrong, "; ".join(wrong)


# --- A task lands on an agent that fits it ------------------------------------

@steps.given('team A has an agent in the pool "sized" with 1 CPU and 1Gi of memory')
def g_sized_agent(world):
    _clone_agent(world, SIZED_AGENT, POOL, "1", SIZED_AGENT_MEMORY // MIB)
    agent = _wait_pool_agent(world, SIZED_AGENT_MEMORY)
    world.note(f"agent of the pool {POOL}: {agent.get('agent_id')}, {agent.get('cpu_millis')}m, "
               f"{agent.get('memory_bytes')} bytes, {agent.get('max_concurrency')} slots")


@steps.given("team A has published the packages of sized tasks")
def g_sized_packages(world):
    for package in (SIZED_PACKAGE, OVERSIZED_PACKAGE):
        world.ensure_version(TEAM, package, package, "0.1.0", key=world.keys[TEAM])


@steps.when("team A starts four runs of the sized task at once")
def w_sized_runs(world):
    answers = [_execute(world, SIZED_WORKFLOW, {"hold_seconds": 30}) for _ in range(SIZED_RUNS)]
    world.scratch["sized"] = [answer["execution_id"] for answer in answers]


@steps.then('each sized task ran on the agent of the pool "sized"')
def t_sized_where(world):
    ids = world.scratch["sized"]
    rows = _wait_ended(world, ids, 600)
    world.scratch["sized_rows"] = rows
    wrong = []
    for row in _run_rows(world, ids, extra=", c.value AS context"):
        host = (_context(row).get("capacity_sized_done") or {}).get("host") or ""
        if row.get("pool") != POOL:
            wrong.append(f"{row['run']}: the task row names the pool {row.get('pool')}")
        if not host.startswith(SIZED_AGENT + "-"):
            wrong.append(f"{row['run']}: ran on {host or 'no host on record'}")
    assert not wrong, "; ".join(wrong)


@steps.then("the sized agent never held more sized tasks than its memory has room for")
def t_sized_room(world):
    room = SIZED_AGENT_MEMORY // SIZED_TASK_MEMORY
    most = _most_at_once(world.scratch["sized_rows"])
    world.note(f"at most {most} sized tasks ran at once; the agent has room for {room}")
    assert 1 <= most <= room, f"{most} sized tasks ran at once on an agent with room for {room}"


@steps.then("every sized run of team A completed")
def t_sized_completed(world):
    wrong = [f"{row['run']} is {row.get('run_status')}" for row in world.scratch["sized_rows"]
             if row.get("run_status") != "Completed"]
    assert not wrong, "; ".join(wrong)


@steps.when("team A starts a run of a task larger than any agent of its pool")
def w_oversized(world):
    answer = _execute(world, OVERSIZED_WORKFLOW, {})
    world.scratch["oversized"] = [answer["execution_id"]]
    world.scratch["oversized_at"] = time.time()


@steps.when(r"(\d+) minutes? pass(?:es)?")
def w_minutes(world, minutes):
    time.sleep(60 * int(minutes))


@steps.then("the task waits, and has not failed")
def t_oversized_waits(world):
    rows = _run_rows(world, world.scratch["oversized"])
    assert rows, "the run has no task row"
    row = rows[0]
    waited = int(time.time() - world.scratch["oversized_at"])
    world.note(f"after {waited} s: run {row.get('run_status')}, task {row.get('status')}, "
               f"attempt {row.get('attempt')}, started {row.get('started_at')}")
    assert row.get("run_status") not in TERMINAL, f"the run is {row.get('run_status')}"
    assert row.get("status") == "Ready" and not row.get("started_at"), (
        f"the task is {row.get('status')}, started {row.get('started_at')}")
    assert int(row.get("attempt") or 0) == 1 and not int(row.get("attempt_errors") or 0), (
        f"the task used attempt {row.get('attempt')}, {row.get('attempt_errors')} failed")


@steps.then("the task's own view says why it waits, naming its pool and its request")
def t_oversized_reason(world):
    execution = world.scratch["oversized"][0]
    _, body = world.api("GET", f"/v1/tenants/{TEAM}/executions/{execution}/tasks",
                        key=world.keys[TEAM], expect=(200,))
    tasks = (body or {}).get("tasks") or []
    assert tasks, "the task view lists no task"
    # CLOACINA-T-1148: `wait_reason` (a code and a message) while it waits.
    why = tasks[0].get("wait_reason") or {}
    said = str(why.get("message") or "")
    world.note(f"the task view says: {why.get('code') or 'no code'}: {said or 'nothing'}")
    assert why.get("code") == "too_large", f"the wait reason is {why or 'not set'}"
    assert POOL in said and ("2Gi" in said or "2147483648" in said or "memory" in said), (
        "the wait reason does not name the pool and the request")


@steps.then("the task runs once an agent of the pool with room for it is added")
def t_oversized_runs(world):
    _clone_agent(world, ROOMY_AGENT, POOL, "1", ROOMY_AGENT_MEMORY // MIB)
    try:
        _wait_pool_agent(world, ROOMY_AGENT_MEMORY)
        rows = _wait_ended(world, world.scratch["oversized"], 300)
        row = _run_rows(world, world.scratch["oversized"], extra=", c.value AS context")[0]
        host = (_context(row).get("capacity_oversized_done") or {}).get("host") or ""
        world.note(f"the run is {rows[0].get('run_status')}, on {host or 'no host on record'}")
        assert rows[0].get("run_status") == "Completed", f"the run is {rows[0].get('run_status')}"
        assert host.startswith(ROOMY_AGENT + "-"), f"the task ran on {host or 'no host on record'}"
    finally:
        _remove_agent(world, ROOMY_AGENT)


@steps.then('team A\'s agent of the pool "sized" is removed')
def t_remove_sized(world):
    _remove_agent(world, SIZED_AGENT)


# --- Priority -------------------------------------------------------------------

@steps.when("team A fills its agents with holds")
def w_fill(world):
    _start_holds(world, SLOTS, 90)
    time.sleep(10)


@steps.when("team A starts holds at priority 9, and then holds at priority 5")
def w_two_priorities(world):
    world.scratch["low"] = [a["execution_id"] for a in _start_holds(world, 10, 30, priority=9)]
    time.sleep(5)
    world.scratch["high"] = [a["execution_id"] for a in _start_holds(world, 10, 30, priority=5)]


@steps.then("each run got the priority it asked for")
def t_given(world):
    wrong = []
    for answer in world.scratch["answers"]:
        grant = answer.get("priority") or {}
        if grant and grant.get("given") != grant.get("asked"):
            wrong.append(f"{answer['execution_id']}: asked {grant.get('asked')}, "
                         f"given {grant.get('given')}: {answer.get('priority_notice')}")
    given = sorted({(answer.get("priority") or {}).get("given") for answer in world.scratch["answers"]},
                   key=str)
    world.note(f"priorities given: {given}")
    assert not wrong, "; ".join(wrong)


@steps.then("the waiting gauge of team A counts its waiting holds at priority 9")
def t_gauge(world):
    value = gauge(_metrics_text(world), "cloacina_waiting_tasks",
                  {"tenant": TEAM, "pool": rules.DEFAULT_POOL, "priority": 9})
    world.note(f'cloacina_waiting_tasks{{tenant="{TEAM}", pool="default", priority="9"}} = {value}')
    assert value is not None and value >= 1, f"the gauge is {value}"


@steps.then("no hold at priority 9 started while a hold at priority 5 still waited")
def t_order(world):
    ids = world.scratch["holds"]
    rows = _wait_ended(world, ids, _drain_seconds(len(ids), 90))
    found = rules.priority_inversions(rows)
    high = [parse_time(r.get("started_at")) for r in rows if r["run"] in world.scratch["high"]]
    low = [parse_time(r.get("started_at")) for r in rows if r["run"] in world.scratch["low"]]
    if high and low and None not in high and None not in low:
        world.note(f"priority 5 started {int(min(high) - min(low))} s to {int(max(high) - min(low))} s "
                   f"after the first priority 9 start")
    for line in found[:10]:
        world.note(line)
    assert not found, f"{len(found)} holds started out of priority order"


@steps.when("team A makes a schedule of the hold job that fires each minute at priority 7")
def w_priority_schedule(world):
    world.scratch["schedule_made"] = time.time()
    world.api("POST", f"/v1/tenants/{TEAM}/workflows/{workload.HOLD_WORKFLOW}/instances",
              key=world.keys[TEAM], expect=(200, 201),
              body={"instance_name": PRIORITY_INSTANCE, "cron": "* * * * *", "priority": 7})


@steps.then("the first run of that schedule is at priority 7")
def t_schedule_priority(world):
    since = world.scratch["schedule_made"]

    def fired():
        return [run for run in world.runs(TEAM, workload.HOLD_WORKFLOW)
                if run.get("trigger_origin") != "manual"
                and (parse_time(run.get("started_at")) or 0) >= since]

    runs = _wait(150, fired, every=10)
    assert runs, "the schedule started no run within 150 s"
    grant = runs[-1].get("priority") or {}
    world.note(f"the first fire runs at priority {grant}")
    assert grant.get("given") == 7, f"the first fire runs at {grant}"


@steps.then("team A removes the schedule")
def t_remove_schedule(world):
    code, _ = world.api("DELETE", f"/v1/tenants/{TEAM}/workflows/{workload.HOLD_WORKFLOW}/instances/"
                        f"{PRIORITY_INSTANCE}", key=world.keys[TEAM], expect=None)
    assert code in (200, 204, 404), f"removing the schedule answered {code}"
