"""Step definitions of the chapter "tenant_code" (CLOACINA-T-1079).

One scenario for each claim of the ADR CLOACINA-A-0019: on a server, tenant
code runs only on the tenant's agents and in the tenant's compiler. The
chapter runs on the standing deployment (`angreal test bdd standing
--chapter tenant_code`), on the `StandingWorld`.

The chapter has a tenant of its own, `team_d`, with its keys, its compiler
and its agent, so a scenario can take that tenant's agent away without
touching the tenants of the standing workload. `public` gets a key, a
compiler and an agent for the scenario of claim 7.

How a mark is read. The fixtures `tenant-code-*` leave a mark wherever their
code runs: load-time code, an import, a task, a poll trigger, a graph node, a
poll function. A mark holds names only (the program, the host name, the
credential names of the environment), never a value. Each mark scenario
first sees the mark where the code must run (a process on an agent, read from
the run's context or the reactor's fires), then looks for it where it must
not run: the log of every server pod, the log of the compiler's own process,
and the libraries the server process has open (`/proc/1/maps` of each server
pod). The compiler's describe child runs a package's load-time code by design
(CLOACINA-A-0019 decision 6); its stderr is captured by the compiler and is
not in the compiler pod's log, so a mark in that log is the compiler's own
process.

Where a step reads a channel that is not the public API (a pod log, a pod's
`/proc`, Postgres), it says so: that is a fact about the harness.
"""

from __future__ import annotations

import json
import subprocess
import time

from .gherkin import StepRegistry

steps = StepRegistry()

CHAPTER = "tenant_code"
FEATURES = ("features/tenant_code/tenant_code_boundary.feature",)

# The chapter's own tenant. A schema name allows no hyphen.
TEAM_D = "team_d"
PUBLIC = "public"
TENANT_OF = {"team D": TEAM_D, "public": PUBLIC}

RUST_FIXTURE = "tenant-code-mark-rust"
RUST_WORKFLOW = "tenant_code_mark"
PY_FIXTURE = "tenant-code-mark-py"
PY_WORKFLOW = "tenant_code_mark_py"
GRAPH_FIXTURE = "tenant-code-graph-py"
GRAPH_REACTOR = "tenant_code_graph_rx"
POLLING_FIXTURE = "tenant-code-polling-py"
NATIVE_FIXTURE = "demo-kafka-stream-rust"
WASM_FIXTURE = "demo-constructor-rust"
WASM_WORKFLOW = "constructor_demo"

LOAD_MARK = "TENANT_CODE_LOAD_MARK"
IMPORT_MARK = "TENANT_CODE_IMPORT_MARK"
TASK_MARK = "TENANT_CODE_TASK_MARK"
POLL_MARK = "TENANT_CODE_POLL_MARK"
GRAPH_MARK = "TENANT_CODE_GRAPH_MARK"
ACCUMULATOR_MARK = "TENANT_CODE_ACCUMULATOR_MARK"
MARK_OF = {"Rust package": LOAD_MARK, "Python package": IMPORT_MARK}

ORCHESTRATOR_PROCESSES = ("cloacina-server", "cloacina-compiler")
TERMINAL = ("Completed", "Failed", "Cancelled")

# The tables of a tenant's build role (security-model.md, "What a tenant's
# build can reach").
BUILD_TABLES = ("workflow_packages", "workflow_registry", "package_providers", "package_artifacts")

# Library paths of the server image itself. A mapped library outside them is
# reported.
SYSTEM_LIBRARY_DIRS = ("/usr/", "/lib/", "/lib64/")


# --- the deployment ----------------------------------------------------------

def _registry_ref(world, image):
    return f"{world._registry_k8s()}/{image}:{world.tag}"


def _wait(seconds, check, every=5):
    """Call `check` until it gives something true, for at most `seconds`."""
    deadline = time.time() + seconds
    while True:
        value = check()
        if value or time.time() >= deadline:
            return value
        time.sleep(every)


def _pods(world, namespace, selector=None):
    """name -> pod, for the pods of a namespace (with a label selector)."""
    args = ["get", "pods", "-n", namespace]
    if selector:
        args += ["-l", selector]
    data = world.kubectl_json(*args) or {}
    return {pod["metadata"]["name"]: pod for pod in data.get("items", [])}


def _server_pods(world):
    pods = _pods(world, world.ns, "app.kubernetes.io/name=cloacina-server")
    assert pods, "no server pod found: a look for a mark there would be a false absence"
    return pods


def _compiler_pods(world, tenant):
    """The pods of the compiler that serves a tenant: (namespace, name, pod)."""
    if world.chart_compilers:
        namespace = world.tenant_namespace(tenant)
        return [(namespace, name, pod) for name, pod in _pods(world, namespace).items()
                if name.startswith("cloacina-compiler")]
    if tenant == PUBLIC:
        # With the fleet compilers off, the chart's own compiler serves public.
        return [(world.ns, name, pod) for name, pod in _pods(world, world.ns).items()
                if "-compiler" in name and not name.startswith("cloacina-compiler-")]
    prefix = "cloacina-compiler-" + tenant.replace("_", "-") + "-"
    return [(world.ns, name, pod) for name, pod in _pods(world, world.ns).items()
            if name.startswith(prefix)]


