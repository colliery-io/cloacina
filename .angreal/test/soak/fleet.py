"""Execution-agent fleet soak (CLOACI-T-0635).

Stands up the full fleet as host subprocesses — Postgres, `cloacina-server`
(routing `**=fleet`), `cloacina-compiler`, and N `cloacina-agent`s — compiles a
workflow, then drives sustained load through the fleet for a fixed duration
while sampling `/metrics`. The point is to surface *slow* problems that
unit/integration tests miss:

- **Roster drift / leaks** — agents falsely evicted, or dead entries lingering.
- **Stuck in-flight work** — executions that never reconcile.
- **Outbox growth** — `delivery_outbox` rows that pile up instead of draining.
- **Lost work** — submitted executions that never complete.

Stability criteria checked at the end:

- every submitted execution Completed (no lost / stuck work),
- `cloacina_fleet_agents_evicted_total` == 0 (no roster drift — agents stayed
  healthy under load),
- `cloacina_fleet_work_reassigned_total` == 0 (no spurious reclaim),
- `cloacina_delivery_outbox_open` drains to 0 (no stuck push queue),
- `cloacina_active_tasks` / `cloacina_active_workflows` drain to 0,
- all agent processes still alive.

Tunable via env (defaults in parens):
  CLOACINA_SOAK_FLEET_DURATION_S        sustained-load seconds (120)
  CLOACINA_SOAK_FLEET_AGENTS            agent count (3)
  CLOACINA_SOAK_FLEET_CONCURRENCY       per-agent max-concurrency (2)
  CLOACINA_SOAK_FLEET_SUBMIT_INTERVAL_S seconds between submits (0.5)
"""

