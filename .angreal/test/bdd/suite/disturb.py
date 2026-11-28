"""The disturbance schedule of the behaviour suite (CLOACINA-T-1109).

On a schedule, something in the standing deployment is broken on purpose, so
the Failure and Capacity promises are judged against real trouble. Each
disturbance is on record, in the form the checker reads.

    angreal test bdd standing --disturb             # run the schedule until stopped
    angreal test bdd standing --disturb --dry-run   # print the schedule and the commands
    angreal test bdd standing --disturb-stop        # stop it; nothing is left half done

What each kind does:

- `kill_agent`: find an agent of one tenant with work in flight, find its pod
  by the agent id in the pod's log, and delete that pod. Even cycles delete it
  the plain way (SIGTERM: the agent drains, CLOACINA-T-1029); odd cycles with
  `--grace-period=0 --force` (the agent dies with its work).
- `restart_server`: delete one pod of the server Deployment (the same two
  modes), and wait for the Deployment to be available again.
- `postgres_away`: scale the Postgres Deployment to zero, wait some minutes,
  scale it back to one, and wait for the server to be ready.
- `fill_capacity`: start more held runs of `standing_hold` than the tenant's
  agents can take, through the API with that tenant's write key.

Containment. The runner acts on the suite's own k3s cluster only:

- the only program it runs is `kubectl`, never `docker`;
- every `kubectl` call names the suite's own kubeconfig file and its one
  context, with no other kubeconfig in the environment; another path is
  refused;
- before each disturbance it reads that kubeconfig and the cluster and
  refuses to act unless the kubeconfig has exactly one context and cluster,
  the cluster is the k3s of the suite (`https://localhost:6443`), and the
  cluster holds the suite's namespace and server Deployment;
- each command is checked against a short list: the verbs it may use, the
  namespaces of the suite and of the workload's tenants, and the one
  Deployment it may scale;
- the API it calls is on a loopback address.

The record. A line is written to the disturbance record before each action
(an open window) and a line with the same id after it (its end and the time
by which the system must be well again). The checker takes the last line of
an id. A second file, the run's log, holds every check, command, refusal and
skip, with its time and target.

Standard library only; no angreal. Nothing here acts until a `Runner` method
is called with a real `Kube`.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request

from . import promises, workload
from .engine import format_time, merge_lines
from .sources import read_disturbances

# --- the suite's own deployment -------------------------------------------
# The same names as `standing.py` (a unit test compares the two copies).

PROJECT = "cloacina-k8s-bdd-standing"
NAMESPACE = "cloacina-bdd-standing"
RELEASE = "bdd-standing"
SERVER_DEPLOYMENT = f"{RELEASE}-cloacina-server"
POSTGRES_DEPLOYMENT = f"{RELEASE}-postgresql"
AGENT_DEPLOYMENT = "cloacina-agent"
SERVER_PODS = f"app.kubernetes.io/name=cloacina-server,app.kubernetes.io/instance={RELEASE}"
AGENT_PODS = "app.kubernetes.io/name=cloacina-agent"
# The k3s of `.angreal/files/docker-compose.k8s.yaml` writes one context named
# `default`; the harness points the host's copy at localhost:6443.
CONTEXT = "default"
CLUSTER_SERVER = "https://localhost:6443"
KUBECONFIG_NAME = "kubeconfig.host.yaml"

KINDS = tuple(promises.DISTURBANCE_TESTS)
MODES = ("sigterm", "force")


def hostdir():
    """The suite's own folder on the host, as `standing.py` makes it."""
    return os.path.join(tempfile.gettempdir(), PROJECT)


def suite_kubeconfig():
    return os.path.join(hostdir(), KUBECONFIG_NAME)


def tenant_namespace(tenant):
    """The namespace of a tenant's agents (as `world.tenant_namespace`)."""
    seg = re.sub(r"[^a-z0-9]+", "-", tenant.lower()).strip("-") or "unnamed"
    return f"cloacina-tenant-{seg}"


# --- the schedule ----------------------------------------------------------
# One place to read and change it. A cycle of four hours; one kind each hour,
# at minute 2, while the hourly six-minute task `standing_archive` of every
# tenant runs on an agent. The tenant turns with each cycle, the mode of a
# kill with every second cycle: in 24 hours each kind runs six times, on each
# tenant twice, and each kill in each mode three times.

CYCLE_SECONDS = 4 * 3600


class Slot:
    def __init__(self, offset_s, kind):
        self.offset_s = offset_s
        self.kind = kind


