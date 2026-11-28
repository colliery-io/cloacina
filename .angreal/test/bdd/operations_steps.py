"""Step definitions of the "Operations" chapter (CLOACINA-T-1115).

What the people who run the orchestrator can see: an alert for a run that
failed for good, who did what in the audit record, the backlog and the lag as
gauges, a history that retention keeps short, and a server log with no task
output and no secret in it. The checker's rules (`suite/rules/operations.py`)
judge the same promises on every pass.

Channels that are not the public API, and are the harness's own:

- the alert receiver (`suite/alerts.py`) and the server's metrics listener,
  read through `kubectl port-forward`;
- the server's log, read with `kubectl logs`;
- the retention facts and the tenant's age, read from postgres;
- the server's retention setting, read from its Deployment.

A chapter's steps file names its chapter and its feature files; see
`chapters.py`.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import time
import urllib.request

from .gherkin import StepRegistry
from .standing_world import TERMINAL
from .suite import alerts, workload
from .suite.engine import parse_metrics, parse_time
from .suite.rules import operations as rules

steps = StepRegistry()

CHAPTER = "operations"
FEATURES = ("features/operations/operations.feature",)

TEAM = workload.TENANTS[0]
OTHER_TEAM = workload.TENANTS[1]
SLOTS = workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS

FAIL_PACKAGE = "operations-fail-py"
FAIL_WORKFLOW = "operations_fail"
KEEPER_PACKAGE = "operations-keeper-py"
KEEPER_WORKFLOW = "operations_keeper"
OUTPUT_PREFIX = "OPERATIONS_OUTPUT"
# Each task of the standing workload prints a line that starts with this.
WORKLOAD_OUTPUT_PREFIX = "STANDING_MARK"
LAG_INSTANCE = "operations_lag"
METRICS_PORT = 9090
ALERT_WAIT_SECONDS = 120


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


def _free_port():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def _forward_get(world, service, remote_port, path):
    """GET `path` of a Service of the deployment's namespace, through a port-forward."""
    port = _free_port()
    forward = subprocess.Popen(["kubectl", "port-forward", f"svc/{service}", f"{port}:{remote_port}",
                                "-n", world.ns], env=world._kube_env(),
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        deadline = time.time() + 20
        while True:
            try:
                with urllib.request.urlopen(f"http://127.0.0.1:{port}{path}", timeout=5) as answer:
                    return answer.read().decode("utf-8", "replace")
            except OSError:
                if time.time() >= deadline:
                    raise AssertionError(f"no answer from svc/{service}:{remote_port}{path} "
                                         "through a port-forward")
                time.sleep(1)
    finally:
        forward.terminate()


def _alerts(world, since):
    return json.loads(_forward_get(world, alerts.NAME, alerts.PORT,
                                   f"{alerts.PATH}?since={int(since) - 5}"))


def _alerts_for(world, execution, since):
    return [record for record in _alerts(world, since).get("records") or []
            if (record.get("body") or {}).get("execution_id") == execution]


def _metrics(world):
    return parse_metrics(_forward_get(world, f"{world.release}-cloacina-server-metrics",
                                      METRICS_PORT, "/metrics"))


def _gauge(series, name, **labels):
    """The sum of the series of `name` whose labels include `labels`, or None."""
    values = [value for metric, found, value in series if metric == name
              and all(found.get(key) == str(wanted) for key, wanted in labels.items())]
    return sum(values) if values else None


def _execute(world, workflow, context):
    _, answer = world.api("POST", f"/v1/tenants/{TEAM}/workflows/{workflow}/execute",
                          key=world.keys[TEAM], body={"context": context}, expect=(200, 201, 202))
    assert isinstance(answer, dict) and answer.get("execution_id"), f"no execution id: {answer}"
    return answer["execution_id"]


def _execution(world, execution):
    _, body = world.api("GET", f"/v1/tenants/{TEAM}/executions/{execution}", key=world.keys[TEAM],
                        expect=(200,))
    return body or {}


def _tasks(world, execution):
    _, body = world.api("GET", f"/v1/tenants/{TEAM}/executions/{execution}/tasks",
                        key=world.keys[TEAM], expect=(200,))
    return (body or {}).get("tasks") or []


def _wait_end(world, execution, timeout_s=600):
    status = _wait(timeout_s, lambda: _execution(world, execution).get("status") in TERMINAL
                   and _execution(world, execution).get("status"), every=5)
    assert status, f"the run {execution} had not ended after {timeout_s} s"
    return status


def _start_fail_run(world, mode):
    marker = f"ops-{os.urandom(6).hex()}"
    execution = _execute(world, FAIL_WORKFLOW, {"mode": mode, "marker": marker})
    world.scratch.update({"run": execution, "marker": marker, "started": time.time()})
    world.scratch.setdefault("markers", []).append(marker)
    return execution


def _audit(world, key):
    since = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(time.time() - 3 * 3600))
    code, body = world.api("GET", f"/v1/tenants/{TEAM}/audit?since={since}&limit=500", key=key,
                           expect=None)
    return code, body


