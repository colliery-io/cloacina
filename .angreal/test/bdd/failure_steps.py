"""Step definitions of the chapter "Failure" (CLOACINA-T-1112).

An agent dies, every server replica restarts, Postgres goes away: each
scenario asks the disturbance schedule's runner (CLOACINA-T-1109) for one
disturbance now, reads its window from the disturbance record, and judges
what happened to the work around it.

Channels that are not the public API, said where they are used:

- which agent pod runs a task: the `STANDING_MARK` line the held job prints
  when its body starts, read from the agent pods' logs. The held job is
  started with a token in its context, so the mark names this run;
- fire times and task counts: the tenant's schedule and run tables in
  Postgres, read with the queries of the chapter's checker rules.

A chapter's steps file names its chapter and its feature files; see
`chapters.py`.
"""

from __future__ import annotations

import json
import time
import uuid
from pathlib import Path

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import disturb, engine, workload
from .suite.rules import failure as rules

steps = StepRegistry()

CHAPTER = "failure"
FEATURES = ("features/failure/failure.feature",)

TEAMS = {"A": "team_a", "B": "team_b", "C": "team_c"}
# Longer than the agent's drain time (25 s), so a plain stop hands the task
# back, and long enough to be running when the disturbance comes.
HOLD_SECONDS = 300
MARK = "STANDING_MARK"


def _wait(seconds, check, every=10):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _call(world, method, path, key, body=None):
    """The API, with a failure as an answer: a server pod may be away."""
    try:
        return world.api(method, path, key=key, body=body)
    except Exception as exc:  # the forward is starting again, or the server is away
        return None, f"{exc.__class__.__name__}: {exc}"


def _runner(world):
    """The runner of the disturbance schedule, on the suite's own cluster."""
    runner = world.scratch.get("runner")
    if runner is None:
        report_dir = Path(world.root) / "bdd-standing-report"
        hostdir = Path(world.kubeconfig).parent
        runner = disturb.Runner(
            # Refuses any kubeconfig but the suite's own.
            disturb.Kube(str(world.kubeconfig)),
            disturb.Api(world.base_url),
            dict(world.keys),
            disturb.Record(str(report_dir / "disturbances.jsonl"),
                           str(report_dir / "disturbance-log.jsonl")),
            read_keys=dict(world.checker_keys) or None,
            stop_path=str(hostdir / "disturb.stop"),
            lock_path=str(hostdir / "disturb.lock"),
            out=world.note,
        )
        world.scratch["runner"] = runner
    return runner


def _psql_rows(world, sql):
    """The rows of a query, read in the deployment's Postgres."""
    raw = world.psql(f"SELECT coalesce(json_agg(q), '[]'::json) FROM ({sql}) q")
    try:
        return json.loads(raw) if raw else []
    except ValueError:
        raise AssertionError(f"the query did not answer with JSON: {raw[:200]}")


def _marks(world, tenant, token):
    """(pod, mark) for each start of the held job's task with this token,
    read from the logs of the tenant's running agent pods."""
    namespace = disturb.tenant_namespace(tenant)
    found = []
    for pod in _runner(world).agent_pods(tenant):
        proc = world.kubectl("logs", pod, "-n", namespace, "--tail=5000", check=False)
        for line in (proc.stdout or "").splitlines():
            line = line.strip()
            if not line.startswith(MARK):
                continue
            try:
                mark = json.loads(line[len(MARK):].strip())
            except ValueError:
                continue
            if mark.get("scheduled_time") == token:
                found.append((pod, mark))
    return found


def _window(world):
    """The last disturbance of the scenario, as the record holds it now."""
    entries = world.scratch.get("disturbances") or []
    assert entries, "no disturbance was done in this scenario"
    ids = {entry["id"] for entry in entries}
    current = {entry["id"]: entry for entry in _runner(world).record.entries() if entry.get("id") in ids}
    return [current.get(entry["id"], entry) for entry in entries]


def _closes(entries):
    windows = [engine.Window.from_dict(entry) for entry in entries]
    return max((w.closes or w.start) for w in windows if w is not None)


def _did(world, entry, what):
    assert entry is not None, (f"{what}: nothing was done (another disturbance holds the lock, or "
                               "the pod was gone); see bdd-standing-report/disturbance-log.jsonl")
    world.scratch.setdefault("disturbances", []).append(entry)
    world.note(f"{what}: {entry.get('id')}, {entry.get('outcome')}")


# --- Background ---------------------------------------------------------------

@steps.given("the standing workload runs for team A, team B and team C")
def bg_workload(world):
    world.ensure_workload()
    counts = world.agent_counts()
    short = [f"{t} {counts.get(t, 0)}" for t in world.tenants
             if counts.get(t, 0) < workload.AGENTS_PER_TENANT]
    assert not short, f"tenants with fewer than {workload.AGENTS_PER_TENANT} agents: {', '.join(short)}"


