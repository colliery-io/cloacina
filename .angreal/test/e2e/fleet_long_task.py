"""End-to-end fleet test: a task that runs longer than the delivery sweeper's
stuck threshold must run one time.

The agent acks a work packet only after the task ends. The delivery sweeper
resets a pushed-but-unacked outbox row to pending after 60 s (sweep every
30 s), and the relay pushes it again. This test runs one task for 150 s on a
real agent and counts how many times the task body started. The fixture
appends a line to a marker file at each start and each end.

Same stack as `angreal test e2e fleet`: Postgres, server with every task sent
to the fleet, compiler, one agent. Requires Docker. Takes about 4 minutes
after the builds. Run with: `angreal test e2e fleet-long-task`.
"""

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from .compiler import (
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
from .fleet import _build_binaries

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

FIXTURE_SRC = "fleet-long-task-rust"
WORKFLOW_NAME = "fleet_long_task_workflow"
# Longer than the sweeper's 60 s threshold plus one 30 s sweep, shorter than
# the 300 s fleet result timeout.
TASK_SECONDS = 150


def _mint_tenant_key(server_url: str, admin_key: str, tenant: str) -> str:
    """Create a key scoped to `tenant`. The agent takes its tenant from its key,
    and dispatch only picks an agent of the task's tenant: an agent on the
    bootstrap admin key has no tenant and gets no work."""
    req = urllib.request.Request(
        f"{server_url}/v1/tenants/{tenant}/keys",
        data=json.dumps({"name": "fleet-long-task-agent", "role": "read"}).encode(),
        headers={
            "Authorization": f"Bearer {admin_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read())["key"]


def _marker_lines(marker: Path) -> list[str]:
    if not marker.exists():
        return []
    return [line for line in marker.read_text().splitlines() if line.strip()]


@test()
@e2e()
@angreal.command(
    name="fleet-long-task",
    about="fleet e2e: a 150 s task must run one time",
    when_to_use=[
        "checking that a task longer than the delivery stuck threshold is not pushed again",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet_long_task():
    print_section_header("cloacina fleet long-task e2e")
    _build_binaries()
    _start_postgres()
    _assert_ports_free(18084, 19004)

    db_url = "postgres://cloacina:cloacina@localhost:15432/cloacina"
    bootstrap_key = "test-bootstrap-fleet-long-task"
    server_bind = "127.0.0.1:18084"
    compiler_bind = "127.0.0.1:19004"
    server_url = f"http://{server_bind}"
    compiler_url = f"http://{compiler_bind}"

    server_proc: subprocess.Popen | None = None
    compiler_proc: subprocess.Popen | None = None
    agent_proc: subprocess.Popen | None = None

    home = Path(tempfile.mkdtemp(prefix="fleet-long-task-e2e-"))
    marker = home / "long-task-marker.txt"
    print(f"fleet-long-task-e2e home: {home}")

    def _dump(label: str, path: Path):
        try:
            with open(path, "r") as f:
                print(f"--- {label} (tail) ---\n{f.read()[-6000:]}\n--- end {label} ---")
        except Exception as exc:
            print(f"(failed to read {path}: {exc})")

    try:
        server_log = open(home / "server.log", "w")  # noqa: SIM115 - stays open for the subprocess
        server_env = os.environ.copy()
        server_env["CLOACINA_DEFAULT_EXECUTOR"] = "fleet"
        server_proc = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", server_bind,
                "--bootstrap-key", bootstrap_key,
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=server_log,
            stderr=subprocess.STDOUT,
            env=server_env,
        )
        _wait_http(f"{server_url}/health", "server", proc=server_proc)
        print("  ok: server up, default executor = fleet")

        shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler_log = open(home / "compiler.log", "w")  # noqa: SIM115 - stays open for the subprocess
        compiler_proc = subprocess.Popen(
            [
                "target/debug/cloacina-compiler",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", compiler_bind,
                "--poll-interval-ms", "500",
                "--cargo-target-dir", str(shared_target),
                "--cargo-flags-replace=build",
                "--cargo-flags-replace=--lib",
                "--verbose",
            ],
            cwd=REPO_ROOT,
            stdout=compiler_log,
            stderr=subprocess.STDOUT,
        )
        _wait_http(f"{compiler_url}/health", "compiler", timeout_s=60.0, proc=compiler_proc)
        print("  ok: compiler up")

        _cloacinactl(
            home,
            "config", "profile", "set", "local", server_url,
            "--api-key", bootstrap_key,
            "--default",
        )
        _cloacinactl(home, "config", "set", "compiler.local_addr", compiler_bind)

        fixture_dir = _stage_fixture(home, FIXTURE_SRC)
        print("  compiling fixture (first run: ~5-10 min cold build; cached: <30s)")
        pkg_id = _upload(home, fixture_dir)
        body = _poll_build_status(home, pkg_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        print("  ok: workflow compiled")

        # Capacity 2, so a second push of the same packet is accepted and run,
        # not refused as "saturated".
        agent_log = open(home / "agent.log", "w")  # noqa: SIM115 - stays open for the subprocess
        agent_env = os.environ.copy()
        agent_env["FLEET_LONG_TASK_MARKER"] = str(marker)
        agent_env["FLEET_LONG_TASK_SECONDS"] = str(TASK_SECONDS)
        agent_key = _mint_tenant_key(server_url, bootstrap_key, "public")
        agent_proc = subprocess.Popen(
            [
                "target/debug/cloacina-agent",
                "--server", server_url,
                "--api-key", agent_key,
                "--max-concurrency", "2",
            ],
            cwd=REPO_ROOT,
            stdout=agent_log,
            stderr=subprocess.STDOUT,
            env=agent_env,
        )
        time.sleep(3.0)
        if agent_proc.poll() is not None:
            _dump("agent.log", home / "agent.log")
            raise AssertionError(f"agent exited early: code={agent_proc.returncode}")
        print("  ok: agent registered")

        execution_id = _poll_run_workflow(home, WORKFLOW_NAME, timeout_s=120.0)
        started_at = time.time()
        print(f"  triggered execution: {execution_id} (task runs {TASK_SECONDS} s)")
        status = _poll_execution_status(
            home, execution_id, {"Completed", "Failed", "Cancelled"}, timeout_s=900.0
        )
        elapsed = time.time() - started_at
        print(f"  execution {status} after {elapsed:.0f} s")
        # Let a second copy, if one started late, write its start line.
        time.sleep(5.0)

        lines = _marker_lines(marker)
        starts = [line for line in lines if line.startswith("start ")]
        ends = [line for line in lines if line.startswith("end ")]
        server_log.flush()
        agent_log.flush()
        reported = (home / "server.log").read_text().count("agent reported result")
        print("  marker file:")
        for line in lines:
            print(f"    {line}")
        print(
            f"  task starts: {len(starts)}, task ends: {len(ends)}, "
            f"results reported to the server: {reported}"
        )

        if len(starts) == 0:
            _dump("server.log", home / "server.log")
            _dump("agent.log", home / "agent.log")
            raise AssertionError(
                "the task body never wrote a start line: it did not run on the agent"
            )
        if len(starts) != 1:
            _dump("agent.log", home / "agent.log")
            raise AssertionError(
                f"the task body started {len(starts)} times for one execution "
                f"(status {status!r}). A {TASK_SECONDS} s task must run one time."
            )
        if status != "Completed":
            _dump("server.log", home / "server.log")
            _dump("agent.log", home / "agent.log")
            raise AssertionError(f"execution ended in {status!r}, expected Completed")
        if len(ends) != 1:
            raise AssertionError(f"the task body ended {len(ends)} times, expected 1")

        # The agent's heartbeat renews the lease of the packet it works on, so
        # the delivery sweeper must not have reset the row for a re-push.
        server_text = re.sub(r"\x1b\[[0-9;]*m", "", (home / "server.log").read_text())
        resets = re.findall(r"delivery sweep complete reset=([1-9][0-9]*)", server_text)
        if resets:
            raise AssertionError(
                f"the delivery sweeper reset a row {len(resets)} time(s) while its "
                "agent was alive and working on it"
            )
        if "refusing packet" in (home / "agent.log").read_text():
            raise AssertionError("the agent refused a packet; no second push was expected")
        print("  ok: one start, one end, no re-push, no refusal")

        print_final_success("cloacina fleet long-task e2e")
    except Exception:
        _dump("server.log", home / "server.log")
        _dump("compiler.log", home / "compiler.log")
        raise
    finally:
        _kill(agent_proc)
        _kill(compiler_proc)
        _kill(server_proc)