def _agent_pod_names(world, tenant):
    return {name for name in _pods(world, world.tenant_namespace(tenant))
            if "compiler" not in name}


def _live_agents(world, tenant):
    _, roster = world.api("GET", "/v1/agents", key=world.bootstrap_key, expect=(200,))
    items = roster.get("items", []) if isinstance(roster, dict) else []
    return [agent for agent in items if agent.get("tenant_id") == tenant]


def _set_agents(world, tenant, count):
    """Bring a tenant to `count` agents: the driver's Deployment, or the
    server's actuator when it makes the agents."""
    if not world.chart_compilers:
        world.kubectl("scale", "deploy/cloacina-agent", "-n", world.tenant_namespace(tenant),
                      f"--replicas={count}")
        return
    for _ in range(4):
        _, fleet = world.api("GET", f"/v1/tenants/{tenant}/fleet", key=world.bootstrap_key,
                             expect=(200,))
        desired = int((fleet or {}).get("desired_count") or 0)
        if desired == count:
            return
        route = "provision" if desired < count else "deprovision"
        world.api("POST", f"/v1/tenants/{tenant}/fleet/{route}", key=world.bootstrap_key,
                  expect=(200, 201, 202, 204))
    raise AssertionError(f"{tenant} did not reach a desired count of {count} agents")


def _ensure_tenant(world, tenant):
    """Keys, a compiler and one live agent for a tenant. Called again on a
    deployment that has them, it only checks the agent."""
    if tenant not in world.keys:
        if tenant != PUBLIC:
            code, body = world.api("POST", "/v1/tenants", key=world.bootstrap_key,
                                   body={"name": tenant}, expect=None)
            if code not in (200, 201, 409) and "exists" not in str(body):
                raise AssertionError(f"creating tenant {tenant} -> {code}: {str(body)[:300]}")
        world.api("POST", f"/v1/tenants/{tenant}/limits", key=world.bootstrap_key,
                  body={"max_agents": 2}, expect=(200, 201))
        for role, holder in (("write", world.keys), ("admin", world.admin_keys)):
            _, created = world.api("POST", f"/v1/tenants/{tenant}/keys", key=world.bootstrap_key,
                                   body={"name": f"{tenant}-tenant-code-{role}", "role": role},
                                   expect=(200, 201))
            holder[tenant] = created["key"]
        if not world.chart_compilers:
            if tenant != PUBLIC:
                world.deploy_compiler(tenant, _registry_ref(world, "cloacina-compiler"))
            world.deploy_agent(tenant, _registry_ref(world, "cloacina-agent"))
    if not _live_agents(world, tenant):
        _set_agents(world, tenant, 1)
    have = _wait(240, lambda: _live_agents(world, tenant))
    assert have, f"{tenant} has no registered agent after 240 s"


# --- runs ------------------------------------------------------------------------

def _execute(world, tenant, workflow, timeout_s=180):
    """Start a run by the API, retried until the tenant's runner has loaded
    the workflow. Returns the execution id."""
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        code, body = world.api("POST", f"/v1/tenants/{tenant}/workflows/{workflow}/execute",
                               key=world.keys[tenant], body={"context": {}}, expect=None)
        if 200 <= code < 300 and isinstance(body, dict):
            found = body.get("execution_id") or body.get("id")
            if found:
                return found
        last = (code, str(body)[:200])
        time.sleep(3)
    raise AssertionError(f"no run of {workflow} in {tenant} started in {timeout_s} s; last: {last}")


def _status(world, tenant, execution_id, timeout_s=300):
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        code, body = world.api("GET", f"/v1/tenants/{tenant}/executions/{execution_id}",
                               key=world.keys[tenant], expect=None)
        if code == 200 and isinstance(body, dict):
            last = body.get("status")
            if last in TERMINAL:
                return last
        time.sleep(3)
    raise AssertionError(f"run {execution_id} did not finish in {timeout_s} s; last status {last!r}")


def _run_to_end(world, tenant, workflow):
    """Run a workflow to its end. Returns (status, context of the run)."""
    execution_id = _execute(world, tenant, workflow)
    status = _status(world, tenant, execution_id)
    # Read from the run's context in postgres: the API has no field for it.
    return status, world.execution_context(tenant, execution_id)


# --- where code ran ------------------------------------------------------------