SCHEDULE = (
    Slot(0 * 3600 + 120, "kill_agent"),
    Slot(1 * 3600 + 120, "restart_server"),
    Slot(2 * 3600 + 120, "postgres_away"),
    Slot(3 * 3600 + 120, "fill_capacity"),
)

# How long Postgres stays away.
POSTGRES_AWAY_SECONDS = 240
# How long the runner waits for an agent of the tenant to hold a task.
AGENT_WAIT_SECONDS = 180
# Held runs a `fill_capacity` starts: twice the tenant's agent slots.
FILL_RUNS = 2 * workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS
# Timeouts of the waits after an action.
ROLLOUT_TIMEOUT_SECONDS = 300
READY_TIMEOUT_SECONDS = 300

# After the action ends, the time by which the system must be well again
# (the window's `settle_until`).
SETTLE_SECONDS = {
    "kill_agent": 900,
    "restart_server": 900,
    "postgres_away": 1200,
    # The held runs drain in two waves of two minutes; the rest is slack.
    "fill_capacity": 900,
}

# The longest an action itself can take, for the check that windows do not
# overlap.
MAX_ACTION_SECONDS = {
    "kill_agent": AGENT_WAIT_SECONDS + ROLLOUT_TIMEOUT_SECONDS + 60,
    "restart_server": ROLLOUT_TIMEOUT_SECONDS + 60,
    "postgres_away": POSTGRES_AWAY_SECONDS + ROLLOUT_TIMEOUT_SECONDS + READY_TIMEOUT_SECONDS + 60,
    "fill_capacity": 300,
}


class Planned:
    """One disturbance of the schedule, or one asked for now."""

    def __init__(self, at, kind, tenant, mode, cycle):
        self.at = at
        self.kind = kind
        self.tenant = tenant
        self.mode = mode
        self.cycle = cycle

    def describe(self):
        mode = f" ({self.mode})" if self.mode else ""
        target = self.tenant if self.kind in ("kill_agent", "fill_capacity") else (
            "one server replica" if self.kind == "restart_server" else "Postgres")
        return f"{format_time(self.at)}  {self.kind}{mode} of {target}"


def planned_at(cycle, slot):
    at = cycle * CYCLE_SECONDS + slot.offset_s
    tenant = workload.TENANTS[cycle % len(workload.TENANTS)]
    mode = MODES[cycle % 2] if slot.kind in ("kill_agent", "restart_server") else None
    return Planned(at, slot.kind, tenant, mode, cycle)


