"""End-to-end execution-agent fleet test (CLOACI-I-0114 / T-0634).

The live multi-agent proof for I-0114: boots the FULL pipeline — Postgres,
`cloacina-server` (with every task routed to the "fleet" executor key),
`cloacina-compiler`, and a real `cloacina-agent` — compiles a source workflow,
runs it, and asserts it executes on the agent (not the in-process thread
executor) and reconciles to Completed. This proves the substrate -> agent ->
reconcile loop closes end to end against real Postgres.

Why the compiler is in the loop: workflow packages are uploaded as *source*
(the test fixtures are a few hundred bytes), so the compiler service must build
the cdylib and store it before the reconciler will register the workflow and
the agent can fetch + dlopen the artifact. This mirrors the compiler e2e's
happy path (compiler.py) — we reuse its proven helpers — but boots the server
with `CLOACINA_FLEET_ROUTES=*=fleet` and adds an agent subprocess.

Requires Docker. Run with: `angreal test e2e fleet`.
"""

import os
import subprocess
import tempfile
import time
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header

# Reuse the compiler e2e's proven service/fixture/poll helpers verbatim so the
# fleet test and compiler test can't drift on the build->reconcile contract.
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

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

# Source fixture that the compiler builds; the workflow it defines is named
# `compiler_happy_workflow` (same as the compiler e2e happy path).
FIXTURE_SRC = "compiler-happy-rust"
WORKFLOW_NAME = "compiler_happy_workflow"


def _build_binaries():
    print("Building cloacina-server + cloacina-compiler + cloacinactl + cloacina-agent (debug)...")
    for pkg in ("cloacina-server", "cloacina-compiler", "cloacinactl", "cloacina-agent"):
        subprocess.run(["cargo", "build", "-p", pkg], cwd=REPO_ROOT, check=True)


@test()
@e2e()
@angreal.command(
    name="fleet",
    about="end-to-end execution-agent fleet test (CLOACI-T-0634)",
    when_to_use=[
        "validating the DB-less agent fleet against a live server",
        "confirming substrate -> agent -> reconcile closes end to end",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def fleet():
    print_section_header("cloacina fleet e2e")
    _build_binaries()
    _start_postgres()
    _assert_ports_free(18083, 19003)

    db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"
    bootstrap_key = "test-bootstrap-fleet-e2e"
    server_bind = "127.0.0.1:18083"
    compiler_bind = "127.0.0.1:19003"
    server_url = f"http://{server_bind}"
    compiler_url = f"http://{compiler_bind}"

    server_proc: subprocess.Popen | None = None
    compiler_proc: subprocess.Popen | None = None
    agent_proc: subprocess.Popen | None = None

    # Persistent home so logs survive past an assertion for post-mortem.
    home = Path(tempfile.mkdtemp(prefix="fleet-e2e-"))
    print(f"fleet-e2e home: {home}")

    def _dump(label: str, path: Path):
        try:
            with open(path, "r") as f:
                print(f"--- {label} (tail) ---\n{f.read()[-6000:]}\n--- end {label} ---")
        except Exception as exc:
            print(f"(failed to read {path}: {exc})")

    try:
        # --- server (all tasks routed to the fleet) -------------------------
        server_log = open(home / "server.log", "w")
        server_env = os.environ.copy()
        # `**` (not `*`) to match across `::` segments — task names are fully
        # qualified, e.g. `public::compiler-happy-rust::compiler_happy_workflow::noop`.
        # A single `*` only matches within one segment (dispatcher/router.rs), so
        # `*=fleet` would silently fall back to the default thread executor.
        server_env["CLOACINA_FLEET_ROUTES"] = "**=fleet"
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
        print("  ok: server up with route '*=fleet'")

        # --- compiler service ----------------------------------------------
        shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler_log = open(home / "compiler.log", "w")
        compiler_proc = subprocess.Popen(
            [
                "target/debug/cloacina-compiler",
                "--home", str(home),
                "--database-url", db_url,
                "--bind", compiler_bind,
                "--poll-interval-ms", "500",
                "--cargo-target-dir", str(shared_target),
                "--cargo-flag=build",
                "--cargo-flag=--lib",
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

        # --- compile the workflow ------------------------------------------
        fixture_dir = _stage_fixture(home, FIXTURE_SRC)
        print("  compiling fixture (first run: ~5-10 min cold build; cached: <30s)")
        pkg_id = _upload(home, fixture_dir)
        body = _poll_build_status(home, pkg_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        print("  ok: workflow compiled (build_status = success)")

        # --- start the agent (after the artifact exists to fetch) ----------
        agent_log = open(home / "agent.log", "w")
        agent_proc = subprocess.Popen(
            [
                "target/debug/cloacina-agent",
                "--server", server_url,
                "--api-key", bootstrap_key,
                "--max-concurrency", "2",
            ],
            cwd=REPO_ROOT,
            stdout=agent_log,
            stderr=subprocess.STDOUT,
        )
        # Let it register + open its delivery WS.
        time.sleep(3.0)
        if agent_proc.poll() is not None:
            _dump("agent.log", home / "agent.log")
            raise AssertionError(f"agent exited early: code={agent_proc.returncode}")
        print("  ok: agent registered")

        # --- run the workflow → must execute on the agent → Completed ------
        execution_id = _poll_run_workflow(home, WORKFLOW_NAME, timeout_s=120.0)
        print(f"  triggered execution: {execution_id}")
        status = _poll_execution_status(
            home, execution_id, {"Completed", "Failed", "Cancelled"}, timeout_s=120.0
        )
        if status != "Completed":
            _dump("server.log", home / "server.log")
            _dump("agent.log", home / "agent.log")
            raise AssertionError(
                f"fleet execution ended in {status!r}, expected Completed. "
                f"A task routed to '*=fleet' must run on the agent and reconcile."
            )
        print(f"  ok: fleet execution {status}")

        # --- prove it went through the agent, not the thread executor ------
        # CRITICAL: "Completed" alone is NOT proof of fleet execution. If the
        # route fails to match, the dispatcher silently falls back to the
        # default thread executor and the task still completes. The canonical
        # proof is the server-side `report_result` INFO log "agent reported
        # result" — it fires ONLY when an agent posts the outcome of a work
        # packet. We also assert the dispatch did NOT go to the default executor.
        server_log.flush()
        agent_log.flush()
        server_text = (home / "server.log").read_text()

        if "agent reported result" not in server_text:
            _dump("server.log", home / "server.log")
            _dump("agent.log", home / "agent.log")
            raise AssertionError(
                "execution Completed but the server never logged 'agent reported "
                "result' — the task ran on the thread executor, NOT the fleet. "
                "Check the routing rule actually matched the task name."
            )
        # Belt-and-suspenders: the noop task must have dispatched to "fleet".
        if 'task_name=public::compiler-happy-rust::compiler_happy_workflow::noop executor="default"' in server_text:
            _dump("server.log", home / "server.log")
            raise AssertionError(
                "the workflow task dispatched to executor=\"default\" — the "
                "'**=fleet' route did not match. Fleet path NOT exercised."
            )
        print("  ok: server log confirms the agent executed + reported the work (fleet path)")

        print_final_success("cloacina fleet e2e")
    except Exception:
        _dump("server.log", home / "server.log")
        _dump("compiler.log", home / "compiler.log")
        _dump("agent.log", home / "agent.log")
        raise
    finally:
        _kill(agent_proc)
        _kill(compiler_proc)
        _kill(server_proc)