def _on_an_agent(world, tenant, facts, what):
    """Assert that a mark was made in a process on an agent of the tenant."""
    assert isinstance(facts, dict) and facts, f"{what}: no mark: the code did not run, or the run lost it"
    names = _agent_pod_names(world, tenant)
    world.note(f"{what}: process {facts.get('process')} on {facts.get('hostname')}, "
               f"credential names: {facts.get('orchestrator_credentials_in_environment') or 'none'}")
    assert facts.get("process") not in ORCHESTRATOR_PROCESSES, (
        f"{what} ran in the process {facts.get('process')}")
    assert not facts.get("orchestrator_credentials_in_environment"), (
        f"{what} ran where the orchestrator's credentials are: "
        f"{facts.get('orchestrator_credentials_in_environment')}")
    assert facts.get("hostname") in names, (
        f"{what} ran on {facts.get('hostname')}, which is not a pod of {tenant}'s agents "
        f"({', '.join(sorted(names)) or 'none'})")


def _log_marks(world, namespace, pod, marker):
    """The marked lines of one pod's log, parsed."""
    proc = world.kubectl("logs", "-n", namespace, pod, "--all-containers=true", check=False)
    found = []
    seen = set()
    for line in ((proc.stdout or "") + (proc.stderr or "")).splitlines():
        line = line.strip()
        start = line.find(marker)
        if start < 0 or line in seen:
            continue
        seen.add(line)
        try:
            found.append(json.loads(line[start + len(marker):].strip()))
        except json.JSONDecodeError:
            found.append({"unparsed": True})
    return found


def _no_mark_in_server(world, marker, what):
    seen = []
    for name in sorted(_server_pods(world)):
        marks = _log_marks(world, world.ns, name, marker)
        world.note(f"server pod {name}: {len(marks)} line(s) with the mark of {what}")
        seen.extend(f"{name} (process {mark.get('process')})" for mark in marks)
    assert not seen, f"the mark of {what} is in the server's log: {'; '.join(seen)}"


def _no_mark_in_compiler(world, tenant, marker, what):
    pods = _compiler_pods(world, tenant)
    assert pods, f"no compiler pod of {tenant} found: a look there would be a false absence"
    seen = []
    for namespace, name, _pod in pods:
        marks = _log_marks(world, namespace, name, marker)
        world.note(f"compiler pod {name}: {len(marks)} line(s) with the mark of {what}")
        seen.extend(f"{name} (process {mark.get('process')}, describe mode "
                    f"{mark.get('describe_mode')})" for mark in marks)
    assert not seen, f"the mark of {what} is in the log of the compiler's own process: {'; '.join(seen)}"


def _foreign_libraries(maps_text):
    """The mapped libraries of a process that are not of the image itself:
    a `.so` outside the system folders, or executable memory from a memfd
    that is not a WASM image. Names only."""
    found = set()
    for line in maps_text.splitlines():
        parts = line.split(None, 5)
        if len(parts) < 6:
            continue
        perms, path = parts[1], parts[5].strip()
        if path.startswith("/memfd:"):
            if "x" in perms and "wasm" not in path:
                found.add(path)
            continue
        if not path.startswith("/"):
            continue
        name = path.rsplit("/", 1)[-1]
        if (".so" in name) and not path.startswith(SYSTEM_LIBRARY_DIRS):
            found.add(path)
    return sorted(found)


# --- psql with its errors ----------------------------------------------------------

def _psql(world, sql):
    """One SQL text in the deployment's postgres, as its admin. Returns
    (exit code, stdout, stderr): a refusal by Postgres is on stderr."""
    target = world._postgres_target()
    proc = world.kubectl("exec", target, "-n", world.ns, "--",
                         "env", "PGPASSWORD=cloacina", "psql", "-U", "cloacina", "-d", "cloacina",
                         "-v", "ON_ERROR_STOP=1", "-tAc", sql, check=False)
    return proc.returncode, (proc.stdout or "").strip(), (proc.stderr or "").strip()


def _roles_of_pod(world, pod):
    """The Postgres roles of the connections from one pod, by its IP."""
    ip = (pod.get("status") or {}).get("podIP")
    assert ip, f"pod {pod['metadata']['name']} has no IP"
    code, out, err = _psql(world, "SELECT DISTINCT usename FROM pg_stat_activity "
                                  f"WHERE client_addr = '{ip}'::inet ORDER BY 1")
    assert code == 0, f"reading pg_stat_activity failed: {err[:200]}"
    return [line for line in out.splitlines() if line]


def _refused(world, role, sql):
    """Run `sql` as `role` (SET ROLE). Returns (refused, what Postgres said)."""
    code, _out, err = _psql(world, f'SET ROLE "{role}"; {sql}')
    refused = code != 0 and "permission denied" in err
    return refused, (err.splitlines()[-1] if err else f"exit {code}")


# --- Background ------------------------------------------------------------------

@steps.given("a shared orchestrator runs in Kubernetes with tenant compilers and agents")
def bg_orchestrator(world):
    code, _ = world.api("GET", "/v1/agents", key=world.bootstrap_key, expect=None)
    assert code == 200, f"the orchestrator's agent roster answered {code}"
    world.note("compilers and agents made by the server's actuator: "
               f"{'yes' if world.chart_compilers else 'no, by the driver'}")