def plan(start, end):
    """Every planned disturbance from `start` to `end` (epoch seconds)."""
    found = []
    cycle = int(start // CYCLE_SECONDS)
    while cycle * CYCLE_SECONDS <= end:
        for slot in SCHEDULE:
            item = planned_at(cycle, slot)
            if start <= item.at <= end:
                found.append(item)
        cycle += 1
    return found


def next_planned(now):
    return plan(now, now + 2 * CYCLE_SECONDS)[0]


def schedule_problems():
    """What is wrong with the schedule. Empty when it is sound."""
    found = []
    offsets = sorted(SCHEDULE, key=lambda slot: slot.offset_s)
    for slot in offsets:
        if slot.kind not in KINDS:
            found.append(f"the schedule names an unknown kind: {slot.kind}")
        if not 0 <= slot.offset_s < CYCLE_SECONDS:
            found.append(f"the slot of {slot.kind} is outside the cycle")
    for position, slot in enumerate(offsets):
        following = offsets[(position + 1) % len(offsets)]
        gap = (following.offset_s - slot.offset_s) % CYCLE_SECONDS or CYCLE_SECONDS
        need = MAX_ACTION_SECONDS.get(slot.kind, 0) + SETTLE_SECONDS.get(slot.kind, 0)
        if need >= gap:
            found.append(f"the window of {slot.kind} ({need} s) can reach the next one ({gap} s later)")
    for kind in KINDS:
        if kind not in SETTLE_SECONDS or kind not in MAX_ACTION_SECONDS:
            found.append(f"{kind} has no settle time or no longest action")
        if not any(slot.kind == kind for slot in SCHEDULE):
            found.append(f"{kind} is not in the schedule")
    per_day = {}
    for item in plan(0, 86400 - 1):
        per_day[item.kind] = per_day.get(item.kind, 0) + 1
    for kind, count in sorted(per_day.items()):
        if count < 2:
            found.append(f"{kind} runs {count} times a day; the task asks for several")
    if FILL_RUNS <= workload.AGENTS_PER_TENANT * workload.AGENT_SLOTS:
        found.append("FILL_RUNS does not exceed the tenant's agent slots")
    return found


# --- containment ----------------------------------------------------------


class Refused(Exception):
    """A command or a disturbance the runner will not do."""


def _namespace_of(args):
    for flag in ("-n", "--namespace"):
        if flag in args:
            position = args.index(flag)
            if position + 1 < len(args):
                return args[position + 1]
    for arg in args:
        if arg.startswith("--namespace="):
            return arg.split("=", 1)[1]
    return None


def allowed_namespaces():
    return {NAMESPACE} | {tenant_namespace(tenant) for tenant in workload.TENANTS}


def check_command(args):
    """Why a kubectl command is refused, or None when it is allowed."""
    args = list(args)
    if not args:
        return "an empty command"
    for arg in args:
        if arg in ("-A", "--all-namespaces", "--all") or arg.startswith("--all-namespaces"):
            return f"{arg} reaches beyond the suite"
        if arg in ("--kubeconfig", "--context", "--cluster", "--server", "-s", "--user", "--token") \
                or arg.startswith(("--kubeconfig=", "--context=", "--cluster=", "--server=",
                                   "--user=", "--token=")):
            return f"{arg} is set by the runner only"
    if args[:2] == ["config", "view"]:
        return None
    namespace = _namespace_of(args)
    if namespace is None:
        return "no namespace: every command names the suite's namespace or a tenant's"
    if namespace not in allowed_namespaces():
        return f"namespace {namespace} is not the suite's"
    verb = args[0]
    if verb in ("get", "logs"):
        return None
    if verb == "delete":
        if len(args) < 3 or args[1] not in ("pod", "pods") or args[2].startswith("-"):
            return "only one pod, by name, may be deleted"
        if any(arg in ("-l", "--selector") or arg.startswith("--selector=") for arg in args):
            return "a delete names its pod; no selector"
        return None
    if verb == "scale":
        if len(args) < 2 or args[1] != f"deployment/{POSTGRES_DEPLOYMENT}" or namespace != NAMESPACE:
            return "only the suite's Postgres Deployment may be scaled"
        if not any(re.fullmatch(r"--replicas=[01]", arg) for arg in args):
            return "Postgres is scaled to 0 or 1 only"
        return None
    if verb == "rollout":
        if len(args) < 3 or args[1] != "status" or not args[2].startswith("deployment/"):
            return "only `rollout status deployment/...` is allowed"
        return None
    return f"the verb {verb} is not allowed"


def _subprocess_run(argv, env, timeout):
    try:
        proc = subprocess.run(argv, env=env, capture_output=True, text=True, timeout=timeout,
                              check=False)
    except subprocess.TimeoutExpired:
        return 124, "", f"timed out after {timeout} s"
    except OSError as exc:
        return 127, "", str(exc)
    return proc.returncode, proc.stdout, proc.stderr


class Kube:
    """`kubectl` on the suite's own kubeconfig, and nothing else."""

    def __init__(self, kubeconfig, expected=None, run=None):
        expected = os.path.realpath(expected or suite_kubeconfig())
        if os.path.realpath(kubeconfig) != expected:
            raise Refused(f"the kubeconfig {kubeconfig} is not the suite's own ({expected})")
        self.kubeconfig = expected
        self._run = run or _subprocess_run
        env = {name: value for name, value in os.environ.items()
               if not name.startswith(("KUBECONFIG", "KUBE_"))}
        env["KUBECONFIG"] = self.kubeconfig
        self.env = env

    def argv(self, args):
        return ["kubectl", "--kubeconfig", self.kubeconfig, "--context", CONTEXT, *args]

    def call(self, *args, timeout=120):
        """Run one allowed command. Returns (code, stdout, stderr)."""
        why = check_command(args)
        if why:
            raise Refused(f"kubectl {' '.join(args)}: {why}")
        argv = self.argv(args)
        if argv[0] != "kubectl":
            raise Refused("the runner runs kubectl only")
        return self._run(argv, self.env, timeout)

    def json(self, *args):
        code, out, err = self.call(*args, "-o", "json")
        if code != 0:
            return None
        try:
            return json.loads(out)
        except ValueError:
            return None


def verify_cluster(kube):
    """Refuse unless the kubeconfig and the cluster are the suite's own.
    Returns what was checked, for the log."""
    code, out, err = kube.call("config", "view", "-o", "json")
    if code != 0:
        raise Refused(f"cannot read the kubeconfig: {err.strip()[:200]}")
    try:
        config = json.loads(out)
    except ValueError:
        raise Refused("the kubeconfig is not readable as JSON")
    contexts = [c.get("name") for c in config.get("contexts") or []]
    clusters = [(c.get("name"), (c.get("cluster") or {}).get("server"))
                for c in config.get("clusters") or []]
    if contexts != [CONTEXT]:
        raise Refused(f"the kubeconfig has the contexts {contexts}, not only {CONTEXT!r}")
    if config.get("current-context") != CONTEXT:
        raise Refused(f"the current context is {config.get('current-context')!r}, not {CONTEXT!r}")
    if len(clusters) != 1 or clusters[0][1] != CLUSTER_SERVER:
        raise Refused(f"the kubeconfig's cluster is {clusters}, not the suite's k3s at {CLUSTER_SERVER}")
    code, out, err = kube.call("get", "deployment", SERVER_DEPLOYMENT, "-n", NAMESPACE, "-o", "name")
    if code != 0 or out.strip() != f"deployment.apps/{SERVER_DEPLOYMENT}":
        raise Refused(f"the cluster has no {NAMESPACE}/{SERVER_DEPLOYMENT}: it is not the suite's "
                      f"standing deployment ({(err or out).strip()[:200]})")
    return f"context {CONTEXT}, cluster {CLUSTER_SERVER}, {NAMESPACE}/{SERVER_DEPLOYMENT} present"


# --- the API --------------------------------------------------------------


class Api:
    """The server's API on a loopback address, with a tenant's key."""

    def __init__(self, base, timeout_s=15, opener=None):
        parsed = urllib.parse.urlparse(base)
        if parsed.scheme not in ("http", "https") or parsed.hostname not in ("127.0.0.1", "localhost", "::1"):
            raise Refused(f"the API {base} is not on a loopback address: the runner talks to the "
                          "suite's forwarded server only")
        self.base = base.rstrip("/")
        self.timeout_s = timeout_s
        self._open = opener or urllib.request.urlopen

    def call(self, method, path, key=None, body=None):
        data = json.dumps(body).encode("utf-8") if body is not None else None
        request = urllib.request.Request(f"{self.base}{path}", data=data, method=method)
        if key:
            request.add_header("Authorization", f"Bearer {key}")
        if data is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with self._open(request, timeout=self.timeout_s) as response:
                text, status = response.read().decode("utf-8", "replace"), response.status
        except urllib.error.HTTPError as exc:
            text, status = exc.read().decode("utf-8", "replace"), exc.code
        except Exception as exc:
            return None, f"{exc.__class__.__name__}: {exc}"
        try:
            return status, (json.loads(text) if text else None)
        except ValueError:
            return status, text[:500]


# --- the record -----------------------------------------------------------


def _append(path, value):
    folder = os.path.dirname(path)
    if folder:
        os.makedirs(folder, exist_ok=True)
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(value, sort_keys=True) + "\n")
        handle.flush()
        os.fsync(handle.fileno())


