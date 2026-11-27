"""Step definitions for the publishing behaviour spec (CLOACINA-T-1026).

Each step says what a team does or sees; the reach into the deployment lives
here, on the `PublishWorld` the step receives. A `Given`/`When` that fails
stops the scenario; a `Then` that fails is recorded and the scenario goes on,
so one run shows every expectation that did not hold.

Where a step reads a channel that is not the public API (an agent pod's log,
the execution context in postgres), it says so, because that is a fact about
the harness, not about the product.
"""

from __future__ import annotations

import json
import time

from .gherkin import StepRegistry
from .world import TENANTS

steps = StepRegistry()

# The chapter this file belongs to, and its feature files, from the root of
# the repository. `chapters.py` finds every `*_steps.py` by these two names.
CHAPTER = "publishing"
FEATURES = ("features/publishing/independent_publish.feature",)

# Fixtures and the names they install under.
REPORT_FIXTURE = "publish-report-rust"
REPORT_BROKEN_FIXTURE = "publish-report-broken-rust"
REPORT_PKG = "publish-report-rust"
REPORT_WORKFLOW = "publish_report"
# The name the broken 0.2.0 calls and does not define: rustc names it.
BROKEN_ERROR_MARK = "publish_report_total_that_does_not_exist"

ORDERS_FIXTURE = "publish-versions-py"
ORDERS_PKG = "publish-versions-py"
ORDERS_WORKFLOW = "publish_versions"

LEDGER_FIXTURE = "publish-ledger-py"
LEDGER_PKG = "publish-ledger-py"
LEDGER_WORKFLOW = "publish_ledger"

PROBE_FIXTURE = "publish-probe-py"
PROBE_WORKFLOW = "publish_probe"

TEAM_A, TEAM_B = TENANTS
LEDGER_SECRET = "ledger_key"
TEAM_A_SECRET = "report_key"
PROBE_IMPORT_MARKER = "PUBLISH_PROBE_IMPORT"
PROBE_REPORT_MARKER = "PUBLISH_PROBE_REPORT"


# --- small helpers ---------------------------------------------------------

def _run_workflow(world, tenant, workflow, *, key, context=None, timeout_s=180):
    """`cloacinactl workflow run`, retried until the runner has loaded the
    workflow. Returns the execution id."""
    args = ["-o", "json", "workflow", "run", workflow]
    if context is not None:
        ctx_path = world.home / f"ctx-{workflow}-{time.time()}.json"
        ctx_path.write_text(json.dumps(context))
        args += ["--context", str(ctx_path)]
    deadline = time.time() + timeout_s
    last = ""
    while time.time() < deadline:
        code, out, err = world.ctl(*args, key=key, tenant=tenant, check=False)
        if code == 0:
            try:
                exec_id = json.loads(out).get("execution_id")
            except json.JSONDecodeError:
                exec_id = out.strip().splitlines()[-1].strip() if out.strip() else ""
            if exec_id and len(exec_id) >= 32:
                return exec_id
        last = err.strip() or out.strip()
        time.sleep(2)
    raise AssertionError(f"workflow run {workflow} never succeeded in {timeout_s}s; last: {last}")


def _execution_count(world, tenant, workflow, *, key):
    code, body = world.api("GET", f"/v1/tenants/{tenant}/executions?workflow={workflow}",
                           key=key, expect=None)
    if code == 200 and isinstance(body, dict):
        return len(body.get("items", []))
    return 0


def _wait_schedule_fires(world, tenant, workflow, *, key, timeout_s=90):
    """Wait until the cron schedule of `workflow` produces at least one more
    execution than it has now. Returns True when it does."""
    start = _execution_count(world, tenant, workflow, key=key)
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if _execution_count(world, tenant, workflow, key=key) > start:
            return True
        time.sleep(3)
    return False


def _poll_execution_status(world, tenant, execution_id, terminal, *, key, timeout_s=300):
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        code, body = world.api("GET", f"/v1/tenants/{tenant}/executions/{execution_id}",
                               key=key, expect=None)
        if code == 200 and isinstance(body, dict):
            last = body.get("status")
            if last in terminal:
                return last
        time.sleep(3)
    raise AssertionError(f"execution {execution_id} never reached {terminal}; last {last!r}")