@steps.given("team D has its own tenant, keys, compiler and agent")
def bg_team_d(world):
    _ensure_tenant(world, TEAM_D)


# --- packages in service ----------------------------------------------------------------

@steps.given(r"(team D|public) has the (Rust|Python) mark package in service")
def given_mark_package(world, who, language):
    tenant = TENANT_OF[who]
    fixture = RUST_FIXTURE if language == "Rust" else PY_FIXTURE
    row = world.ensure_version(tenant, fixture, fixture, "0.1.0", key=world.keys[tenant])
    world.note(f"{tenant}: {fixture} {row.get('version')} in service, build {row.get('build_status')}")


@steps.given("team D has the Rust mark package and the Python mark package in service")
def given_both_mark_packages(world):
    for language in ("Rust", "Python"):
        given_mark_package(world, "team D", language)


@steps.given("team D has the graph mark package in service")
def given_graph_package(world):
    world.ensure_version(TEAM_D, GRAPH_FIXTURE, GRAPH_FIXTURE, "0.1.0", key=world.keys[TEAM_D])


@steps.given("the tenant public has a key, a compiler and an agent")
def given_public(world):
    _ensure_tenant(world, PUBLIC)
    assert _compiler_pods(world, PUBLIC) or world.chart_compilers, (
        "no compiler serves public: the chart's compiler is off and no fleet compiler is set")


# --- Scenarios 1, 2, 7: load-time code and import --------------------------------------

@steps.when(r"(team D|public) runs the (Rust|Python) mark workflow")
def when_run_mark(world, who, language):
    tenant = TENANT_OF[who]
    workflow = RUST_WORKFLOW if language == "Rust" else PY_WORKFLOW
    status, context = _run_to_end(world, tenant, workflow)
    world.note(f"{tenant}: the run of {workflow} ended {status}")
    assert status == "Completed", f"the run of {workflow} in {tenant} ended {status}"
    world.scratch["tenant"] = tenant
    world.scratch["context"] = context
    world.scratch["language"] = language


@steps.then(r"the task of the run left its mark in a process on an agent of (team D|public)")
def then_task_on_agent(world, who):
    _on_an_agent(world, TENANT_OF[who], world.scratch["context"].get("task_mark"), "the task")


@steps.then(r"the package's (load-time code|import) ran in that same process")
def then_load_in_task_process(world, kind):
    context = world.scratch["context"]
    key = "load_mark_in_this_process" if kind == "load-time code" else "import_mark_in_this_process"
    loaded = context.get(key)
    task = context.get("task_mark") or {}
    # The control: the mark is made, so its absence elsewhere means something.
    assert isinstance(loaded, dict) and loaded.get("moment") in ("load", "import"), (
        f"the package's {kind} left no mark in the process of its task: the mark itself is broken")
    # The pid can differ from the task's: a worker forked after the load
    # keeps the facts of the process that loaded.
    world.note(f"{kind} ran in process {loaded.get('process')} (pid {loaded.get('pid')}) "
               f"on {loaded.get('hostname')}; the task ran in pid {task.get('pid')}")
    assert loaded.get("hostname") == task.get("hostname"), (
        f"the {kind} mark is of another pod than the task's: {loaded} / {task}")
    assert loaded.get("process") not in ORCHESTRATOR_PROCESSES, (
        f"the {kind} ran in the process {loaded.get('process')}")


@steps.then(r"no log line of the server carries the mark of the (Rust package|Python package)")
def then_no_mark_server(world, what):
    _no_mark_in_server(world, MARK_OF[what], f"the {what}")


@steps.then(r"no log line of the compiler's own process carries the mark of the (Rust package|Python package)")
def then_no_mark_compiler(world, what):
    _no_mark_in_compiler(world, world.scratch.get("tenant", TEAM_D), MARK_OF[what], f"the {what}")


@steps.then("the server process has no library of a tenant open")
def then_no_foreign_library(world):
    # The server's list of loaded libraries, read from outside: /proc/1/maps
    # of each server pod. It needs `cat` in the server image.
    found = {}
    for name in sorted(_server_pods(world)):
        proc = world.kubectl("exec", "-n", world.ns, name, "--", "cat", "/proc/1/maps", check=False)
        assert proc.returncode == 0, (
            f"could not read /proc/1/maps of {name}: {(proc.stderr or '').strip()[:200]}")
        libraries = _foreign_libraries(proc.stdout or "")
        world.note(f"server pod {name}: {len(libraries)} mapped librar(y/ies) outside the image")
        if libraries:
            found[name] = libraries
    assert not found, f"the server process has libraries open that are not of its image: {found}"


@steps.then(r"(team D|public)'s compiler connects to Postgres with the role ([a-z0-9_]+\$build)")
def then_compiler_role(world, who, role):
    tenant = TENANT_OF[who]
    pods = _compiler_pods(world, tenant)
    assert pods, f"no compiler pod of {tenant} found"
    seen = {}
    for _namespace, name, pod in pods:
        seen[name] = _roles_of_pod(world, pod)
    world.note(f"Postgres roles of the connections of {tenant}'s compiler: {seen}")
    if not world.chart_compilers:
        world.note("the driver applied this compiler with the admin URL; the server makes "
                   "compilers with the build role only with --chart-actuator")
    wrong = {name: roles for name, roles in seen.items() if roles != [role]}
    assert not wrong, f"{tenant}'s compiler connects with {wrong}, not only with {role}"