class Record:
    """The disturbance record the checker reads, and the run's log."""

    def __init__(self, path, log_path, clock=time.time):
        self.path = path
        self.log_path = log_path
        self.clock = clock

    def log(self, event, **fields):
        line = {"at": format_time(self.clock()), "event": event}
        line.update(fields)
        _append(self.log_path, line)
        return line

    def open(self, kind, target, **fields):
        now = self.clock()
        entry = {"id": f"d-{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime(now))}-{kind}",
                 "kind": kind, "target": target, "start": format_time(now)}
        entry.update(fields)
        _append(self.path, entry)
        self.log("start", id=entry["id"], kind=kind, target=target)
        return entry

    def close(self, entry, outcome, settle_s, **fields):
        now = self.clock()
        closed = dict(entry)
        closed.update(fields)
        closed["end"] = format_time(now)
        closed["settle_until"] = format_time(now + settle_s)
        closed["outcome"] = outcome
        _append(self.path, closed)
        self.log("end", id=entry["id"], kind=entry["kind"], target=entry["target"], outcome=outcome)
        return closed

    def entries(self):
        """The record, one entry for each id (the last line of an id)."""
        return merge_lines(read_disturbances(self.path))

    def open_entries(self):
        return [entry for entry in self.entries() if not entry.get("end")]