def _probe_report_from_log(world, tenant):
    """The probe task's report, read from the agent pod's log (the probe emits
    one marked JSON line). Returns (report, import_facts)."""
    log = world.agent_pod_logs(tenant)
    report, imp = None, None
    for line in log.splitlines():
        line = line.strip()
        if line.startswith(PROBE_REPORT_MARKER):
            report = json.loads(line[len(PROBE_REPORT_MARKER):].strip())
        elif line.startswith(PROBE_IMPORT_MARKER):
            imp = json.loads(line[len(PROBE_IMPORT_MARKER):].strip())
    if report is None:
        raise AssertionError("the probe task wrote no report line to the agent log — it did not run")
    return report, imp


# --- Background ------------------------------------------------------------

@steps.given("a shared orchestrator runs in Kubernetes with a compiler and agents")
def bg_orchestrator(world):
    code, body = world.api("GET", "/v1/agents", key=world.bootstrap_key, expect=(200,))
    items = body.get("items", []) if isinstance(body, dict) else []
    assert items, "no agents registered — the fleet did not come up"
    # Baseline for the no-restart check later.
    world.scratch["restart_baseline"] = world.server_restart_count()


@steps.given("team A and team B each have their own tenant, publish key, secret and agent")
def bg_tenants(world):
    for tenant, secret in ((TEAM_A, TEAM_A_SECRET), (TEAM_B, LEDGER_SECRET)):
        assert tenant in world.keys, f"no publish key for {tenant}"
        # A secret per tenant, so the probe has something of team B to try to read.
        world.api("PUT", f"/v1/tenants/{tenant}/secrets/{secret}", key=world.bootstrap_key,
                  body={"fields": {"token": f"{tenant}-only"}}, expect=None)
        world.api("POST", f"/v1/tenants/{tenant}/secrets", key=world.bootstrap_key,
                  body={"name": secret, "fields": {"token": f"{tenant}-only"}}, expect=None)
        _, roster = world.api("GET", "/v1/agents", key=world.bootstrap_key, expect=(200,))
        have = any(a.get("tenant_id") == tenant for a in roster.get("items", []))
        assert have, f"{tenant} has no registered agent"


# --- Scenario 1: build + validate + pack, no server ------------------------

@steps.given("team A keeps its report job and its orders job in its own repository")
def s1_repo(world):
    world.scratch["report_dir"] = world.stage_fixture(REPORT_FIXTURE, patch_crates=True)
    world.scratch["orders_dir"] = world.stage_fixture(ORDERS_FIXTURE)


@steps.when("team A builds, validates and packs both jobs with no server to talk to")
def s1_build(world):
    produced = {}
    # Point cloacinactl at a server that is not there, to prove these verbs are
    # local: build/validate/pack never call the orchestrator.
    for name, directory in (("report", world.scratch["report_dir"]),
                            ("orders", world.scratch["orders_dir"])):
        world.ctl("package", "validate", str(directory), key="unused")
        world.ctl("package", "build", str(directory), key="unused")
        out = world.home / f"s1-{name}.cloacina"
        world.ctl("package", "pack", str(directory), "--out", str(out), key="unused")
        assert out.exists(), f"{name}: pack produced no archive"
        produced[name] = out
        world.note(f"{name}: packed to {out.name} ({out.stat().st_size} bytes), no server contacted")
    world.scratch["packed"] = produced


@steps.then("each job becomes one package file that is ready to publish")
def s1_files(world):
    packed = world.scratch.get("packed", {})
    assert set(packed) == {"report", "orders"}, f"expected two archives, got {sorted(packed)}"
    for name, path in packed.items():
        assert path.exists() and path.stat().st_size > 0, f"{name}: empty or missing archive"


# --- Scenario 2: publish, compile, schedule starts, no restart -------------

@steps.given("team A has packed version 0.1.0 of its report job")
def s2_packed(world):
    # As the team ships it: version deps, no patch. The compiler resolves them.
    directory = world.stage_fixture(REPORT_FIXTURE)
    world.scratch["archive"] = world.pack(directory)
    world.scratch["restart_before"] = world.server_restart_count()


@steps.when("team A publishes the report job with its own key")
def s2_publish(world):
    code, out, err = world.publish_archive(world.scratch["archive"], tenant=TEAM_A,
                                           key=world.keys[TEAM_A])
    assert code == 0, f"publish failed: {err or out}"
    world.note(f"publish accepted with team A's write key -> {out.strip()[:80]}")