def _server_log(world, since):
    """The server pods' log since `since` (the compiler pods are not the server)."""
    seconds = max(60, int(time.time() - since) + 30)
    proc = world.kubectl("logs", "-n", world.ns, "-l",
                         "app.kubernetes.io/name=cloacina-server,app.kubernetes.io/component!=compiler",
                         f"--since={seconds}s", "--tail=-1", "--all-containers=true",
                         "--max-log-requests=10", check=False)
    assert proc.returncode == 0, f"kubectl logs failed: {proc.stderr}"
    return proc.stdout or ""


# --- Background ---------------------------------------------------------------

@steps.given("the standing workload is in place for each team")
def bg_workload(world):
    world.ensure_workload()


# --- Alerts -------------------------------------------------------------------

@steps.given("the alert receiver of the deployment is up")
def g_receiver(world):
    answer = json.loads(_forward_get(world, alerts.NAME, alerts.PORT, "/health"))
    assert answer.get("ok"), f"the alert receiver answered {answer}"
    world.scratch["receiver_since"] = _alerts(world, time.time()).get("started_at")


@steps.given("team A has published the operations packages")
def g_packages(world):
    for package in (FAIL_PACKAGE, KEEPER_PACKAGE):
        world.ensure_version(TEAM, package, package, "0.1.0", key=world.keys[TEAM])


@steps.when("team A starts a run of the failing job that fails for good")
def w_fail(world):
    _start_fail_run(world, "fail")


@steps.when("team A starts a run of the failing job that fails once and then passes")
def w_fail_once(world):
    _start_fail_run(world, "fail_once")


@steps.then("the run ends Failed")
def t_failed(world):
    status = _wait_end(world, world.scratch["run"])
    world.note(f"the run {world.scratch['run']} ended {status}")
    assert status == "Failed", f"the run ended {status}"


@steps.then("one alert for that run arrives within two minutes")
def t_one_alert(world):
    run = world.scratch["run"]
    got = _wait(ALERT_WAIT_SECONDS, lambda: _alerts_for(world, run, world.scratch["started"]), every=10)
    world.note(f"alerts for the run: {len(got)}")
    assert got, f"no alert for the run {run} within {ALERT_WAIT_SECONDS} s"
    # A repeat can come: delivery is at least once. Wait a little, then count.
    time.sleep(30)
    got = _alerts_for(world, run, world.scratch["started"])
    world.scratch["alert"] = got[0].get("body") or {}
    assert len(got) == 1, f"{len(got)} alerts for one failed run"


@steps.then("the alert names team A, the failing job and the run")
def t_alert_fields(world):
    alert = world.scratch["alert"]
    world.note(f"the alert: event {alert.get('event')}, tenant {alert.get('tenant')}, workflow "
               f"{alert.get('workflow')}, run {alert.get('execution_id')}, "
               f"failed tasks {alert.get('failed_task_count')}")
    wrong = [f"{field} is {alert.get(field)!r}, not {wanted!r}" for field, wanted in (
        ("event", rules.FAILED_EVENT), ("tenant", TEAM), ("workflow", FAIL_WORKFLOW),
        ("execution_id", world.scratch["run"])) if alert.get(field) != wanted]
    assert not wrong, "; ".join(wrong)


@steps.then("the run ends Completed after a failed attempt")
def t_completed_after_retry(world):
    status = _wait_end(world, world.scratch["run"])
    tasks = _tasks(world, world.scratch["run"])
    attempts = [task.get("attempt") for task in tasks]
    world.note(f"the run ended {status}; attempts {attempts}")
    assert status == "Completed", f"the run ended {status}"
    assert any(int(task.get("attempt") or 0) > 1 for task in tasks), "no task took a second attempt"