@steps.given("the disturbance schedule acts on the suite's own cluster only")
def bg_contained(world):
    world.note(disturb.verify_cluster(_runner(world).kube))


# --- the held job ---------------------------------------------------------------

@steps.given(r"team (A|B|C) has a held job of five minutes running on one of its agents")
def held_job(world, team):
    tenant = TEAMS[team]
    token = f"failure-{uuid.uuid4().hex[:12]}"
    code, body = _call(world, "POST", f"/v1/tenants/{tenant}/workflows/{workload.HOLD_WORKFLOW}/execute",
                       world.keys[tenant],
                       body={"context": {"hold_seconds": HOLD_SECONDS, "scheduled_time": token}})
    assert code in (200, 201, 202), f"starting the held job of {tenant} -> {code}: {str(body)[:200]}"
    execution = (body or {}).get("execution_id") if isinstance(body, dict) else None
    assert execution, f"no execution id in {str(body)[:200]}"
    world.scratch.update(tenant=tenant, token=token, execution=execution)
    marks = _wait(180, lambda: _marks(world, tenant, token))
    assert marks, f"the held job of {tenant} did not start on an agent within 180 s"
    world.scratch["pod"] = marks[0][0]
    world.note(f"{tenant}: held job {execution[:8]} runs on {marks[0][0]} (token {token})")


@steps.when("that agent's pod is killed with no time to stop")
def kill_agent(world):
    entry = _runner(world).run_now("kill_agent", tenant=world.scratch["tenant"], mode="force",
                                   pod=world.scratch["pod"])
    _did(world, entry, f"killed {world.scratch['pod']}")


@steps.when("that agent's pod is stopped the way Kubernetes stops a pod")
def stop_agent(world):
    entry = _runner(world).run_now("kill_agent", tenant=world.scratch["tenant"], mode="sigterm",
                                   pod=world.scratch["pod"])
    _did(world, entry, f"stopped {world.scratch['pod']}")


@steps.when("each server replica in turn is stopped with no time to stop")
def restart_every_replica(world):
    runner = _runner(world)
    pods = runner.server_pods()
    assert pods, "no running server pod"
    world.scratch["replicas"] = len(pods)
    for pod in pods:
        _did(world, runner.run_now("restart_server", mode="force", pod=pod), f"restarted {pod}")


@steps.then(r"the disturbance is on record with (the pod and )?its window")
def on_record(world, with_pod=None):
    for entry in _window(world):
        for field in ("start", "end", "settle_until"):
            assert entry.get(field), f"{entry.get('id')}: no {field} on record"
        if with_pod:
            assert entry.get("pod") == world.scratch.get("pod"), (
                f"{entry.get('id')}: the record names {entry.get('pod')}, the task ran on "
                f"{world.scratch.get('pod')}")
        world.note(f"on record: {entry.get('kind')} {entry.get('target')}, {entry.get('start')} to "
                   f"{entry.get('end')}, settles by {entry.get('settle_until')}")


@steps.then("each restart is on record with the number of server replicas")
def restarts_on_record(world):
    entries = _window(world)
    replicas = world.scratch.get("replicas")
    assert len(entries) == replicas, f"{len(entries)} restarts on record for {replicas} replicas"
    for entry in entries:
        assert entry.get("end") and entry.get("replicas"), f"{entry.get('id')}: no end or no replica count"
        assert str(entry.get("outcome", "")).startswith("pod deleted"), f"{entry.get('id')}: {entry.get('outcome')}"
    world.note(f"server replicas: {replicas}" + (" (one replica: the hardest case)" if replicas == 1 else ""))


@steps.then("the orchestrator is ready again")
def ready_again(world):
    ready = _wait(300, lambda: _call(world, "GET", "/ready", world.bootstrap_key)[0] == 200)
    assert ready, "the orchestrator is not ready 300 s after the disturbance"
    for entry in _window(world):
        if entry.get("kind") == "postgres_away":
            assert entry.get("recovered"), f"the runner says the deployment did not recover: {entry.get('outcome')}"


@steps.then("the held job's run ends Completed within the window")
def run_completed(world):
    tenant, execution = world.scratch["tenant"], world.scratch["execution"]
    closes = _closes(_window(world))

    def status():
        code, body = _call(world, "GET", f"/v1/tenants/{tenant}/executions/{execution}", world.keys[tenant])
        value = body.get("status") if code == 200 and isinstance(body, dict) else None
        return value if value in TERMINAL else None

    ended = _wait(max(60.0, closes - time.time()), status)
    world.note(f"{tenant}: held job {execution[:8]} ended {ended or 'not at all'}; the window "
               f"closed {engine.format_time(closes)}")
    assert ended == "Completed", f"the held job's run is {ended or 'still not ended'} after the window"


