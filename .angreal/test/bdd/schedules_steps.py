"""Step definitions of the chapter "Schedules" (CLOACINA-T-1111).

The fire times a step expects come from the cron expression, computed here
with `suite/cron.py`, not from the server. What a fire did is read from
postgres (`schedule_executions`, the run's context): the API lists a
schedule's next and last run, not each fire. The schedules a scenario makes
are workflow instances, made and removed through the API with the team's own
key.

The restart scenarios ask the runner of the disturbance schedule
(CLOACINA-T-1109) to delete every server pod at once; the runner writes the
window to the disturbance record of the checker. They then open a port
forward of their own to the new pods.

The world is the standing workload's (`standing_world.StandingWorld`).
"""

from __future__ import annotations

import atexit
import json
import subprocess
import time
import urllib.request
from pathlib import Path

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import disturb, workload
from .suite.cron import Cron, local_minute_for
from .suite.engine import format_time, parse_time
from .suite.rules.schedules import LATE_LIMIT_SECONDS

steps = StepRegistry()

CHAPTER = "schedules"
FEATURES = ("features/schedules/schedules.feature",)

FIVE_MINUTES = Cron(workload.FIVE_MINUTES)
SLOTS = workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS
EXTRACT = "standing_extract"
DATA_JOBS = ("standing_rollup", "standing_extract")
INTERVAL_KEYS = {"standing_rollup": "standing_rollup_done", "standing_extract": "standing_extract_loaded"}
BACKLOG_INSTANCE = "schedules_backlog"
TZ_INSTANCE = "schedules_tz"
TZ = "Asia/Kathmandu"
# A port of its own for the forward the restart scenarios open.
FORWARD_PORT = 18110
DISTURBANCES = ("bdd-standing-report", "disturbances.jsonl")


def _wait(seconds, check, every=5):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _rows(world, sql):
    """The rows of one query, as dictionaries (via postgres)."""
    raw = world.psql(f"SELECT coalesce(json_agg(q), '[]'::json) FROM ({sql}) q;")
    try:
        return json.loads(raw) if raw else []
    except json.JSONDecodeError:
        raise AssertionError(f"postgres did not answer with rows: {raw[:300]}") from None


def _five_minute_schedules(tenant):
    """The five-minute schedules of the workload that fire once for each
    fire time (`skip`), as (workflow, instance or None)."""
    return [(s.workflow, s.instance if s.kind == "instance" else None)
            for s in workload.SCHEDULES if s.cron == workload.FIVE_MINUTES and s.catchup != "run_all"]


def _fires(world, tenant, fire_time):
    """(workflow, instance) -> the fires of that fire time, with their runs."""
    rows = _rows(world, (
        "SELECT s.workflow_name, s.instance_name, se.scheduled_time, we.status, we.started_at, "
        "c.value::jsonb AS context "
        f"FROM {tenant}.schedule_executions se JOIN {tenant}.schedules s ON s.id = se.schedule_id "
        f"LEFT JOIN {tenant}.workflow_executions we ON we.id = se.workflow_execution_id "
        f"LEFT JOIN {tenant}.contexts c ON c.id = we.context_id "
        f"WHERE se.scheduled_time = '{format_time(fire_time)}'"))
    found = {}
    for row in rows:
        found.setdefault((row.get("workflow_name"), row.get("instance_name")), []).append(row)
    return found


def _next_fire(after_s=0):
    return FIVE_MINUTES.next_after(time.time() + after_s)


def _sleep_past(fire_time, extra):
    time.sleep(max(0.0, fire_time + extra - time.time()))


def _hold_all_slots(world, tenant, seconds):
    executions = []
    for _ in range(SLOTS):
        _, body = world.api("POST", f"/v1/tenants/{tenant}/workflows/{workload.HOLD_WORKFLOW}/execute",
                            key=world.keys[tenant],
                            body={"context": {"hold_seconds": seconds}},
                            expect=(200, 201, 202))
        executions.append(body.get("execution_id"))
    world.scratch["holds"] = (tenant, executions)
    world.note(f"{tenant}: {len(executions)} held tasks of {seconds} s started")