# --- Scenario 3: the default chart -----------------------------------------------------------

def _helm_template(world, *sets):
    args = ["helm", "template", "tenant-code-check", str(world.root / "charts" / "cloacina-server")]
    for value in sets:
        args += ["--set", value]
    return subprocess.run(args, capture_output=True, text=True, check=False)


@steps.when("the chart is rendered with its default values")
def when_render_default(world):
    # The bundled database is the only value given: a render with no
    # database at all stops at that check first.
    world.scratch["render"] = _helm_template(world, "postgresql.enabled=true")


@steps.then("the render is refused until the install says where the agents come from")
def then_render_refused(world):
    proc = world.scratch["render"]
    world.note(f"helm template with the default values: exit {proc.returncode}: "
               f"{(proc.stderr or '').strip()[-200:]}")
    assert proc.returncode != 0, "the chart renders with no fleet and no single trust"
    assert "runs tenant tasks only on agents" in (proc.stderr or ""), (
        f"the render failed for another reason: {(proc.stderr or '').strip()[-300:]}")


@steps.when("the chart is rendered with the Kubernetes actuator and nothing else")
def when_render_actuator(world):
    world.scratch["render"] = _helm_template(world, "postgresql.enabled=true",
                                             "fleet.actuator=kubernetes")


@steps.then("the server it renders does not run with single trust")
def then_render_no_single_trust(world):
    proc = world.scratch["render"]
    assert proc.returncode == 0, f"the render failed: {(proc.stderr or '').strip()[-300:]}"
    text = proc.stdout or ""
    assert "--single-trust" not in text, "the rendered server runs with --single-trust"
    assert "CLOACINA_SINGLE_TRUST" not in text, "the rendered server sets CLOACINA_SINGLE_TRUST"


@steps.then("the server of this deployment does not run with single trust")
def then_live_no_single_trust(world):
    data = world.kubectl_json("get", "deploy", f"{world.release}-cloacina-server", "-n", world.ns)
    assert data, "the server Deployment was not found"
    wrong = []
    for container in data["spec"]["template"]["spec"].get("containers", []):
        if "--single-trust" in (container.get("args") or []) + (container.get("command") or []):
            wrong.append(f"{container['name']}: --single-trust")
        for env in container.get("env") or []:
            if env.get("name") == "CLOACINA_SINGLE_TRUST" and str(env.get("value", "")).lower() not in ("", "false", "0"):
                wrong.append(f"{container['name']}: CLOACINA_SINGLE_TRUST={env.get('value')}")
    assert not wrong, f"the server runs with single trust: {wrong}"


@steps.when("team D runs both mark workflows")
def when_run_both(world):
    world.scratch["runs"] = {}
    for workflow in (RUST_WORKFLOW, PY_WORKFLOW):
        status, context = _run_to_end(world, TEAM_D, workflow)
        world.scratch["runs"][workflow] = (status, context)
        world.note(f"{workflow}: {status}")


@steps.then("every task of the mark packages ran on an agent")
def then_every_task_on_agent(world):
    for workflow, (status, context) in sorted(world.scratch["runs"].items()):
        assert status == "Completed", f"the run of {workflow} ended {status}"
        _on_an_agent(world, TEAM_D, context.get("task_mark"), f"the task of {workflow}")


# --- Scenario 4: a graph firing with no agent ----------------------------------------------

def _fires(world):
    code, body = world.api("GET", f"/v1/health/reactors/{GRAPH_REACTOR}/fires?limit=20",
                           key=world.keys[TEAM_D], expect=None)
    assert code == 200, f"the fires of {GRAPH_REACTOR} answered {code}"
    return body.get("items", []) if isinstance(body, dict) else []


def _fire_and_wait(world, timeout_s=240):
    """Fire the reactor and wait for the fire to be recorded. Returns it."""
    before = {fire.get("fired_at") for fire in _fires(world)}
    deadline = time.time() + 120
    while True:
        code, body = world.api("POST", f"/v1/health/reactors/{GRAPH_REACTOR}/fire",
                               key=world.keys[TEAM_D], body={"mode": "force_fire"}, expect=None)
        if code == 200 or time.time() >= deadline:
            break
        time.sleep(3)  # the reactor is not registered yet
    assert code == 200, f"firing {GRAPH_REACTOR} answered {code}: {str(body)[:200]}"
    found = _wait(timeout_s, lambda: [fire for fire in _fires(world)
                                      if fire.get("fired_at") not in before])
    assert found, f"the fire of {GRAPH_REACTOR} was not recorded in {timeout_s} s"
    return found[0]