@steps.then("the orchestrator compiles the report job")
def s2_compiles(world):
    started = time.time()
    row = world.wait_build_status(TEAM_A, REPORT_PKG, {"success", "failed"},
                                  key=world.keys[TEAM_A], version="0.1.0")
    status = row.get("build_status")
    world.note(f"report job 0.1.0 build_status={status} after {int(time.time() - started)}s, "
               f"active={row.get('active')}")
    assert status == "success", (
        f"expected a successful build, got {status}: {str(row.get('build_error'))[-600:]}")


@steps.then("the report job runs on its schedule")
def s2_schedule(world):
    key = world.keys[TEAM_A]
    listed = len(world.schedules(TEAM_A, REPORT_WORKFLOW, key=key))
    fired = _wait_schedule_fires(world, TEAM_A, REPORT_WORKFLOW, key=key)
    count = _execution_count(world, TEAM_A, REPORT_WORKFLOW, key=key)
    world.note(f"after the publish and the build, with no other call for the tenant: "
               f"{listed} schedule(s) listed, {count} execution(s) in 90s (schedule fired: {fired})")
    if not fired:
        # Record what it takes to start it; the expectation above still failed.
        first = world.wake_tenant(TEAM_A, REPORT_WORKFLOW, key=key)
        after = _wait_schedule_fires(world, TEAM_A, REPORT_WORKFLOW, key=key, timeout_s=60)
        world.note(f"then one `execute` call for the tenant answered {first}; after it "
                   f"{len(world.schedules(TEAM_A, REPORT_WORKFLOW, key=key))} schedule(s) listed, "
                   f"schedule fired within 60s: {after}")
    assert fired, ("the report job's schedule did not start after the publish: no execution "
                   "within 90s until another call made the server start the tenant's runner")


@steps.then("the report job still runs on its schedule")
def s3_schedule(world):
    fired = _wait_schedule_fires(world, TEAM_A, REPORT_WORKFLOW, key=world.keys[TEAM_A])
    count = _execution_count(world, TEAM_A, REPORT_WORKFLOW, key=world.keys[TEAM_A])
    world.note(f"report job executions seen: {count} (schedule fired after the failed build: {fired})")
    assert fired, "the report job's cron schedule produced no execution within the window"


@steps.then("the orchestrator was not restarted")
def s2_norestart(world):
    before = world.scratch.get("restart_before", world.scratch.get("restart_baseline", 0))
    after = world.server_restart_count()
    world.note(f"server restart count: {before} -> {after}")
    assert after == before, f"the server restarted ({before} -> {after}) around the publish"


# --- Scenario 3: a build error leaves the running version in service -------

@steps.given("version 0.1.0 of team A's report job is in service")
def s3_v1(world):
    row = world.ensure_version(TEAM_A, REPORT_PKG, REPORT_FIXTURE, "0.1.0", key=world.keys[TEAM_A])
    world.scratch["v1_hash"] = row.get("content_hash")
    # "In service" includes a running schedule. The server starts a tenant's
    # schedules only after a first call for the tenant (scenario 2 records
    # that), so make that call here.
    if not world.schedules(TEAM_A, REPORT_WORKFLOW, key=world.keys[TEAM_A]):
        world.wake_tenant(TEAM_A, REPORT_WORKFLOW, key=world.keys[TEAM_A])
    assert _wait_schedule_fires(world, TEAM_A, REPORT_WORKFLOW, key=world.keys[TEAM_A]), (
        "the report job 0.1.0 does not run on its schedule, so there is nothing to keep in service")


@steps.when("team A publishes version 0.2.0 of the report job, which does not compile")
def s3_v2(world):
    directory = world.stage_fixture(REPORT_BROKEN_FIXTURE)
    archive = world.pack(directory)
    code, out, err = world.publish_archive(archive, tenant=TEAM_A, key=world.keys[TEAM_A])
    world.note(f"publish of the broken 0.2.0 returned exit {code}")
    assert code == 0, f"the upload itself should be accepted; the BUILD is what fails: {err or out}"