@steps.then("the failed attempt's error is on record with the run")
def t_attempt_error(world):
    errors = [error for task in _tasks(world, world.scratch["run"])
              for error in task.get("attempt_errors") or []]
    world.note(f"attempt errors: {[(e.get('attempt'), e.get('message')) for e in errors]}")
    assert any("operations_fail: fails once" in str(error.get("message")) for error in errors), (
        "the failed attempt's error is not on record")


@steps.then("no alert for that run arrives within two minutes")
def t_no_alert(world):
    run = world.scratch["run"]
    time.sleep(ALERT_WAIT_SECONDS)
    got = _alerts_for(world, run, world.scratch["started"])
    assert not got, f"{len(got)} alerts for a run that completed"


# --- The audit record -----------------------------------------------------------

@steps.when("team A's admin reads team A's audit record")
def w_read_audit(world):
    code, body = _audit(world, world.admin_keys[TEAM])
    assert code == 200, f"team A's audit record answered {code}: {str(body)[:300]}"
    world.scratch["audit"] = (body or {}).get("items") or []
    world.note(f"team A's audit record: {len(world.scratch['audit'])} rows in three hours")


@steps.then("each publish, version change and run by hand in the last five runs of team A's "
            "self-driving workflow has a row")
def t_driver_rows(world):
    records = [r for r in world.driver_actions(TEAM, runs=5)
               if r.get("action") in rules.AUDIT_ACTION and r.get("action") != "start_holds"
               and isinstance(r.get("status"), int) and 200 <= r["status"] < 300]
    assert records, "the last five runs of the self-driving workflow did nothing to judge"
    rows = world.scratch["audit"]
    for row in rows:
        row.setdefault("tenant_id", TEAM)
    missing = []
    found = []
    for record in records:
        stamp = parse_time(record.get("at"))
        if stamp is None:
            continue
        record = dict(record, at_epoch=stamp)
        match = rules.audit_row_for(rows, TEAM, record)
        (found if match else missing).append((record, match))
    world.note(f"{len(found)} of {len(records)} actions have their row")
    world.scratch["driver_rows"] = [row for _record, match in found for row in match]
    assert not missing, "no row for: " + "; ".join(
        f"{r.get('action')} {r.get('package') or r.get('execution_id')} at {r.get('at')}"
        for r, _ in missing)


@steps.then("each of those rows names the key of the self-driving workflow and team A")
def t_driver_rows_named(world):
    key_name = rules.DRIVER_KEY_NAME.format(tenant=TEAM)
    wrong = [f"row {row.get('id')}: key {row.get('actor_key_name')!r}, tenant {row.get('tenant_id')!r}"
             for row in world.scratch["driver_rows"]
             if row.get("actor_key_name") != key_name or not row.get("actor_key_id")
             or row.get("tenant_id") != TEAM]
    assert not wrong, "; ".join(wrong)


@steps.then("every row of the answer is a row of team A")
def t_own_rows(world):
    other = [row for row in world.scratch["audit"] if row.get("tenant_id") != TEAM]
    assert not other, f"{len(other)} rows of other tenants: " + ", ".join(
        f"{row.get('id')} ({row.get('tenant_id')})" for row in other[:10])


@steps.then("team A's admin key cannot read team B's audit record")
def t_other_record(world):
    code, body = world.api("GET", f"/v1/tenants/{OTHER_TEAM}/audit", key=world.admin_keys[TEAM],
                           expect=None)
    world.note(f"team A's admin key on team B's audit record: {code}")
    assert code in (401, 403, 404), f"answered {code}: {str(body)[:200]}"


# --- Backlog and lag ------------------------------------------------------------

@steps.when("team A starts more holds at once than its agents can take")
def w_burst(world):
    world.scratch["holds"] = [
        _execute(world, workload.HOLD_WORKFLOW, {"hold_seconds": workload.HOLD_SECONDS})
        for _ in range(workload.HOLD_BURST)]


@steps.when("team A makes a schedule of the hold job each minute, with one active run at most")
def w_lag_schedule(world):
    world.api("POST", f"/v1/tenants/{TEAM}/workflows/{workload.HOLD_WORKFLOW}/instances",
              key=world.keys[TEAM], expect=(200, 201, 409),
              body={"instance_name": LAG_INSTANCE, "cron": "* * * * *", "max_active_runs": 1})