class Lock:
    """One disturbance at a time, across processes: the schedule's runner and
    a scenario that asks for one now."""

    def __init__(self, path):
        self.path = path

    def take(self, what):
        os.makedirs(os.path.dirname(self.path), exist_ok=True)
        for _ in range(2):
            try:
                handle = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            except FileExistsError:
                if self._stale():
                    os.unlink(self.path)
                    continue
                return False
            with os.fdopen(handle, "w", encoding="utf-8") as out:
                out.write(json.dumps({"pid": os.getpid(), "what": what}))
            return True
        return False

    def _stale(self):
        try:
            with open(self.path, "r", encoding="utf-8") as handle:
                pid = int(json.load(handle).get("pid"))
        except (OSError, ValueError, TypeError, AttributeError):
            return True
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return True
        except PermissionError:
            return False
        return False

    def release(self):
        try:
            os.unlink(self.path)
        except FileNotFoundError:
            pass


# --- the commands ---------------------------------------------------------
# Built in one place, so the dry run prints what a real run does.


def delete_pod_args(pod, namespace, mode):
    args = ["delete", "pod", pod, "-n", namespace, "--wait=false"]
    if mode == "force":
        args += ["--grace-period=0", "--force"]
    return args


def rollout_args(deployment, namespace, timeout_s=ROLLOUT_TIMEOUT_SECONDS):
    return ["rollout", "status", f"deployment/{deployment}", "-n", namespace, f"--timeout={timeout_s}s"]


def postgres_scale_args(replicas):
    return ["scale", f"deployment/{POSTGRES_DEPLOYMENT}", f"--replicas={replicas}", "-n", NAMESPACE]


def hold_path(tenant):
    return f"/v1/tenants/{tenant}/workflows/{workload.HOLD_WORKFLOW}/execute"


AGENT_ID = re.compile(r'agent_id["\']?\s*[=:]\s*"?([A-Za-z0-9][A-Za-z0-9_.:-]*)')


def agent_id_in_log(text):
    """The last agent id an agent's log names (text or JSON lines)."""
    found = AGENT_ID.findall(text or "")
    return found[-1] if found else None


def commands_of(item):
    """The steps of a planned disturbance, as text, for the dry run."""
    guard = ["kubectl config view -o json  (one context `default`, cluster https://localhost:6443)",
             f"kubectl get deployment {SERVER_DEPLOYMENT} -n {NAMESPACE} -o name"]
    if item.kind == "kill_agent":
        ns = tenant_namespace(item.tenant)
        steps = [
            f"GET /v1/agents with the read key of {item.tenant}: an agent with in_flight > 0 "
            f"(wait up to {AGENT_WAIT_SECONDS} s)",
            f"kubectl get pods -n {ns} -l {AGENT_PODS} -o json",
            f"kubectl logs <each pod> -n {ns}: the agent id of each pod",
            "kubectl " + " ".join(delete_pod_args("<the pod of that agent>", ns, item.mode)),
            "kubectl " + " ".join(rollout_args(AGENT_DEPLOYMENT, ns)),
        ]
    elif item.kind == "restart_server":
        steps = [
            f"kubectl get pods -n {NAMESPACE} -l {SERVER_PODS} -o json",
            "kubectl " + " ".join(delete_pod_args("<the next server pod in turn>", NAMESPACE, item.mode)),
            "kubectl " + " ".join(rollout_args(SERVER_DEPLOYMENT, NAMESPACE)),
        ]
    elif item.kind == "postgres_away":
        steps = [
            "kubectl " + " ".join(postgres_scale_args(0)),
            f"wait {POSTGRES_AWAY_SECONDS} s",
            "kubectl " + " ".join(postgres_scale_args(1)),
            "kubectl " + " ".join(rollout_args(POSTGRES_DEPLOYMENT, NAMESPACE)),
            f"GET /ready until it answers 200 (up to {READY_TIMEOUT_SECONDS} s)",
        ]
    else:
        steps = [f"POST {hold_path(item.tenant)} {FILL_RUNS} times with the write key of "
                 f"{item.tenant}, context {{hold_seconds: {workload.HOLD_SECONDS}}}"]
    return guard + steps


def plan_text(start, hours=24):
    """The dry run: the schedule and the commands of the next `hours`."""
    lines = [f"Disturbance schedule: a cycle of {CYCLE_SECONDS // 3600} h, from {format_time(start)}.",
             "Each disturbance is written to the record before it acts and again when it ends.",
             ""]
    for item in plan(start, start + hours * 3600):
        tested = ", ".join(promises.DISTURBANCE_TESTS[item.kind])
        lines.append(item.describe())
        lines.append(f"    tests: {tested}; settles {SETTLE_SECONDS[item.kind]} s after it ends")
        for step in commands_of(item):
            lines.append(f"    {step}")
    problems = schedule_problems()
    lines.append("")
    lines.append("schedule: " + ("sound" if not problems else "; ".join(problems)))
    return "\n".join(lines)