def _graph_where(fire):
    """The facts the graph node returned, wherever the output put them."""
    def find(value):
        if isinstance(value, dict):
            if value.get("moment") == "graph":
                return value
            for inner in value.values():
                hit = find(inner)
                if hit:
                    return hit
        if isinstance(value, list):
            for inner in value:
                hit = find(inner)
                if hit:
                    return hit
        return None

    return find(fire.get("outputs"))


@steps.given("a firing of team D's reactor ran on an agent of team D")
def given_fire_on_agent(world):
    fire = _fire_and_wait(world)
    world.note(f"fire with an agent: ok={fire.get('ok')}, error={fire.get('error')}")
    assert fire.get("ok"), f"the firing with an agent failed: {fire.get('error')}"
    _on_an_agent(world, TEAM_D, _graph_where(fire), "the graph node")


@steps.when("team D has no live agent")
def when_no_agent(world):
    _set_agents(world, TEAM_D, 0)
    gone = _wait(300, lambda: not _live_agents(world, TEAM_D))
    world.note(f"team D's agents in the roster after the scale to 0: {len(_live_agents(world, TEAM_D))}")
    assert gone, "team D still has an agent in the roster after 300 s"


@steps.when("team D fires its reactor")
def when_fire(world):
    world.scratch["fire"] = _fire_and_wait(world)


@steps.then("the firing failed with a reason that names the missing agent")
def then_fire_failed(world):
    fire = world.scratch["fire"]
    error = str(fire.get("error") or "")
    world.note(f"fire with no agent: ok={fire.get('ok')}, error: {error[:300]}")
    assert not fire.get("ok"), "the firing with no agent of the tenant succeeded"
    assert "agent" in error, f"the reason does not name the missing agent: {error[:300]}"
    assert _graph_where(fire) is None, "the firing with no agent has the graph node's output"


@steps.then("no log line of the server carries the mark of the graph")
def then_no_graph_mark(world):
    _no_mark_in_server(world, GRAPH_MARK, "the graph node")


# --- Scenario 5: a poll trigger ---------------------------------------------------------------

@steps.when("the poll trigger of the Rust mark package has fired a run")
def when_poll_fired(world):
    def polled():
        code, body = world.api("GET", f"/v1/tenants/{TEAM_D}/executions?workflow={RUST_WORKFLOW}&limit=20",
                               key=world.keys[TEAM_D], expect=None)
        if code != 200 or not isinstance(body, dict):
            return None
        for run in body.get("items", []):
            if run.get("status") != "Completed":
                continue
            context = world.execution_context(TEAM_D, run["id"])
            if isinstance(context.get("poll_mark"), dict):
                return context
        return None

    # The trigger polls each 60 s.
    context = _wait(300, polled, every=10)
    assert context, "no completed run fired by the poll trigger in 300 s"
    world.scratch["context"] = context


@steps.then("the poll trigger's code ran in a process on an agent of team D")
def then_poll_on_agent(world):
    _on_an_agent(world, TEAM_D, world.scratch["context"].get("poll_mark"), "the poll trigger")


@steps.then("no log line of the server carries the poll trigger's mark")
def then_no_poll_mark(world):
    _no_mark_in_server(world, POLL_MARK, "the poll trigger")


# --- Scenario 6: native provider, Python accumulator, WASM provider -----------------------------

def _stage_with_workspace(world, fixture, replace=None):
    """Stage a fixture whose manifests name `__WORKSPACE__`: it becomes the
    workspace the compiler image carries (`--dev-workspace /workspace`)."""
    directory = world.stage_fixture(fixture)
    for path in list(directory.rglob("Cargo.toml")) + list(directory.rglob("package.toml")) + \
            list(directory.rglob("*.rs")):
        text = path.read_text()
        changed = text.replace("__WORKSPACE__", "/workspace")
        for old, new in (replace or {}).items():
            changed = changed.replace(old, new)
        if changed != text:
            path.write_text(changed)
    return directory


def _publish_and_settle(world, directory, name, version):
    archive = world.pack(directory)
    code, out, err = world.publish_archive(archive, tenant=TEAM_D, key=world.keys[TEAM_D])
    assert code == 0, f"the upload of {name} {version} was refused: {(err or out)[-300:]}"
    return world.wait_build_status(TEAM_D, name, {"success", "failed"}, key=world.keys[TEAM_D],
                                   version=version)


@steps.when("team D publishes a package whose stream source is a native provider")
def when_publish_native(world):
    directory = _stage_with_workspace(world, NATIVE_FIXTURE)
    world.scratch["row"] = _publish_and_settle(world, directory, NATIVE_FIXTURE, "0.1.1")
    world.scratch["refused_names"] = ("cloacina-provider-kafka", "kafka_alpha")


@steps.when("team D publishes a Python package with a polling accumulator")
def when_publish_polling(world):
    directory = world.stage_fixture(POLLING_FIXTURE)
    world.scratch["row"] = _publish_and_settle(world, directory, POLLING_FIXTURE, "0.1.0")
    world.scratch["refused_names"] = ("tc_poll",)