@steps.then("the held job's task started again on another agent")
def started_elsewhere(world):
    tenant, token, killed = world.scratch["tenant"], world.scratch["token"], world.scratch["pod"]
    marks = _marks(world, tenant, token)
    others = sorted({pod for pod, _ in marks if pod != killed})
    world.note(f"starts seen after the kill: {len(marks)}, on {', '.join(others) or 'no other pod'} "
               f"(the log of {killed} went with its pod)")
    assert others, f"no start of the held job's task on an agent other than {killed}"


@steps.then("the held job's task started once")
def started_once(world):
    tenant, token = world.scratch["tenant"], world.scratch["token"]
    marks = _marks(world, tenant, token)
    world.note(f"starts of the held job's task: {len(marks)} ({', '.join(pod for pod, _ in marks)})")
    assert len(marks) == 1, f"the held job's task started {len(marks)} times"


# --- Postgres away --------------------------------------------------------------

def _fires(world, tenant):
    return rules.schedules_from_rows(_psql_rows(world, rules.FIRES_SQL.replace("{schema}", tenant)))


def _postgres_window(world):
    entries = [e for e in _window(world) if e.get("kind") == "postgres_away"]
    assert entries, "no postgres_away in this scenario"
    return engine.Window.from_dict(entries[-1])


def _fire_faults_until_settled(world, tenant, keep):
    """Faults of the schedules that `keep` picks, read again until there is
    none or the window has closed. Returns (faults, what was seen)."""
    window = _postgres_window(world)

    def read():
        faults, seen = [], []
        for key, entry in sorted(_fires(world, tenant).items(), key=lambda item: str(item[0])):
            if keep(key, entry):
                found, lines = rules.fire_faults(tenant, key, entry, window)
                faults.extend(found)
                seen.extend(lines)
        return faults, seen

    faults, seen = read()
    deadline = window.closes or time.time()
    while faults and time.time() < deadline:
        time.sleep(30)
        faults, seen = read()
    return faults, seen


@steps.given(r"team (A|B|C)'s extract job has a schedule that runs every missed fire time, one at a time")
def in_order_schedule(world, team):
    tenant = TEAMS[team]
    world.scratch["tenant"] = tenant
    rows = _psql_rows(world, f"SELECT catchup_policy, max_active_runs FROM {tenant}.schedules "
                             "WHERE workflow_name = 'standing_extract' AND instance_name = 'in_order'")
    assert rows, f"{tenant} has no in_order schedule of standing_extract"
    assert rows[0].get("catchup_policy") == "run_all", f"the in_order schedule is {rows[0]}"
    world.note(f"{tenant}/standing_extract/in_order: {rows[0]}")


@steps.when("Postgres goes away for eleven minutes and comes back")
def postgres_away(world):
    entry = _runner(world).run_now("postgres_away")
    _did(world, entry, f"Postgres away {entry.get('away_seconds') if entry else '?'} s")


@steps.then(r"each fire time of the window has one run of team (A|B|C)'s in-order extract schedule")
def in_order_runs(world, team):
    tenant = TEAMS[team]
    faults, seen = _fire_faults_until_settled(
        world, tenant, lambda key, entry: key == ("standing_extract", "in_order"))
    for line in seen:
        world.note(line)
    assert seen, "the in-order schedule had no fire time in the window"
    assert not faults, "; ".join(faults)


@steps.then(r"each five-minute schedule of team (A|B|C) that skips missed fire times has one run for the window")
def skip_runs(world, team):
    tenant = TEAMS[team]
    faults, seen = _fire_faults_until_settled(
        world, tenant, lambda key, entry: entry["policy"] != "run_all"
        and rules.fire_times(entry["cron"], 0, 600) == [0.0, 300.0, 600.0])
    for line in seen:
        world.note(line)
    assert seen, "no five-minute skip schedule had a fire time in the window"
    assert not faults, "; ".join(faults)


@steps.then("no fire time of the window has two runs")
def none_doubled(world):
    tenant = world.scratch["tenant"]
    window = _postgres_window(world)
    doubled = []
    for key, entry in _fires(world, tenant).items():
        found, _seen = rules.fire_faults(tenant, key, entry, window)
        doubled.extend(fault for fault in found if "two runs" in fault)
    assert not doubled, "; ".join(doubled)


@steps.then(r"every run of team (A|B|C) that was active when Postgres went away has ended")
def active_runs_ended(world, team):
    tenant = TEAMS[team]
    window = _postgres_window(world)

    def faults():
        rows = _psql_rows(world, rules.RUNS_SQL.replace("{schema}", tenant))
        active = rules.active_at(rules.runs_from_rows(rows, tenant), window.start)
        world.scratch["active"] = active
        return [fault for fault in (rules.postgres_run_fault(run) for run in active) if fault]

    found = faults()
    while found and time.time() < (window.closes or 0):
        time.sleep(30)
        found = faults()
    for run in world.scratch.get("active", []):
        world.note(rules.describe(run))
    assert not found, "; ".join(found)