@steps.then("team A's waiting gauge rises above 0")
def t_waiting_up(world):
    def seen():
        value = _gauge(_metrics(world), rules.WAITING_GAUGE, tenant=TEAM, pool="default")
        return value if value else None

    value = _wait(60, seen, every=5)
    world.note(f"team A's waiting gauge (pool default): {value}")
    assert value, "team A's waiting gauge stayed at 0 while its holds waited"


@steps.then("team A's schedule lag gauge rises above 30 seconds")
def t_lag_up(world):
    def seen():
        value = _gauge(_metrics(world), rules.LAG_GAUGE, tenant=TEAM)
        return value if value and value > 30 else None

    value = _wait(300, seen, every=10)
    lag = _rows(world, rules.LAG_SQL.replace("{schema}", TEAM))
    world.note(f"team A's lag gauge: {value}; in the database: {lag}")
    assert value, "team A's schedule lag gauge did not rise above 30 s"


@steps.then("team A removes the schedule")
def t_remove_schedule(world):
    code, _ = world.api("DELETE", f"/v1/tenants/{TEAM}/workflows/{workload.HOLD_WORKFLOW}/instances/"
                        f"{LAG_INSTANCE}", key=world.keys[TEAM], expect=None)
    assert code in (200, 204, 404), f"removing the schedule answered {code}"


@steps.then("team A's waiting gauge and schedule lag gauge go back to 0 once the holds end")
def t_back_to_zero(world):
    for execution in world.scratch["holds"]:
        _wait_end(world, execution, timeout_s=900)

    def back():
        series = _metrics(world)
        waiting = _gauge(series, rules.WAITING_GAUGE, tenant=TEAM, pool="default")
        lag = _gauge(series, rules.LAG_GAUGE, tenant=TEAM)
        world.scratch["last_gauges"] = (waiting, lag)
        return not waiting and not lag

    ok = _wait(300, back, every=10)
    world.note(f"team A's waiting and lag gauges after: {world.scratch.get('last_gauges')}")
    assert ok, f"the gauges did not go back to 0: {world.scratch.get('last_gauges')}"


# --- Retention --------------------------------------------------------------------

def _keeper_runs(world):
    return [run for run in world.runs(TEAM, KEEPER_WORKFLOW) if run.get("status") not in TERMINAL]


@steps.when("team A has a run of the job of the pool no agent serves")
def w_keeper(world):
    if not _keeper_runs(world):
        _execute(world, KEEPER_WORKFLOW, {})


@steps.then("that run waits, and is active")
def t_keeper_waits(world):
    runs = _wait(60, lambda: _keeper_runs(world), every=5)
    assert runs, "team A has no active run of the keeper job"
    world.note(f"the keeper run {runs[-1].get('id')} started {runs[-1].get('started_at')}, "
               f"is {runs[-1].get('status')}")


@steps.given(r"the server runs with a retention of (\d+) days")
def g_retention_setting(world, days):
    deploy = world.kubectl_json("get", "deploy", f"{world.release}-cloacina-server", "-n", world.ns)
    assert deploy, "no server Deployment"
    env = {item.get("name"): item.get("value")
           for container in deploy["spec"]["template"]["spec"]["containers"]
           for item in container.get("env") or []}
    value = env.get("CLOACINA_RUN_HISTORY_RETENTION_DAYS")
    world.note(f"CLOACINA_RUN_HISTORY_RETENTION_DAYS={value}")
    assert value == days and int(days) == workload.RUN_HISTORY_DAYS, (
        f"the server keeps runs for {value} days; the suite expects {workload.RUN_HISTORY_DAYS}")


@steps.given("team A has run for longer than the retention period")
def g_old_enough(world):
    row = _rows(world, rules.RETENTION_SQL.replace("{schema}", TEAM))[0]
    world.scratch["retention"] = row
    since = parse_time(row.get("tenant_since"))
    age = None if since is None else time.time() - since
    world.note(f"team A's oldest schedule is {int(age or 0) // 3600} h old")
    assert age is not None and age > rules.RETENTION_SECONDS + rules.RETENTION_SLACK_SECONDS, (
        f"team A is {int((age or 0) // 3600)} h old: the retention of "
        f"{workload.RUN_HISTORY_DAYS} days cannot be judged before the third day")