# --- the runner -----------------------------------------------------------


class Runner:
    def __init__(self, kube, api, keys, record, *, read_keys=None, stop_path=None, lock_path=None,
                 clock=time.time, sleep=time.sleep, out=print):
        self.kube = kube
        self.api = api
        # Write keys, tenant to key: `fill_capacity` starts runs with them.
        self.keys = dict(keys)
        # Read keys for the roster; the write keys serve when none is given.
        self.read_keys = dict(read_keys or keys)
        self.record = record
        self.stop_path = stop_path or os.path.join(hostdir(), "disturb.stop")
        self.lock = Lock(lock_path or os.path.join(hostdir(), "disturb.lock"))
        self.clock = clock
        self.sleep = sleep
        self.out = out
        # Set when a disturbance left the deployment unwell: the schedule stops.
        self.halted = None

    # -- stopping ----------------------------------------------------------

    def stop_requested(self):
        return os.path.exists(self.stop_path)

    def _wait(self, seconds, step=15, until=None):
        """Sleep up to `seconds`, a step at a time. Returns early when `until`
        gives something true, or when a stop is asked for."""
        deadline = self.clock() + seconds
        while True:
            if until is not None:
                value = until()
                if value:
                    return value
            if self.stop_requested() or self.clock() >= deadline:
                return None
            self.sleep(min(step, max(0.0, deadline - self.clock())))

    # -- one disturbance ---------------------------------------------------

    def run_now(self, kind, tenant=None, mode=None):
        """One disturbance now, outside the schedule (a scenario asks for it).
        Returns the closed record entry, or None when nothing was done."""
        if kind not in KINDS:
            raise Refused(f"unknown kind of disturbance: {kind}")
        tenant = tenant or workload.TENANTS[0]
        if kind in ("kill_agent", "restart_server"):
            mode = mode or MODES[0]
        return self.disturb(Planned(self.clock(), kind, tenant, mode, cycle=0))

    def disturb(self, item):
        if item.kind not in KINDS:
            raise Refused(f"unknown kind of disturbance: {item.kind}")
        if item.mode is not None and item.mode not in MODES:
            raise Refused(f"unknown mode: {item.mode}")
        if item.tenant not in workload.TENANTS:
            raise Refused(f"{item.tenant} is not a tenant of the workload")
        try:
            checked = verify_cluster(self.kube)
        except Refused as exc:
            self.record.log("refused", kind=item.kind, target=item.tenant, reason=str(exc))
            raise
        self.record.log("checked", kind=item.kind, target=item.tenant, what=checked)
        if not self.lock.take(item.kind):
            self.record.log("skipped", kind=item.kind, target=item.tenant,
                            reason="another disturbance is running")
            return None
        try:
            return getattr(self, f"_{item.kind}")(item)
        finally:
            self.lock.release()

    def _call(self, args, timeout=120):
        code, out, err = self.kube.call(*args, timeout=timeout)
        self.record.log("command", argv=["kubectl", *args], code=code,
                        output=(err or out or "").strip()[-300:])
        return code, out, err

    def _close(self, entry, outcome, **fields):
        return self.record.close(entry, outcome, SETTLE_SECONDS[entry["kind"]], **fields)

    # kill_agent

    def _running_pods(self, namespace, selector):
        data = self.kube.json("get", "pods", "-n", namespace, "-l", selector)
        pods = []
        for pod in (data or {}).get("items", []):
            meta, status = pod.get("metadata") or {}, pod.get("status") or {}
            if status.get("phase") == "Running" and not meta.get("deletionTimestamp"):
                pods.append(meta.get("name"))
        return sorted(name for name in pods if name)

    def agent_pod_with_work(self, tenant):
        """(pod, agent id, in flight, why) of an agent of the tenant that holds
        a task. pod is None when none can be named."""
        status, body = self.api.call("GET", "/v1/agents", key=self.read_keys.get(tenant))
        if status != 200 or not isinstance(body, dict):
            return None, None, 0, f"the roster answered {status}"
        agents = [a for a in body.get("items") or []
                  if a.get("tenant_id") == tenant and int(a.get("seconds_since_heartbeat") or 0) < 60]
        busy = {a.get("agent_id"): int(a.get("in_flight") or 0) for a in agents
                if int(a.get("in_flight") or 0) > 0}
        if not busy:
            return None, None, 0, f"no agent of {tenant} holds a task ({len(agents)} live)"
        namespace = tenant_namespace(tenant)
        pods = self._running_pods(namespace, AGENT_PODS)
        if not pods:
            return None, None, 0, f"no running agent pod in {namespace}"
        named = {}
        for pod in pods:
            code, out, err = self.kube.call("logs", pod, "-n", namespace, timeout=60)
            agent = agent_id_in_log(out) if code == 0 else None
            if agent:
                named[agent] = pod
        for agent, in_flight in sorted(busy.items(), key=lambda pair: -pair[1]):
            if agent in named:
                return named[agent], agent, in_flight, "found by the agent id in the pod's log"
        if len(busy) == len(agents) and len(pods) == len(agents):
            agent, in_flight = sorted(busy.items())[0]
            return pods[0], agent, in_flight, "every agent of the tenant holds a task"
        return None, None, 0, (f"{len(busy)} of {len(agents)} agents hold a task, and no pod's log "
                               "names one of them")

    def _delete_and_wait(self, pod, namespace, mode, deployment):
        """Delete a pod and wait for its Deployment. The outcome in words."""
        code, _out, err = self._call(delete_pod_args(pod, namespace, mode))
        if code != 0:
            return f"the delete failed: {err.strip()[:200]}"
        code, _out, _err = self._call(rollout_args(deployment, namespace),
                                      timeout=ROLLOUT_TIMEOUT_SECONDS + 30)
        if code != 0:
            return f"pod deleted; {deployment} not available after {ROLLOUT_TIMEOUT_SECONDS} s"
        return f"pod deleted; {deployment} available again"

    def _kill_agent(self, item):
        namespace = tenant_namespace(item.tenant)

        def holder():
            found = self.agent_pod_with_work(item.tenant)
            return found if found[0] else None

        found = self._wait(AGENT_WAIT_SECONDS, step=15, until=holder)
        if not found:
            _pod, _agent, _n, why = self.agent_pod_with_work(item.tenant)
            self.record.log("skipped", kind=item.kind, target=item.tenant, reason=why)
            self.out(f"skipped kill_agent of {item.tenant}: {why}")
            return None
        pod, agent, in_flight, how = found
        entry = self.record.open("kill_agent", f"{item.tenant} pod {pod}", tenant=item.tenant,
                                 pod=pod, agent_id=agent, in_flight=in_flight, mode=item.mode,
                                 chosen=how)
        outcome = "interrupted"
        try:
            outcome = self._delete_and_wait(pod, namespace, item.mode, AGENT_DEPLOYMENT)
        finally:
            closed = self._close(entry, outcome)
            self.out(f"kill_agent ({item.mode}) of {item.tenant}, pod {pod}: {outcome}")
        return closed

    # restart_server

    def _restart_server(self, item):
        pods = self._running_pods(NAMESPACE, SERVER_PODS)
        if not pods:
            self.record.log("skipped", kind=item.kind, target="server", reason="no running server pod")
            return None
        pod = pods[item.cycle % len(pods)]
        entry = self.record.open("restart_server", f"server pod {pod}", pod=pod, replicas=len(pods),
                                 mode=item.mode)
        outcome = "interrupted"
        try:
            outcome = self._delete_and_wait(pod, NAMESPACE, item.mode, SERVER_DEPLOYMENT)
        finally:
            closed = self._close(entry, outcome)
            self.out(f"restart_server ({item.mode}), pod {pod} of {len(pods)}: {outcome}")
        return closed

    # postgres_away

    def _ready(self):
        status, _body = self.api.call("GET", "/ready")
        return status == 200

    def restore_postgres(self):
        """Scale Postgres back and wait for it and the server. Returns the
        outcome in words, and whether the deployment is well."""
        code, _out, err = self._call(postgres_scale_args(1))
        if code != 0:
            return f"Postgres could not be scaled back: {err.strip()[:200]}", False
        code, _out, _err = self._call(rollout_args(POSTGRES_DEPLOYMENT, NAMESPACE),
                                      timeout=ROLLOUT_TIMEOUT_SECONDS + 30)
        if code != 0:
            return f"Postgres not available {ROLLOUT_TIMEOUT_SECONDS} s after it was scaled back", False
        deadline = self.clock() + READY_TIMEOUT_SECONDS
        while self.clock() < deadline:
            if self._ready():
                return "Postgres back; the server is ready", True
            self.sleep(10)
        return f"Postgres back; the server not ready after {READY_TIMEOUT_SECONDS} s", False

    def _postgres_away(self, item):
        entry = self.record.open("postgres_away", "postgres", away_seconds=POSTGRES_AWAY_SECONDS)
        outcome, well = "interrupted", True
        started = self.clock()
        try:
            code, _out, err = self._call(postgres_scale_args(0))
            if code != 0:
                outcome = f"the scale to zero failed: {err.strip()[:200]}"
            else:
                self._wait(POSTGRES_AWAY_SECONDS, step=10)
                away = int(self.clock() - started)
                outcome = f"away {away} s" + (", cut short by a stop" if self.stop_requested() else "")
        finally:
            # Whatever happened, Postgres is brought back before the window closes.
            restored, well = self.restore_postgres()
            outcome = f"{outcome}; {restored}"
            closed = self._close(entry, outcome, recovered=well)
            if not well:
                self.halted = outcome
                self.out("The deployment did not recover. The schedule stops. Bring it back by hand:\n"
                         f"  kubectl --kubeconfig {self.kube.kubeconfig} "
                         + " ".join(postgres_scale_args(1)))
            self.out(f"postgres_away: {outcome}")
        return closed

    # fill_capacity

    def _fill_capacity(self, item):
        key = self.keys.get(item.tenant)
        if not key:
            self.record.log("skipped", kind=item.kind, target=item.tenant, reason="no write key")
            return None
        entry = self.record.open("fill_capacity", item.tenant, tenant=item.tenant, runs=FILL_RUNS,
                                 workflow=workload.HOLD_WORKFLOW)
        statuses = []
        try:
            for _ in range(FILL_RUNS):
                if self.stop_requested():
                    break
                status, _body = self.api.call("POST", hold_path(item.tenant), key=key,
                                              body={"context": {"hold_seconds": workload.HOLD_SECONDS}})
                statuses.append(status)
        finally:
            accepted = sum(1 for s in statuses if isinstance(s, int) and 200 <= s < 300)
            outcome = f"{accepted} of {FILL_RUNS} held runs accepted"
            closed = self._close(entry, outcome, statuses=statuses)
            self.out(f"fill_capacity of {item.tenant}: {outcome}")
        return closed

    # -- the schedule ------------------------------------------------------

    def recover_open(self):
        """Close and restore the disturbances an earlier runner left open.
        Not while another process holds a disturbance."""
        open_entries = self.record.open_entries()
        if not open_entries:
            return []
        if not self.lock.take("recover"):
            self.record.log("skipped", kind="recover", target="",
                            reason="another disturbance is running")
            return []
        closed = []
        try:
            for entry in open_entries:
                outcome, well = "closed by the runner after it was left open", True
                if entry.get("kind") == "postgres_away":
                    restored, well = self.restore_postgres()
                    outcome = f"{outcome}; {restored}"
                closed.append(self._close(entry, outcome, recovered=well))
                if not well:
                    self.halted = outcome
        finally:
            self.lock.release()
        return closed

    def run(self, until=None):
        """Run the schedule until a stop, a halt, or the time `until`."""
        if self.stop_requested():
            os.unlink(self.stop_path)
            self.record.log("stop file removed at start")
        self.recover_open()
        self.record.log("schedule started")
        last_at = None
        try:
            while not self.halted and not self.stop_requested():
                # Never the same planned disturbance twice, even when it took
                # no time.
                now = self.clock()
                item = next_planned(now if last_at is None else max(now, last_at + 1))
                last_at = item.at
                if until is not None and item.at > until:
                    break
                self.out(f"next: {item.describe()}")
                self._wait(item.at - self.clock(), step=30)
                if self.stop_requested():
                    break
                try:
                    self.disturb(item)
                except Refused as exc:
                    self.halted = str(exc)
                    self.out(f"refused: {exc}")
        except KeyboardInterrupt:
            self.out("interrupted")
        finally:
            # A disturbance cut by Ctrl-C is restored and closed here.
            self.recover_open()
            if self.stop_requested():
                os.unlink(self.stop_path)
            self.record.log("schedule stopped", reason=self.halted or "asked to stop")
        return 1 if self.halted else 0


def request_stop(stop_path=None):
    """Ask a running schedule to stop. It ends the current disturbance first."""
    path = stop_path or os.path.join(hostdir(), "disturb.stop")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(format_time(time.time()) + "\n")
    return path


def read_settings(path):
    """The runner's settings: the server URL and a write key for each tenant."""
    with open(path, "r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict) or not value.get("server") or not value.get("keys"):
        raise ValueError(f"{path} has no server or no keys")
    return value