@steps.then("team A sees that the build failed, with the compiler's error")
def s3_failed(world):
    v2 = world.wait_build_status(TEAM_A, REPORT_PKG, {"success", "failed"},
                                 key=world.keys[TEAM_A], version="0.2.0")
    error = v2.get("build_error") or ""
    world.note(f"0.2.0 build_status={v2.get('build_status')}, active={v2.get('active')}, "
               f"build_error is {len(error)} characters")
    assert v2.get("build_status") == "failed", f"expected failed, got {v2.get('build_status')}"
    assert BROKEN_ERROR_MARK in error, (
        f"the build error does not carry the compiler's message ({BROKEN_ERROR_MARK}): {error[-400:]}")
    code, out, err = world.ctl("package", "versions", REPORT_PKG, key=world.keys[TEAM_A],
                               tenant=TEAM_A, check=False)
    world.note(f"`package versions {REPORT_PKG}` exit {code}: " +
               " | ".join(line.strip() for line in out.splitlines()[1:]))


@steps.then("team A's list of versions of the report job shows 0.1.0 in service")
def s3_v1_active(world):
    active = world.active_version(TEAM_A, REPORT_PKG, key=world.keys[TEAM_A])
    assert active is not None, "no active version found (versions route absent, or none active)"
    world.note(f"active version of the report job: {active.get('version')}")
    assert active.get("version") == "0.1.0", f"expected 0.1.0 in service, got {active.get('version')}"


# --- Scenario 4: a new version while an execution runs ---------------------

@steps.given("version 0.1.0 of team A's orders job is in service")
def s4_v1(world):
    world.ensure_version(TEAM_A, ORDERS_PKG, ORDERS_FIXTURE, "0.1.0", key=world.keys[TEAM_A])


@steps.given("an execution of the orders job is running")
def s4_running(world):
    # The first task holds the execution open, so a new version can land mid-run.
    exec_id = _run_workflow(world, TEAM_A, ORDERS_WORKFLOW, key=world.keys[TEAM_A],
                            context={"hold_seconds": 45})
    world.scratch["running_exec"] = exec_id
    world.scratch["running_since"] = time.time()
    world.note(f"orders execution {exec_id[:8]} started on 0.1.0, first task holding")


@steps.when("team A publishes version 0.2.0 of the orders job")
def s4_publish_v2(world):
    directory = world.stage_fixture(ORDERS_FIXTURE, version="0.2.0")
    archive = world.pack(directory)
    code, out, err = world.publish_archive(archive, tenant=TEAM_A, key=world.keys[TEAM_A])
    assert code == 0, f"publishing orders 0.2.0 failed: {err or out}"


@steps.when("version 0.2.0 of the orders job comes into service")
def s4_v2_active(world):
    world.wait_build_status(TEAM_A, ORDERS_PKG, {"success"}, key=world.keys[TEAM_A],
                            version="0.2.0", timeout_s=300)
    deadline = time.time() + 60
    while time.time() < deadline:
        active = world.active_version(TEAM_A, ORDERS_PKG, key=world.keys[TEAM_A])
        if active and active.get("version") == "0.2.0":
            _, body = world.api("GET", f"/v1/tenants/{TEAM_A}/executions/{world.scratch['running_exec']}",
                                key=world.keys[TEAM_A], expect=None)
            world.note(f"0.2.0 came into service {int(time.time() - world.scratch['running_since'])}s "
                       f"after the execution started; the execution was then "
                       f"{body.get('status') if isinstance(body, dict) else body}")
            return
        time.sleep(3)
    raise AssertionError("version 0.2.0 never became the active version")


@steps.then("the running execution finishes")
def s4_finishes(world):
    status = _poll_execution_status(world, TEAM_A, world.scratch["running_exec"],
                                    {"Completed", "Failed", "Cancelled"}, key=world.keys[TEAM_A])
    world.note(f"running execution finished in {status}, about "
               f"{int(time.time() - world.scratch['running_since'])}s after it started "
               "(its first task holds for 45s)")
    assert status == "Completed", f"the running execution ended in {status}"


@steps.then("every task of the running execution ran on version 0.1.0")
def s4_pinned(world):
    ctx = world.execution_context(TEAM_A, world.scratch["running_exec"])
    first = ctx.get("first_task_version")
    second = ctx.get("second_task_version")
    world.note(f"running execution task versions: first={first}, second={second} "
               "(read from the execution context in postgres)")
    assert first == "0.1.0", f"first task ran on {first}, expected 0.1.0"
    assert second == "0.1.0", (
        f"second task ran on {second}, expected 0.1.0 — the execution was not pinned to its "
        "starting version")