def _status(world, tenant, execution):
    code, body = world.api("GET", f"/v1/tenants/{tenant}/executions/{execution}",
                           key=world.keys[tenant], expect=None)
    return body.get("status") if code == 200 and isinstance(body, dict) else None


def _make_instance(world, tenant, name, body):
    path = f"/v1/tenants/{tenant}/workflows/{EXTRACT}/instances"
    world.api("DELETE", f"{path}/{name}", key=world.keys[tenant], expect=None)
    _, made = world.api("POST", path, key=world.keys[tenant],
                        body={"instance_name": name, "params": {}, **body}, expect=(200, 201))
    world.scratch["instance"] = (tenant, name, made.get("id"), time.time())
    world.note(f"{tenant}: made the schedule {EXTRACT}/{name} ({made.get('id')}): {body}")
    return made


def _instance_fires(world):
    tenant, _name, schedule_id, _made = world.scratch["instance"]
    return _rows(world, (
        "SELECT se.scheduled_time, we.status, we.started_at, we.completed_at, "
        "c.value::jsonb -> 'standing_extract_loaded' AS loaded "
        f"FROM {tenant}.schedule_executions se "
        f"LEFT JOIN {tenant}.workflow_executions we ON we.id = se.workflow_execution_id "
        f"LEFT JOIN {tenant}.contexts c ON c.id = we.context_id "
        f"WHERE se.schedule_id = '{schedule_id}' ORDER BY se.scheduled_time"))


# --- restarts -----------------------------------------------------------------

def _runner(world):
    """The runner of the disturbance schedule (CLOACINA-T-1109), on the
    suite's own cluster: it restarts the server and writes the window to the
    disturbance record, as in the chapter failure."""
    runner = world.scratch.get("runner")
    if runner is None:
        report_dir = Path(world.root).joinpath(*DISTURBANCES).parent
        hostdir = Path(world.kubeconfig).parent
        runner = disturb.Runner(
            # Refuses any kubeconfig but the suite's own.
            disturb.Kube(str(world.kubeconfig)),
            disturb.Api(world.base_url),
            dict(world.keys),
            disturb.Record(str(Path(world.root).joinpath(*DISTURBANCES)),
                           str(report_dir / "disturbance-log.jsonl")),
            read_keys=dict(world.checker_keys) or None,
            stop_path=str(hostdir / "disturb.stop"),
            lock_path=str(hostdir / "disturb.lock"),
            out=world.note,
        )
        world.scratch["runner"] = runner
    return runner