import os
import re
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from ..e2e.compiler import (
    REPO_ROOT,
    _assert_ports_free,
    _cloacinactl,
    _kill,
    _poll_build_status,
    _poll_execution_status,
    _poll_run_workflow,
    _stage_fixture,
    _start_postgres,
    _upload,
    _wait_http,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
soak = angreal.command_group(name="soak", about="sustained-load soak tests")

FIXTURE_SRC = "compiler-happy-rust"
WORKFLOW_NAME = "compiler_happy_workflow"

DURATION_S = int(os.environ.get("CLOACINA_SOAK_FLEET_DURATION_S", "120"))
AGENTS = int(os.environ.get("CLOACINA_SOAK_FLEET_AGENTS", "3"))
CONCURRENCY = int(os.environ.get("CLOACINA_SOAK_FLEET_CONCURRENCY", "2"))
SUBMIT_INTERVAL_S = float(os.environ.get("CLOACINA_SOAK_FLEET_SUBMIT_INTERVAL_S", "0.5"))


def _scrape(url: str) -> str:
    """Fetch the Prometheus text exposition; '' on transient failure."""
    try:
        with urllib.request.urlopen(url, timeout=5) as r:
            return r.read().decode()
    except Exception:
        return ""


def _metric(text: str, name: str, contains: str = None) -> float:
    """Sum every series for `name` (handles unlabeled `name v` and labeled
    `name{...} v`), optionally only lines containing `contains`."""
    total = 0.0
    for line in text.splitlines():
        if line.startswith("#"):
            continue
        if not (line.startswith(name + " ") or line.startswith(name + "{")):
            continue
        if contains is not None and contains not in line:
            continue
        m = re.search(r"([0-9.eE+-]+)\s*$", line)
        if m:
            try:
                total += float(m.group(1))
            except ValueError:
                pass
    return total


def _run_once(home: Path) -> str | None:
    """Submit one workflow run; return the execution id (bare `println!` line)
    or None on a non-zero exit."""
    code, out, _ = _cloacinactl(
        home, "-o", "json", "workflow", "run", WORKFLOW_NAME, check=False
    )
    if code != 0 or not out.strip():
        return None
    tail = out.strip().splitlines()[-1].strip()
    return tail if len(tail) >= 32 else None


@test()
@soak()
@angreal.command(
    name="fleet",
    about="fleet soak — server + N agents under sustained load (roster/outbox/drift)",
    when_to_use=[
        "validating the execution-agent fleet under sustained load",
        "hunting roster leaks, stuck outbox entries, reconciliation drift",
    ],
    when_not_to_use=["unit testing", "quick validation", "environments without Docker"],
)
def fleet():
    """Run the execution-agent fleet soak."""
    print_section_header("Execution-Agent Fleet Soak")
    print(
        f"  config: duration={DURATION_S}s agents={AGENTS} "
        f"concurrency={CONCURRENCY} submit_interval={SUBMIT_INTERVAL_S}s"
    )

    print_section_header("Build server + compiler + cloacinactl + agent")
    for pkg in ("cloacina-server", "cloacina-compiler", "cloacinactl", "cloacina-agent"):
        subprocess.run(["cargo", "build", "-p", pkg], cwd=REPO_ROOT, check=True)

    _start_postgres()
    _assert_ports_free(18088, 19008)

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "soak-fleet-key"
    server_bind = "127.0.0.1:18088"
    compiler_bind = "127.0.0.1:19008"
    server_url = f"http://{server_bind}"
    compiler_url = f"http://{compiler_bind}"
    metrics_url = f"{server_url}/metrics"

    home = Path(tempfile.mkdtemp(prefix="fleet-soak-"))
    print(f"  soak home: {home}")

    server_proc = None
    compiler_proc = None
    agent_procs: list[subprocess.Popen] = []

    def _dump(label, path):
        try:
            print(f"--- {label} (tail) ---\n{open(path).read()[-4000:]}\n--- end ---")
        except Exception as exc:
            print(f"(failed to read {path}: {exc})")

    try:
        # --- server (all tasks routed to the fleet) ------------------------
        print_section_header("Boot server + compiler + agents")
        server_env = os.environ.copy()
        server_env["CLOACINA_FLEET_ROUTES"] = "**=fleet"
        server_proc = subprocess.Popen(
            ["target/debug/cloacina-server", "--home", str(home),
             "--database-url", db_url, "--bind", server_bind,
             "--bootstrap-key", bootstrap_key],
            cwd=REPO_ROOT, stdout=open(home / "server.log", "w"),
            stderr=subprocess.STDOUT, env=server_env,
        )
        _wait_http(f"{server_url}/health", "server", proc=server_proc)

        # --- compiler ------------------------------------------------------
        shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler_proc = subprocess.Popen(
            ["target/debug/cloacina-compiler", "--home", str(home),
             "--database-url", db_url, "--bind", compiler_bind,
             "--poll-interval-ms", "500", "--cargo-target-dir", str(shared_target),
             "--cargo-flag=build", "--cargo-flag=--lib"],
            cwd=REPO_ROOT, stdout=open(home / "compiler.log", "w"),
            stderr=subprocess.STDOUT,
        )
        _wait_http(f"{compiler_url}/health", "compiler", timeout_s=60.0, proc=compiler_proc)

        _cloacinactl(home, "config", "profile", "set", "local", server_url,
                     "--api-key", bootstrap_key, "--default")
        _cloacinactl(home, "config", "set", "compiler.local_addr", compiler_bind)

        # --- compile the workflow ------------------------------------------
        print("  compiling fixture (cold build can take minutes; cached <30s)")
        pkg_id = _upload(home, _stage_fixture(home, FIXTURE_SRC))
        body = _poll_build_status(home, pkg_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        print("  ok: workflow compiled")

        # --- start N agents ------------------------------------------------
        for i in range(AGENTS):
            p = subprocess.Popen(
                ["target/debug/cloacina-agent", "--server", server_url,
                 "--api-key", bootstrap_key, "--max-concurrency", str(CONCURRENCY)],
                cwd=REPO_ROOT, stdout=open(home / f"agent-{i}.log", "w"),
                stderr=subprocess.STDOUT,
            )
            agent_procs.append(p)
        time.sleep(4.0)
        for i, p in enumerate(agent_procs):
            if p.poll() is not None:
                _dump(f"agent-{i}.log", home / f"agent-{i}.log")
                raise AssertionError(f"agent {i} exited early: code={p.returncode}")
        print(f"  ok: {AGENTS} agents registered")

        # --- warm-up: prove the fleet path works before loading it ---------
        warm = _poll_run_workflow(home, WORKFLOW_NAME, timeout_s=120.0)
        if _poll_execution_status(home, warm, {"Completed", "Failed", "Cancelled"},
                                  timeout_s=60.0) != "Completed":
            _dump("server.log", home / "server.log")
            raise AssertionError("warm-up execution did not Complete")
        if "agent reported result" not in open(home / "server.log").read():
            raise AssertionError("warm-up did not run on the fleet (no agent report)")
        print("  ok: warm-up executed on the fleet")

        # --- sustained load ------------------------------------------------
        print_section_header(f"Sustained load for {DURATION_S}s")
        completed_before = _metric(_scrape(metrics_url), "cloacina_workflows_total",
                                   'status="completed"')
        submitted: list[str] = []
        failed_submits = 0
        max_outbox = 0.0
        max_active = 0.0
        deadline = time.time() + DURATION_S
        next_sample = time.time()
        while time.time() < deadline:
            eid = _run_once(home)
            if eid:
                submitted.append(eid)
            else:
                failed_submits += 1
            if time.time() >= next_sample:
                text = _scrape(metrics_url)
                outbox = _metric(text, "cloacina_delivery_outbox_open")
                active = _metric(text, "cloacina_active_tasks")
                evicted = _metric(text, "cloacina_fleet_agents_evicted_total")
                reassigned = _metric(text, "cloacina_fleet_work_reassigned_total")
                max_outbox = max(max_outbox, outbox)
                max_active = max(max_active, active)
                print(f"  [{int(deadline - time.time())}s left] submitted={len(submitted)} "
                      f"outbox={outbox:.0f} active_tasks={active:.0f} "
                      f"evicted={evicted:.0f} reassigned={reassigned:.0f}")
                next_sample = time.time() + 10
            time.sleep(SUBMIT_INTERVAL_S)
        print(f"  submitted {len(submitted)} executions ({failed_submits} submit failures)")

        # --- drain: wait for in-flight work to settle ----------------------
        print_section_header("Drain")
        drain_deadline = time.time() + 120
        while time.time() < drain_deadline:
            text = _scrape(metrics_url)
            active_t = _metric(text, "cloacina_active_tasks")
            active_w = _metric(text, "cloacina_active_workflows")
            outbox = _metric(text, "cloacina_delivery_outbox_open")
            if active_t == 0 and active_w == 0 and outbox == 0:
                break
            time.sleep(2)
        text = _scrape(metrics_url)

        # --- stability assertions ------------------------------------------
        print_section_header("Stability checks")
        problems = []
        active_t = _metric(text, "cloacina_active_tasks")
        active_w = _metric(text, "cloacina_active_workflows")
        outbox = _metric(text, "cloacina_delivery_outbox_open")
        evicted = _metric(text, "cloacina_fleet_agents_evicted_total")
        reassigned = _metric(text, "cloacina_fleet_work_reassigned_total")
        completed_after = _metric(text, "cloacina_workflows_total", 'status="completed"')
        failed_after = _metric(text, "cloacina_workflows_total", 'status="failed"')
        completed_delta = completed_after - completed_before

        if active_t != 0 or active_w != 0:
            problems.append(f"stuck in-flight after drain: active_tasks={active_t:.0f} "
                            f"active_workflows={active_w:.0f}")
        if outbox != 0:
            problems.append(f"outbox did not drain: cloacina_delivery_outbox_open={outbox:.0f}")
        if evicted != 0:
            problems.append(f"roster drift: cloacina_fleet_agents_evicted_total={evicted:.0f} "
                            "(agents were declared dead under load)")
        if reassigned != 0:
            problems.append(f"spurious reclaim: cloacina_fleet_work_reassigned_total={reassigned:.0f}")
        if completed_delta < len(submitted):
            problems.append(f"lost work: completed +{completed_delta:.0f} but submitted "
                            f"{len(submitted)} during the soak")
        if failed_after != 0:
            problems.append(f"workflow failures: cloacina_workflows_total{{status=failed}}={failed_after:.0f}")
        dead_agents = [i for i, p in enumerate(agent_procs) if p.poll() is not None]
        if dead_agents:
            problems.append(f"agent process(es) exited during soak: {dead_agents}")

        print(f"  submitted={len(submitted)} completed_delta={completed_delta:.0f} "
              f"max_outbox={max_outbox:.0f} max_active={max_active:.0f} "
              f"evicted={evicted:.0f} reassigned={reassigned:.0f}")

        if problems:
            _dump("server.log", home / "server.log")
            raise AssertionError("fleet soak instability:\n  - " + "\n  - ".join(problems))

        print_final_success(
            f"Fleet soak stable over {DURATION_S}s: {len(submitted)} executions "
            f"completed, outbox flat (max {max_outbox:.0f}), no evictions, no lost work."
        )
    finally:
        for p in agent_procs:
            _kill(p)
        _kill(compiler_proc)
        _kill(server_proc)
        print(f"  (logs preserved in {home})")