@steps.then("a new execution of the orders job runs on version 0.2.0")
@steps.given("a new execution of the orders job runs on version 0.2.0")
def s4_new_v2(world):
    exec_id = _run_workflow(world, TEAM_A, ORDERS_WORKFLOW, key=world.keys[TEAM_A],
                            context={"hold_seconds": 0})
    _poll_execution_status(world, TEAM_A, exec_id, {"Completed", "Failed", "Cancelled"},
                           key=world.keys[TEAM_A])
    ctx = world.execution_context(TEAM_A, exec_id)
    world.note(f"new execution {exec_id[:8]} ran on version {ctx.get('second_task_version')}")
    assert ctx.get("second_task_version") == "0.2.0", (
        f"a fresh execution ran on {ctx.get('second_task_version')}, expected the active 0.2.0")
    world.scratch["last_new_exec"] = exec_id


# --- Scenario 5: go back to an earlier version -----------------------------

@steps.given("team A has published versions 0.1.0 and 0.2.0 of its orders job")
def s5_two_versions(world):
    for version in ("0.1.0", "0.2.0"):
        world.ensure_version(TEAM_A, ORDERS_PKG, ORDERS_FIXTURE, version, key=world.keys[TEAM_A])


@steps.when("team A goes back to version 0.1.0 of the orders job with one command")
def s5_rollback(world):
    code, out, err = world.ctl("package", "activate", ORDERS_PKG, "0.1.0",
                               key=world.keys[TEAM_A], tenant=TEAM_A, check=False)
    world.note(f"`package activate {ORDERS_PKG} 0.1.0` returned exit {code}")
    assert code == 0, f"activate failed: {err or out}"


@steps.then("team A's list of versions of the orders job shows 0.1.0 in service")
def s5_v1_active(world):
    active = world.active_version(TEAM_A, ORDERS_PKG, key=world.keys[TEAM_A])
    assert active is not None, "no active version found"
    world.note(f"active version of the orders job: {active.get('version')}")
    assert active.get("version") == "0.1.0", f"expected 0.1.0, got {active.get('version')}"


@steps.then("a new execution of the orders job runs on version 0.1.0")
def s5_new_v1(world):
    exec_id = _run_workflow(world, TEAM_A, ORDERS_WORKFLOW, key=world.keys[TEAM_A],
                            context={"hold_seconds": 0})
    _poll_execution_status(world, TEAM_A, exec_id, {"Completed", "Failed", "Cancelled"},
                           key=world.keys[TEAM_A])
    ctx = world.execution_context(TEAM_A, exec_id)
    world.note(f"new execution ran on version {ctx.get('second_task_version')}")
    assert ctx.get("second_task_version") == "0.1.0", (
        f"a fresh execution ran on {ctx.get('second_task_version')}, expected 0.1.0 after going back")


# --- Scenario 6: a key of one team does nothing in another's tenant --------

@steps.given("team B has published its ledger job")
def s6_ledger(world):
    if world.scratch.get("ledger_published"):
        return
    active = world.ensure_version(TEAM_B, LEDGER_PKG, LEDGER_FIXTURE, "0.1.0",
                                  key=world.keys[TEAM_B])
    # The digest team A will try to fetch in the probe scenario.
    world.scratch["ledger_hash"] = active.get("content_hash")
    world.scratch["ledger_published"] = True


@steps.when("Robin of team A uses team A's key against team B's tenant")
def s6_crosskey(world):
    world.scratch["a_key"] = world.keys[TEAM_A]


@steps.then("Robin cannot publish a package to team B")
def s6_nopublish(world):
    directory = world.stage_fixture(ORDERS_FIXTURE)
    archive = world.pack(directory)
    for role, key in (("write", world.keys[TEAM_A]), ("admin", world.admin_keys[TEAM_A])):
        code, out, err = world.publish_archive(archive, tenant=TEAM_B, key=key)
        world.note(f"team A {role} key publishing into team B -> exit {code}: "
                   f"{(err or out).strip()[:120]}")
        assert code != 0, f"team A's {role} key was allowed to publish into team B's tenant"