@steps.then(r"the build of that package fails with a reason that names its (native provider|polling accumulator)")
def then_refused(world, kind):
    row = world.scratch["row"]
    error = str(row.get("build_error") or "")
    world.note(f"{kind}: build_status={row.get('build_status')}, active={row.get('active')}, "
               f"build_error: {error[-300:]}")
    assert row.get("build_status") == "failed", (
        f"the package with a {kind} is {row.get('build_status')}, not refused")
    assert not row.get("active"), f"the refused package with a {kind} is in service"
    assert any(name in error for name in world.scratch["refused_names"]), (
        f"the reason does not name the {kind} ({', '.join(world.scratch['refused_names'])}): {error[-300:]}")


@steps.then("no log line of the server carries the mark of the polling accumulator")
def then_no_accumulator_mark(world):
    _no_mark_in_server(world, ACCUMULATOR_MARK, "the polling accumulator")


@steps.when("team D publishes a package whose task uses a WASM provider")
def when_publish_wasm(world):
    directory = _stage_with_workspace(world, WASM_FIXTURE)
    world.scratch["row"] = _publish_and_settle(world, directory, WASM_FIXTURE, "0.1.2")


@steps.then("that package is built and in service")
def then_wasm_built(world):
    row = world.scratch["row"]
    world.note(f"WASM provider package: build_status={row.get('build_status')}")
    assert row.get("build_status") == "success", (
        f"the package with a WASM provider is {row.get('build_status')}: "
        f"{str(row.get('build_error'))[-300:]}")
    active = _wait(60, lambda: (world.active_version(TEAM_D, WASM_FIXTURE, key=world.keys[TEAM_D]) or {})
                   .get("version") == "0.1.2")
    assert active, "the package with a WASM provider did not come into service"


@steps.then("a run of it reads the file its grant allows")
def then_wasm_reads(world):
    status, context = _run_to_end(world, TEAM_D, WASM_WORKFLOW)
    world.note(f"WASM provider run: {status}, bytes read inside the grant: "
               f"{context.get('sandbox_read_bytes')}")
    assert status == "Completed", f"the run with the WASM provider ended {status}"
    assert int(context.get("sandbox_read_bytes") or 0) > 0, "the provider read nothing inside its grant"


@steps.when("team D publishes a version whose WASM provider reads outside its grant")
def when_publish_wasm_outside(world):
    # The grant stays `ro:/etc`; the path moves out of it.
    directory = _stage_with_workspace(world, WASM_FIXTURE, replace={
        'version = "0.1.2"': 'version = "0.1.3"',
        'path = "/etc/hostname"': 'path = "/proc/self/environ"',
    })
    row = _publish_and_settle(world, directory, WASM_FIXTURE, "0.1.3")
    assert row.get("build_status") == "success", (
        f"the version that reads outside its grant did not build: {str(row.get('build_error'))[-300:]}")
    active = _wait(60, lambda: (world.active_version(TEAM_D, WASM_FIXTURE, key=world.keys[TEAM_D]) or {})
                   .get("version") == "0.1.3")
    assert active, "version 0.1.3 did not come into service"


@steps.then("a run of that version fails, and nothing outside the grant was read")
def then_wasm_refused(world):
    status, context = _run_to_end(world, TEAM_D, WASM_WORKFLOW)
    world.note(f"run that reads outside the grant: {status}, bytes: {context.get('sandbox_read_bytes')}")
    assert status == "Failed", f"the run that reads outside the provider's grant ended {status}"
    assert not context.get("sandbox_read_bytes"), "the provider read a file outside its grant"


# --- Scenario 8: the compiler's credential ------------------------------------------------------

@steps.given("team D's compiler has built a package")
def given_compiler_built(world):
    given_mark_package(world, "team D", "Rust")


@steps.then(r"that role can read team D's build tables")
def then_build_role_reads_own(world):
    role = f"{TEAM_D}$build"
    refused, said = _refused(world, role, f"SELECT count(*) FROM {TEAM_D}.workflow_packages")
    world.note(f"{role} reads {TEAM_D}.workflow_packages: {'refused: ' + said if refused else 'yes'}")
    assert not refused, f"{role} cannot read its own build table: {said}"


@steps.then("Postgres refuses that role a read of another tenant's tables")
def then_build_role_other_tenant(world):
    role = f"{TEAM_D}$build"
    wrong = []
    for other in ("team_a", "team_b"):
        for table in ("workflow_packages", "workflow_executions"):
            refused, said = _refused(world, role, f"SELECT count(*) FROM {other}.{table}")
            world.note(f"{role} reads {other}.{table}: {said if refused else 'allowed'}")
            if not refused:
                wrong.append(f"{other}.{table}")
    assert not wrong, f"Postgres let {role} read: {', '.join(wrong)}"