@steps.then("no finished run of any team is older than the retention period")
def t_nothing_old(world):
    wrong = []
    for tenant in workload.TENANTS:
        row = _rows(world, rules.RETENTION_SQL.replace("{schema}", tenant))[0]
        world.note(f"{tenant}: {row.get('old_finished')} old finished runs, "
                   f"{row.get('orphan_tasks')} task rows with no run")
        if int(row.get("old_finished") or 0) or int(row.get("orphan_tasks") or 0):
            wrong.append(f"{tenant}: {row}")
    assert not wrong, "; ".join(wrong)


@steps.then("team A's run that waits is still there, older than the retention period")
def t_keeper_kept(world):
    runs = _keeper_runs(world)
    assert runs, "team A's keeper run is gone"
    oldest = min(parse_time(run.get("started_at")) or time.time() for run in runs)
    world.note(f"the keeper run is {int(time.time() - oldest) // 3600} h old")
    assert time.time() - oldest > rules.RETENTION_SECONDS, (
        "the keeper run is younger than the retention period: start it, and judge again later")


# --- The server log -----------------------------------------------------------------

@steps.when("team A's self-driving workflow gets a new key in its secret")
def w_new_secret(world):
    world.scratch["log_since"] = time.time()
    _, created = world.api("POST", f"/v1/tenants/{TEAM}/keys", key=world.bootstrap_key,
                           body={"name": rules.DRIVER_KEY_NAME.format(tenant=TEAM), "role": "write"},
                           expect=(200, 201))
    value = created["key"]
    world.api("PUT", f"/v1/tenants/{TEAM}/secrets/{workload.DRIVER_SECRET}", key=world.bootstrap_key,
              body={"fields": {"key": value}}, expect=(200, 201, 204))
    # Kept in memory for the next step only; never written or noted.
    world.scratch["secret_values"] = [value]


@steps.when("the self-driving workflow of team A has run with it")
def w_driver_ran(world):
    since = world.scratch["log_since"]

    def done():
        return [run for run in world.runs(TEAM, workload.DRIVER_WORKFLOW)
                if run.get("status") == "Completed" and (parse_time(run.get("started_at")) or 0) > since]

    runs = _wait(2 * workload.SLOT_SECONDS + 120, done, every=10)
    assert runs, "the self-driving workflow of team A finished no run after the change"
    actions = world.driver_actions(TEAM, runs=1)
    errors = [a for a in actions if a.get("action") == "read_key" and a.get("error")]
    assert not errors, f"the self-driving workflow could not read its key: {errors}"


@steps.then("the server log since the scenario began holds no line a task printed")
def t_no_output_in_log(world):
    text = _server_log(world, world.scratch.get("log_since") or world.scratch.get("started"))
    lines = text.splitlines()
    found = [line for line in lines if WORKLOAD_OUTPUT_PREFIX in line or OUTPUT_PREFIX in line
             or any(marker in line for marker in world.scratch.get("markers", []))]
    world.note(f"{len(lines)} lines of the server log read; {len(found)} hold task output")
    for line in found[:5]:
        world.note(f"task output in the server log: {line[:200]}")
    assert not found, f"{len(found)} lines of the server log hold task output"


@steps.then("the server log holds no value of team A's secret")
def t_no_secret_in_log(world):
    text = _server_log(world, world.scratch["log_since"])
    hits = sum(text.count(value) for value in world.scratch.get("secret_values", []))
    world.scratch.pop("secret_values", None)
    assert not hits, f"the server log holds the value of team A's secret {hits} times"


# --- Attempt output (CLOACINA-T-1091) -------------------------------------------------

@steps.then("the output of its failed attempt holds the line the task printed")
def t_attempt_output(world):
    execution = world.scratch["run"]
    tasks = _tasks(world, execution)
    assert tasks, "the run has no task"
    code, body = world.api("GET", f"/v1/tenants/{TEAM}/executions/{execution}/tasks/"
                           f"{tasks[0].get('id')}/output", key=world.keys[TEAM], expect=None)
    text = json.dumps(body) if not isinstance(body, str) else body
    world.note(f"the output route answered {code}")
    assert code == 200, f"the output route answered {code}"
    assert f"{OUTPUT_PREFIX} {world.scratch['marker']}" in text, "the output does not hold the line"