@steps.then("Robin cannot read team B's packages, executions or secrets")
def s6_noread(world):
    checks = {
        "packages": f"/v1/tenants/{TEAM_B}/workflows",
        "executions": f"/v1/tenants/{TEAM_B}/executions",
        "secrets": f"/v1/tenants/{TEAM_B}/secrets",
        "secret by name": f"/v1/tenants/{TEAM_B}/secrets/{LEDGER_SECRET}",
        "package source": f"/v1/tenants/{TEAM_B}/workflows/{LEDGER_PKG}/source",
        "keys": f"/v1/tenants/{TEAM_B}/keys",
    }
    allowed = []
    # Team A's admin key too: the secrets routes refuse a write key in any tenant.
    for role, key in (("write", world.keys[TEAM_A]), ("admin", world.admin_keys[TEAM_A])):
        seen = []
        for what, path in checks.items():
            code, _ = world.api("GET", path, key=key, expect=None)
            seen.append(f"{what} {code}")
            if code not in (401, 403, 404):
                allowed.append(f"{role} key: {what}={code}")
        world.note(f"team A {role} key GET in team B: " + ", ".join(seen))
    assert not allowed, f"team A read in team B: {', '.join(allowed)} (expected 401/403/404)"


@steps.then("Robin cannot run team B's ledger job")
def s6_norun(world):
    for role, key in (("write", world.keys[TEAM_A]), ("admin", world.admin_keys[TEAM_A])):
        code, body = world.api("POST", f"/v1/tenants/{TEAM_B}/workflows/{LEDGER_WORKFLOW}/execute",
                               key=key, body={"context": {}}, expect=None)
        world.note(f"team A {role} key executing team B's ledger -> {code}")
        assert code in (401, 403, 404), f"team A's {role} key could run team B's workflow (got {code})"


@steps.then("the same calls succeed with team B's key")
def s6_bkey_ok(world):
    # The controls: without them a 403 could be a broken route, not a wall.
    key = world.admin_keys[TEAM_B]
    seen = []
    for what, path in (("packages", f"/v1/tenants/{TEAM_B}/workflows"),
                       ("executions", f"/v1/tenants/{TEAM_B}/executions"),
                       ("secrets", f"/v1/tenants/{TEAM_B}/secrets"),
                       ("secret by name", f"/v1/tenants/{TEAM_B}/secrets/{LEDGER_SECRET}")):
        code, _ = world.api("GET", path, key=key, expect=None)
        seen.append((what, code))
    world.note("team B admin key GET in team B: " + ", ".join(f"{w} {c}" for w, c in seen))
    exec_id = _run_workflow(world, TEAM_B, LEDGER_WORKFLOW, key=world.keys[TEAM_B])
    status = _poll_execution_status(world, TEAM_B, exec_id, {"Completed", "Failed", "Cancelled"},
                                    key=world.keys[TEAM_B])
    world.note(f"team B write key ran its ledger job -> {status}")
    bad = [f"{w}={c}" for w, c in seen if c != 200]
    assert not bad, f"team B's own key was refused in its own tenant: {', '.join(bad)}"
    assert status == "Completed", f"team B's own run of the ledger job ended in {status}"


# --- Scenario 7: signing (known gap, CLOACINA-T-1025) ----------------------

@steps.given("signing is turned on for the orchestrator")
def s7_signing_on(world):
    # Today the server is deployed with --require-signatures off and no route
    # stores a trusted key. The scenario documents the target; mark the gap.
    raise AssertionError(
        "signing is not deployable yet: the chart runs without --require-signatures and there is "
        "no route to register a key a team trusts (CLOACINA-T-1025). `cloacinactl package pack "
        "--sign` also fails hard (I-0103).")


@steps.when("team A publishes a package with no signature")
def s7_unsigned(world):
    raise AssertionError("not reached: signing is not turned on (CLOACINA-T-1025)")


@steps.then("the publish is refused")
def s7_refused(world):
    raise AssertionError("not reached: signing is not turned on (CLOACINA-T-1025)")


@steps.when("team A registers Robin's signing key as a key it trusts")
def s7_trust(world):
    raise AssertionError("no route stores a trusted signing key per team (CLOACINA-T-1025)")


@steps.when("Robin signs a package and publishes it to team A")
def s7_robin(world):
    raise AssertionError("`package pack --sign` is not implemented (I-0103 / CLOACINA-T-1025)")


@steps.then("the publish is accepted")
def s7_accepted(world):
    raise AssertionError("not reached: signing is not turned on (CLOACINA-T-1025)")