def _forward(world):
    """A port forward of this chapter's own to the server, opened again after
    a restart. The forward of the driver points at a pod that is gone."""
    old = getattr(world, "schedules_forward", None)
    if old is not None and old.poll() is None:
        old.terminate()
    proc = subprocess.Popen(["kubectl", "port-forward", f"svc/{world.release}-cloacina-server",
                             f"{FORWARD_PORT}:8080", "-n", world.ns], env=world._kube_env(),
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    world.schedules_forward = proc
    atexit.register(lambda: proc.poll() is None and proc.terminate())
    base = f"http://127.0.0.1:{FORWARD_PORT}"

    def ready():
        try:
            with urllib.request.urlopen(f"{base}/ready", timeout=5) as answer:
                return answer.status == 200
        except Exception:
            return False

    assert _wait(120, ready, every=3), "the server is not ready through the new port forward"
    world.base_url = base


def _restart_servers(world):
    """Every server pod at once, through the disturbance runner: one
    `restart_server` window on record (Dylan, 2026-11-28, on this task)."""
    entry = _runner(world).run_now("restart_server", mode="force", together=True)
    assert entry is not None, ("the server was not restarted (another disturbance holds the lock, "
                               "or no server pod runs); see bdd-standing-report/disturbance-log.jsonl")
    world.scratch["restart_started"] = parse_time(entry.get("start"))
    world.scratch["restart"] = entry
    world.note(f"restarted every server pod: {entry.get('id')}, {entry.get('start')} to "
               f"{entry.get('end')}, {entry.get('outcome')}")


def _ready_since(world):
    """pod -> when it became ready, for each ready server pod."""
    data = world.kubectl_json("get", "pods", "-n", world.ns, "-l", workload.SERVER_SELECTOR)
    found = {}
    for pod in (data or {}).get("items", []):
        if pod.get("metadata", {}).get("deletionTimestamp"):
            continue
        for condition in pod.get("status", {}).get("conditions", []):
            if condition.get("type") == "Ready" and condition.get("status") == "True":
                found[pod["metadata"]["name"]] = parse_time(condition.get("lastTransitionTime"))
    return found


# --- Background ---------------------------------------------------------------

@steps.given("a shared orchestrator runs in Kubernetes with the standing workload of each team")
def bg_workload(world):
    world.ensure_workload()


# --- Scenario 1: on time while other work is slow ------------------------------

@steps.given("every agent slot of team C is held for six minutes")
def s1_hold(world):
    _hold_all_slots(world, "team_c", 360)


@steps.when("the next fire time of the five-minute jobs has passed")
def s1_wait(world):
    fire_time = _next_fire(after_s=30)
    world.scratch["fire_time"] = fire_time
    world.note(f"waiting for the fire time {format_time(fire_time)}")
    _sleep_past(fire_time, 0)
    if "holds" in world.scratch:
        tenant, executions = world.scratch["holds"]
        world.scratch["holds_at_fire"] = [_status(world, tenant, e) for e in executions]
    _sleep_past(fire_time, LATE_LIMIT_SECONDS + 30)


@steps.then("each five-minute schedule of team C started its run for that fire time within 60 seconds")
def s1_on_time(world):
    fire_time = world.scratch["fire_time"]
    fires = _fires(world, "team_c", fire_time)
    wrong = []
    for key in _five_minute_schedules("team_c"):
        runs = fires.get(key, [])
        if len(runs) != 1:
            wrong.append(f"{key}: {len(runs)} runs for {format_time(fire_time)}")
            continue
        started = parse_time(runs[0].get("started_at"))
        delay = None if started is None else started - fire_time
        world.note(f"{key}: started {delay if delay is None else round(delay, 1)} s after the fire time")
        if delay is None or delay > LATE_LIMIT_SECONDS:
            wrong.append(f"{key}: started {delay} s after the fire time")
    assert not wrong, "; ".join(wrong)


@steps.then("team C's held tasks were still running at that fire time")
def s1_busy(world):
    statuses = world.scratch.get("holds_at_fire") or []
    world.note(f"held runs at the fire time: {statuses}")
    assert statuses and not any(s in TERMINAL for s in statuses), (
        "the held tasks were not all running at the fire time: the fire was not made while "
        "other work was slow")


# --- Scenario 2: a run knows its interval -------------------------------------

@steps.then("each data job of each team completed a run for that fire time")
def s2_completed(world):
    fire_time = world.scratch["fire_time"]

    def waiting():
        missing = []
        for tenant in world.tenants:
            fires = _fires(world, tenant, fire_time)
            for job in DATA_JOBS:
                runs = fires.get((job, None), [])
                if not any(run.get("status") == "Completed" for run in runs):
                    missing.append(f"{tenant}/{job}")
        return missing

    _wait(300, lambda: not waiting(), every=10)
    still = waiting()
    assert not still, f"no completed run for {format_time(fire_time)}: " + ", ".join(still)


@steps.then("each of those runs wrote the interval from the fire time before it to its own")
def s2_interval(world):
    fire_time = world.scratch["fire_time"]
    expected_start = FIVE_MINUTES.previous_before(fire_time)
    wrong = []
    for tenant in world.tenants:
        fires = _fires(world, tenant, fire_time)
        for job in DATA_JOBS:
            for run in fires.get((job, None), []):
                written = (run.get("context") or {}).get(INTERVAL_KEYS[job]) or {}
                start = parse_time(written.get("interval_start"))
                end = parse_time(written.get("interval_end"))
                world.note(f"{tenant}/{job}: {written.get('interval_start')} to {written.get('interval_end')}")
                if start != expected_start or end != fire_time:
                    wrong.append(f"{tenant}/{job}: wrote {format_time(start)} to {format_time(end)}, "
                                 f"not {format_time(expected_start)} to {format_time(fire_time)}")
    assert not wrong, "; ".join(wrong)


# --- Scenario 3: a backlog ----------------------------------------------------

@steps.given("team B has an every-minute schedule of the extract job that runs every missed fire "
             "time, one run at a time")
def s3_schedule(world):
    _make_instance(world, "team_b", BACKLOG_INSTANCE,
                   {"cron": "* * * * *", "catchup_policy": "run_all", "max_active_runs": 1})


@steps.when("every agent slot of team B is held for four minutes")
def s3_hold(world):
    _hold_all_slots(world, "team_b", 240)


@steps.when("the held tasks of team B have ended")
def s3_holds_end(world):
    tenant, executions = world.scratch["holds"]
    _wait(900, lambda: all(_status(world, tenant, e) in TERMINAL for e in executions), every=15)
    world.scratch["holds_ended"] = time.time()


@steps.when("the schedule has run every fire time that is due")
def s3_drained(world):
    def drained():
        fires = _instance_fires(world)
        due = int(time.time() - 90) // 60 * 60
        newest = max((parse_time(f.get("scheduled_time")) or 0 for f in fires), default=0)
        active = [f for f in fires if f.get("status") not in TERMINAL]
        return newest >= due and not active

    assert _wait(600, drained, every=10), "the backlog did not drain in ten minutes"
    world.scratch["backlog"] = _instance_fires(world)
    world.note(f"{len(world.scratch['backlog'])} runs of the schedule")


def _backlog_runs(world):
    runs = []
    for fire in world.scratch["backlog"]:
        fire_time = parse_time(fire.get("scheduled_time"))
        started = parse_time(fire.get("started_at"))
        if fire_time is not None and started is not None:
            runs.append((fire_time, started, parse_time(fire.get("completed_at")) or time.time()))
    return sorted(runs)


@steps.then("the schedule had a backlog: runs that started more than a minute after their fire time")
def s3_backlog(world):
    late = [(f, s) for f, s, _ in _backlog_runs(world) if s - f > 60]
    world.note(f"{len(late)} runs started late; the latest by "
               f"{int(max((s - f for f, s in late), default=0))} s")
    assert len(late) >= 2, f"{len(late)} late runs: no backlog was made"


@steps.then("its runs started in the order of their fire times")
def s3_order(world):
    runs = _backlog_runs(world)
    wrong = [f"{format_time(b[0])} started before {format_time(a[0])}"
             for a, b in zip(runs, runs[1:]) if b[1] < a[1]]
    assert not wrong, "; ".join(wrong)


@steps.then("no two of its runs were active at once")
def s3_limit(world):
    runs = _backlog_runs(world)
    wrong = []
    for fire_time, started, _ in runs:
        active = sum(1 for _f, begin, end in runs if begin <= started < end)
        if active > 1:
            wrong.append(f"{active} runs active when the run for {format_time(fire_time)} started")
    assert not wrong, "; ".join(wrong)


@steps.then("every fire time since the schedule was made has exactly one run")
def s3_every(world):
    _tenant, _name, _id, made = world.scratch["instance"]
    expected = Cron("* * * * *").between(made + 1, world.scratch["holds_ended"])
    counts = {}
    for fire in world.scratch["backlog"]:
        stamp = parse_time(fire.get("scheduled_time"))
        if stamp is not None:
            counts[int(stamp)] = counts.get(int(stamp), 0) + 1
    wrong = [f"{format_time(f)}: {counts.get(int(f), 0)} runs" for f in expected
             if counts.get(int(f), 0) != 1]
    world.note(f"{len(expected)} fire times from {format_time(made)} to the end of the holds")
    assert not wrong, "; ".join(wrong)


@steps.then("team B can remove that schedule")
def s3_remove(world):
    tenant, name, _id, _made = world.scratch["instance"]
    code, body = world.api("DELETE", f"/v1/tenants/{tenant}/workflows/{EXTRACT}/instances/{name}",
                           key=world.keys[tenant], expect=None)
    assert code in (200, 204), f"removing the schedule answered {code}: {str(body)[:200]}"


# --- Scenario 4: a time zone --------------------------------------------------

@steps.when("team B makes an hourly schedule of the extract job for a minute of the hour in "
            "Asia/Kathmandu, a few minutes from now")
def s4_make(world):
    fire_time = (int(time.time()) // 60 + 4) * 60
    minute = local_minute_for(fire_time, TZ)
    cron = f"{minute} * * * *"
    # The driver's own reading of the expression must give the same minute.
    assert Cron(cron, TZ).next_after(time.time()) == fire_time, "the driver's cron reader disagrees"
    world.scratch["fire_time"] = float(fire_time)
    world.note(f"cron {cron!r} in {TZ}: local minute {minute} is {format_time(fire_time)}; read "
               f"as UTC it would fire at {format_time(Cron(cron).next_after(time.time()))}")
    _make_instance(world, "team_b", TZ_INSTANCE, {"cron": cron, "timezone": TZ})


@steps.then("the server's next fire time of that schedule is that minute")
def s4_next(world):
    tenant, name, _id, _made = world.scratch["instance"]
    _, body = world.api("GET", f"/v1/tenants/{tenant}/workflows/{EXTRACT}/instances/{name}",
                        key=world.keys[tenant], expect=(200,))
    world.note(f"next_run_at {body.get('next_run_at')}, timezone {body.get('timezone')}")
    assert parse_time(body.get("next_run_at")) == world.scratch["fire_time"], (
        f"the server's next fire time is {body.get('next_run_at')}, "
        f"not {format_time(world.scratch['fire_time'])}")


@steps.then("the schedule fires one run for that minute")
def s4_fires(world):
    fire_time = world.scratch["fire_time"]
    _sleep_past(fire_time, LATE_LIMIT_SECONDS + 60)
    _wait(120, lambda: any(f.get("status") == "Completed" for f in _instance_fires(world)), every=10)
    fires = _instance_fires(world)
    world.note("fires: " + ", ".join(f"{f.get('scheduled_time')} {f.get('status')}" for f in fires))
    times = [parse_time(f.get("scheduled_time")) for f in fires]
    assert times == [fire_time], f"fires at {[format_time(t) for t in times]}, not only at {format_time(fire_time)}"


@steps.then("that run covers the hour before that minute")
def s4_interval(world):
    fire_time = world.scratch["fire_time"]
    fires = _instance_fires(world)
    loaded = (fires[0].get("loaded") if fires else None) or {}
    start = parse_time(loaded.get("interval_start"))
    end = parse_time(loaded.get("interval_end"))
    assert (start, end) == (fire_time - 3600, fire_time), (
        f"the run covers {format_time(start)} to {format_time(end)}")


# --- Scenario 5: a fire due across a restart ----------------------------------

@steps.given("the next fire time of the five-minute jobs is a minute away")
def s5_near(world):
    fire_time = _next_fire(after_s=90)
    world.scratch["fire_time"] = fire_time
    _sleep_past(fire_time, -50)
    world.note(f"the fire time {format_time(fire_time)} is 50 s away")


@steps.when("both server replicas are restarted together just before that fire time")
def s5_restart(world):
    _restart_servers(world)


@steps.when("both server replicas are restarted together")
def s6_restart(world):
    _restart_servers(world)


@steps.when("the server replicas are serving again")
def s5_serving(world):
    start = world.scratch["restart_started"]

    def ready():
        pods = _ready_since(world)
        fresh = {pod: at for pod, at in pods.items() if at is not None and at > start}
        return fresh if len(fresh) >= min(2, workload_replicas(world)) else None

    fresh = _wait(300, ready, every=5)
    assert fresh, "the server replicas did not come back in five minutes"
    world.scratch["ready_at"] = fresh
    world.note("ready again: " + ", ".join(f"{p} at {format_time(a)}" for p, a in sorted(fresh.items())))
    _forward(world)


def workload_replicas(world):
    """The replicas the deployment asks for."""
    data = world.kubectl_json("get", f"deploy/{world.release}-cloacina-server", "-n", world.ns)
    return int(((data or {}).get("spec") or {}).get("replicas") or 1)


@steps.then("no server replica was serving at that fire time")
def s5_down(world):
    fire_time = world.scratch["fire_time"]
    first = min(world.scratch["ready_at"].values())
    assert first > fire_time, (
        f"a replica was ready at {format_time(first)}, before the fire time {format_time(fire_time)}: "
        "the scenario did not make a fire that is due while no scheduler runs")


@steps.then("each five-minute schedule of team A has one run for that fire time")
def s5_fired(world):
    fire_time = world.scratch["fire_time"]

    def wrong():
        fires = _fires(world, "team_a", fire_time)
        return [f"{key}: {len(fires.get(key, []))} runs" for key in _five_minute_schedules("team_a")
                if len(fires.get(key, [])) != 1]

    _wait(300, lambda: not wrong(), every=10)
    still = wrong()
    assert not still, f"for {format_time(fire_time)}: " + "; ".join(still)


# --- Scenario 6: two replicas start together ----------------------------------

@steps.given("the server runs two ready replicas")
def s6_two(world):
    pods = _ready_since(world)
    world.note(f"ready server replicas: {', '.join(sorted(pods)) or 'none'}")
    assert len(pods) >= 2, f"{len(pods)} ready server replica(s); the scenario needs two"


@steps.then("each schedule of the workload is registered once in each team")
def s6_once(world):
    wrong = []
    for tenant in world.tenants:
        rows = _rows(world, (
            "SELECT workflow_name, instance_name, count(*) AS n "
            f"FROM {tenant}.schedules WHERE schedule_type = 'cron' AND backfill IS NULL "
            "AND enabled GROUP BY 1, 2"))
        counts = {(r.get("workflow_name"), r.get("instance_name")): int(r.get("n") or 0) for r in rows}
        for schedule in workload.SCHEDULES:
            key = (schedule.workflow, schedule.instance if schedule.kind == "instance" else None)
            if counts.get(key, 0) != 1:
                wrong.append(f"{tenant}/{key}: {counts.get(key, 0)} rows")
    assert not wrong, "; ".join(wrong)


@steps.then("at the next fire time each five-minute schedule of each team has one run")
def s6_next(world):
    fire_time = _next_fire(after_s=30)
    _sleep_past(fire_time, LATE_LIMIT_SECONDS + 60)

    def wrong():
        found = []
        for tenant in world.tenants:
            fires = _fires(world, tenant, fire_time)
            found.extend(f"{tenant}/{key}: {len(fires.get(key, []))} runs"
                         for key in _five_minute_schedules(tenant) if len(fires.get(key, [])) != 1)
        return found

    _wait(240, lambda: not wrong(), every=10)
    still = wrong()
    assert not still, f"for {format_time(fire_time)}: " + "; ".join(still)