@steps.then("Postgres refuses that role a read of the server's own tables in public and cloacina_global")
def then_build_role_public(world):
    role = f"{TEAM_D}$build"
    wrong = []
    # The global tables are in the schema `cloacina_global` (CLOACINA-T-1080);
    # `public` keeps the public tenant's own tables.
    for table in ("cloacina_global.api_keys", "cloacina_global.fleet_agents",
                  "public.tenant_secrets", "public.workflow_executions"):
        refused, said = _refused(world, role, f"SELECT count(*) FROM {table}")
        world.note(f"{role} reads {table}: {said if refused else 'allowed'}")
        if not refused and "does not exist" not in said:
            wrong.append(table)
    assert not wrong, f"Postgres let {role} read: {', '.join(wrong)}"


@steps.then("no build role has a right outside its own tenant's build tables")
def then_no_build_role_outside(world):
    # The same question as the checker's rule `tenant_code.build_role_reach`.
    from .suite.rules import tenant_code as rules_of_chapter

    code, out, err = _psql(world, f"SELECT coalesce(json_agg(q), '[]'::json) FROM "
                                  f"({rules_of_chapter.BUILD_ROLE_REACH_SQL}) q")
    assert code == 0, f"reading the rights of the build roles failed: {err[:200]}"
    wrong = rules_of_chapter.build_role_faults(json.loads(out or "[]"))
    world.note(f"build roles with a right outside their build tables: {len(wrong)}")
    assert not wrong, "; ".join(wrong[:20])


# --- Scenario 9: a package built before the change -------------------------------------------------

@steps.when("the stored description of the Rust mark package is removed, as for a package built before the change")
def when_remove_document(world):
    code, out, err = _psql(world, f"UPDATE {TEAM_D}.workflow_packages SET metadata_document = NULL, "
                                  "metadata_document_version = NULL, describe_error = NULL "
                                  f"WHERE package_name = '{RUST_FIXTURE}' AND build_status = 'success'")
    assert code == 0 and out.startswith("UPDATE") and out != "UPDATE 0", (
        f"no stored description was removed: {out or err[:200]}")
    world.note(f"removed in postgres: {out}")
    world.scratch["removed_at"] = time.time()


@steps.then("team D's compiler describes the package again with no publish")
def then_described_again(world):
    def described():
        code, out, _err = _psql(world, f"SELECT count(*) FROM {TEAM_D}.workflow_packages "
                                       f"WHERE package_name = '{RUST_FIXTURE}' AND build_status = 'success' "
                                       "AND metadata_document IS NOT NULL AND describe_error IS NULL")
        return code == 0 and out not in ("", "0")

    _, detail = world.api("GET", f"/v1/tenants/{TEAM_D}/workflows/{RUST_FIXTURE}",
                          key=world.keys[TEAM_D], expect=None)
    if isinstance(detail, dict):
        world.note(f"while it has no description: not_loaded_reason={detail.get('not_loaded_reason')}, "
                   f"describe_error={detail.get('describe_error')}")
    done = _wait(600, described, every=10)
    world.note(f"described again {int(time.time() - world.scratch['removed_at'])} s after the removal: {bool(done)}")
    assert done, "the compiler did not describe the package again in 600 s"


@steps.then("a new run of the Rust mark workflow completes on an agent of team D")
def then_runs_again(world):
    status, context = _wait(300, lambda: _try_run(world), every=10) or ("never started", {})
    assert status == "Completed", f"the run after the new description ended {status}"
    _on_an_agent(world, TEAM_D, context.get("task_mark"), "the task after the new description")


def _try_run(world):
    try:
        return _run_to_end(world, TEAM_D, RUST_WORKFLOW)
    except AssertionError:
        return None


# --- Scenario 10: the server's tenant pools ------------------------------------------------------

@steps.then("the server's connections for team D use the role team_d")
def then_pool_role(world):
    roles = {}
    for name, pod in sorted(_server_pods(world).items()):
        roles[name] = _roles_of_pod(world, pod)
    world.note(f"Postgres roles of the server's connections: {roles}")
    without = [name for name, found in roles.items() if TEAM_D not in found]
    assert len(without) < len(roles), (
        f"no server connection uses the role {TEAM_D}: the tenant's pool connects as {roles}")


@steps.then("Postgres refuses the role team_d a read of another tenant's tables")
def then_tenant_role_refused(world):
    wrong = []
    for other in ("team_a", "team_b"):
        refused, said = _refused(world, TEAM_D, f"SELECT count(*) FROM {other}.workflow_executions")
        world.note(f"{TEAM_D} reads {other}.workflow_executions: {said if refused else 'allowed'}")
        if not refused:
            wrong.append(other)
    assert not wrong, f"Postgres let the role {TEAM_D} read the tables of {', '.join(wrong)}"


@steps.then("the role team_d can read team D's own tables")
def then_tenant_role_own(world):
    refused, said = _refused(world, TEAM_D, f"SELECT count(*) FROM {TEAM_D}.workflow_executions")
    assert not refused, f"the role {TEAM_D} cannot read its own tables: {said}"