@steps.when("Sam, whose key team A does not trust, signs a package and publishes it to team A")
def s7_sam(world):
    raise AssertionError("no per-team trust set to reject an untrusted signer against (CLOACINA-T-1025)")


# --- Scenario 8: a task cannot read another team's packages or secrets -----

@steps.given("team B keeps a secret in its tenant")
def s8_secret(world):
    world.api("POST", f"/v1/tenants/{TEAM_B}/secrets", key=world.bootstrap_key,
              body={"name": LEDGER_SECRET, "fields": {"token": "team-b-only"}}, expect=None)
    # Confirm it is there (metadata only).
    code, _ = world.api("GET", f"/v1/tenants/{TEAM_B}/secrets/{LEDGER_SECRET}",
                        key=world.admin_keys[TEAM_B], expect=None)
    assert code == 200, f"team B's secret is not present (got {code})"


@steps.when("team A publishes a job whose task tries to read what belongs to team B")
def s8_publish_probe(world):
    own = world.ensure_version(TEAM_A, PROBE_FIXTURE, PROBE_FIXTURE, "0.1.0",
                               key=world.keys[TEAM_A])
    world.scratch["probe_own_hash"] = own.get("content_hash") or ""


@steps.when("that job runs")
def s8_run_probe(world):
    # The server's bundled postgres is the orchestrator's database.
    context = {
        "other_tenant": TEAM_B,
        "other_digest": world.scratch.get("ledger_hash", ""),
        "other_secret": LEDGER_SECRET,
        "own_digest": world.scratch.get("probe_own_hash", ""),
        "own_secret": TEAM_A_SECRET,
        # The probe declares its own secret; a declared secret is a required
        # input, bound here by reference (no value travels in the context).
        TEAM_A_SECRET: {"$secret": TEAM_A_SECRET},
        "own_tenant": TEAM_A,
        # The agent runs in its tenant's namespace: name the service in full.
        "database_host": f"{world.release}-postgresql.{world.ns}.svc.cluster.local",
        "database_port": 5432,
        "metrics_url": f"http://{world.release}-cloacina-server.{world.ns}.svc.cluster.local:8080/metrics",
        "other_namespace": world.tenant_namespace(TEAM_B),
        "internet_host": "1.1.1.1",
        "internet_port": 443,
        "needles": [TEAM_B, "team-b-only", LEDGER_SECRET],
    }
    exec_id = _run_workflow(world, TEAM_A, PROBE_WORKFLOW, key=world.keys[TEAM_A], context=context)
    _poll_execution_status(world, TEAM_A, exec_id, {"Completed", "Failed", "Cancelled"},
                           key=world.keys[TEAM_A])
    report, _ = _probe_report_from_log(world, TEAM_A)
    world.scratch["probe_report"] = report
    world.note(f"probe control (own artifact fetch): {report.get('fetch_own_artifact')}")
    world.note(f"probe control (own secret read): {report.get('read_own_secret')}")


@steps.then("the task could not fetch team B's package")
def s8_no_pkg(world):
    report = world.scratch["probe_report"]
    art = report.get("fetch_other_artifact", {})
    src = report.get("fetch_other_source", {})
    lst = report.get("list_other_workflows", {})
    world.note(f"team B artifact fetch={art}, source fetch={src}, workflows list={lst}")
    assert art.get("status") in (403, 404, None), f"the task fetched team B's artifact: {art}"
    assert src.get("status") in (403, 404, None), f"the task fetched team B's source: {src}"
    assert lst.get("status") in (403, 404, None), f"the task listed team B's packages: {lst}"


@steps.then("the task could not read team B's secret")
def s8_no_secret(world):
    report = world.scratch["probe_report"]
    read = report.get("read_other_secret", {})
    lst = report.get("list_other_secrets", {})
    world.note(f"team B secret read={read}, secrets list={lst}")
    assert read.get("outcome") != "read", f"the task read team B's secret: {read}"
    assert lst.get("status") in (403, 404, None), f"the task listed team B's secrets: {lst}"


@steps.then("the task could not use its key in team B's tenant")
def s8_no_exec(world):
    report = world.scratch["probe_report"]
    ex = report.get("list_other_executions", {})
    world.note(f"team B executions list with the probe's key={ex}")
    brief = ", ".join(
        f"{label} {report.get(field, {}).get('status')}"
        for label, field in (("own packages", "list_own_workflows"), ("own secrets", "list_own_secrets"),
                             ("own keys", "list_own_keys"), ("all tenants", "list_all_tenants"),
                             ("all agents", "list_all_agents"), ("compiler status", "compiler_status"),
                             ("team B providers by digest", "fetch_other_providers")))
    world.note(f"what else the task's key opens (the agent's key, present in the task's "
               f"environment: {report.get('agent_key_present')}): {brief}")
    assert ex.get("status") in (403, 404, None), f"the task read team B's executions: {ex}"
    # CLOACINA-T-1055: the agent's key has the role `agent`. It opens the agent
    # routes and nothing else: not its own tenant's packages, secrets or keys,
    # and no platform route. A field has no status when the key is not in the
    # task's environment at all.
    outside_the_agent_set = ("list_own_workflows", "list_own_secrets", "list_own_keys",
                             "list_all_tenants", "list_all_agents", "compiler_status")
    opened = {field: report.get(field, {}).get("status") for field in outside_the_agent_set
              if report.get(field, {}).get("status") not in (403, None)}
    assert not opened, f"the agent's key opened routes outside the agent set: {opened}"


@steps.then("the task found nothing of team B in its environment or on its disk")
def s8_no_trace(world):
    report = world.scratch["probe_report"]
    scan = report.get("scan", {})
    world.note(f"environment/disk scan for team B's names: {scan}")
    world.note(f"names in the task's environment: {', '.join(report.get('environment_names', []))}")
    world.note(f"the task's disk: {report.get('filesystem')}")
    assert not scan.get("environment_name_or_value_hits"), (
        f"team B's names were in the probe's environment: {scan.get('environment_name_or_value_hits')}")
    assert not scan.get("path_hits"), f"team B's names were on the probe's disk: {scan.get('path_hits')}"


@steps.then("the task could not reach the orchestrator's database")
def s8_no_db(world):
    report = world.scratch["probe_report"]
    db = report.get("reach_database", {})
    world.note(f"probe TCP connect to the orchestrator database: {db}")
    net = report.get("network", {})
    kube = net.get("kubernetes", {})
    world.note(f"network from the task: server /metrics with no key -> "
               f"{net.get('server_metrics', {}).get('status')} "
               f"({net.get('server_metrics', {}).get('bytes')} bytes), server /health -> "
               f"{net.get('server_health', {}).get('status')}, internet (1.1.1.1:443) -> "
               f"{net.get('internet', {}).get('outcome')}")
    world.note("Kubernetes API from the task: service account token readable="
               f"{kube.get('token_readable')}, /version -> "
               f"{kube.get('version_no_token', {}).get('status') or kube.get('version_no_token', {}).get('error')}, "
               "secrets of team B's namespace -> "
               f"{kube.get('other_namespace_secrets', {}).get('status') or kube.get('other_namespace_secrets', {}).get('error')}")
    assert db.get("outcome") != "connected", f"the task reached the orchestrator's database: {db}"


@steps.then("team A's code never ran where the orchestrator keeps its own credentials")
def s8_not_in_server(world):
    report = world.scratch["probe_report"]
    run = report.get("run", {})
    world.note(f"the task ran in process={run.get('process')} on {run.get('hostname')}")
    # The package's code runs at import in every process that loads it. Each
    # import wrote one marked line to that pod's log: look in all of them.
    imports = world.marked_lines(PROBE_IMPORT_MARKER)
    assert imports, "no import line of the probe in any pod log: the check itself is broken"
    bad = []
    for where, facts in imports:
        creds = facts.get("orchestrator_credentials_in_environment") or []
        world.note(f"the probe package was imported in the {where} (process {facts.get('process')}); "
                   f"credential names in that environment: {creds or 'none'}; "
                   f"service account token readable: {facts.get('kubernetes_token_readable')}")
        if creds or facts.get("process") in ("cloacina-server", "cloacina-compiler"):
            bad.append(f"{where} ({', '.join(creds) or 'no credential names'})")
    run_creds = run.get("orchestrator_credentials_in_environment") or []
    assert not run_creds, f"the task saw the orchestrator's credentials where it ran: {run_creds}"
    assert not bad, ("team A's package code ran at import inside: " + "; ".join(bad))
