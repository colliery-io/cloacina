"""End-to-end Sync resume test on a live server and agent (COLLIERY-I-0632 /
CLOACINA-T-0980).

Boots Postgres, `cloacina-server` with every task routed to the fleet, and a
real `cloacina-agent` with the Postgres connector staged under its provider
search path. Creates a sync definition and fires it, then kills things in the
window a crash must not lose a record in:

  A. the agent dies after the destination accepted the segment and before the
     task returned (the write task is held by an injected fault); a new agent
     takes the reclaimed task, writes the segment again, and the cursor
     advances once. Upsert keeps the destination equal to the source.
  B. the server dies with the agent's result in hand and nothing committed
     (an injected fault exits before the completion writes); the restarted
     server's next fire waits for the outstanding execution, whose stale claim
     is swept and re-dispatched, then the pass ends with the cursor committed
     once and the destination equal to the source.

Both binaries are built with `--features fault-injection`; a shipped binary has
no such code. Requires Docker and the wasm32-wasip2 target. Run with:
`angreal test e2e sync`.
"""
import json
import os
import signal
import subprocess
import tempfile
import time
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_final_success, print_section_header
from .compiler import (
    REPO_ROOT,
    _assert_ports_free,
    _cloacinactl,
    _kill,
    _poll_run_workflow,
    _psql,
    _start_postgres,
    _wait_http,
)

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)
e2e = angreal.command_group(name="e2e", about="end-to-end tests against a live server")

DB_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"
CONNECTOR_PKG = "cloacina-connector-postgres"


def _build_binaries():
    print("Building cloacina-server + cloacina-agent (fault-injection) + cloacinactl (debug)...")
    subprocess.run(
        [
            "cargo", "build", "-p", "cloacina-server", "-p", "cloacina-agent", "-p", "cloacinactl",
            "--features", "cloacina-server/fault-injection,cloacina-agent/fault-injection",
        ],
        cwd=REPO_ROOT,
        check=True,
    )


def _stage_connector(providers: Path) -> None:
    """Build the Postgres connector guest and stage it as a provider package the
    agent finds under CLOACINA_PROVIDER_PATH."""
    crate = REPO_ROOT / "connectors" / "postgres"
    subprocess.run(
        ["cargo", "build", "--lib", "--target", "wasm32-wasip2", "--release"],
        cwd=crate,
        check=True,
        env={k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"},
    )
    wasm = crate / "target" / "wasm32-wasip2" / "release" / "cloacina_connector_postgres.wasm"
    pkg = providers / CONNECTOR_PKG
    pkg.mkdir(parents=True, exist_ok=True)
    component = "cloacina_connector_postgres.wasm"
    (pkg / component).write_bytes(wasm.read_bytes())
    (pkg / "package.toml").write_text(
        f'[package]\nname = "{CONNECTOR_PKG}"\nversion = "0.1.0"\ninterface = "connector"\n'
        'interface_version = 1\nruntime = "wasm"\n\n[metadata]\ncategory = "constructor"\n\n'
        f'[wasm]\ncomponent = "{component}"\n'
    )

    def member(name: str, kind: str, role: str) -> dict:
        return {
            "name": name,
            "version": "0.1.0",
            "primitive_kind": kind,
            "interface": "connector",
            "interface_version": 1,
            "params": [],
            "config_fields": [{"name": "json", "ty": "String"}],
            "dependencies": [],
            "connector": {
                "roles": [role],
                "supported_sync_modes": ["FullRefresh", "Incremental", "Cdc"],
                "config_schema": '{"type":"object"}',
            },
        }

    (pkg / "provider.json").write_text(
        json.dumps(
            {
                "name": CONNECTOR_PKG,
                "version": "0.1.0",
                "component": component,
                "runtime": "wasm",
                "constructors": [
                    member("source", "source", "Source"),
                    member("destination", "destination", "Destination"),
                ],
            },
            indent=2,
        )
    )


def _sync_params(sync: str, src_table: str, dst_table: str, fault: str) -> dict:
    url = f"{DB_URL}?sslmode=disable"
    stream = {
        "stream": src_table,
        "sync_mode": "Incremental",
        "cursor_field": "n",
        "primary_key": ["n"],
        "write_mode": {"Upsert": {"business_keys": ["n"]}},
        "mapping": {"ops": []},
    }
    grants = json.dumps({"tcp": ["*:15432"]})
    return {
        "sync": sync,
        "source_provider": CONNECTOR_PKG,
        "source_member": "source",
        "source_config": json.dumps({"url": url, "table": src_table}),
        "source_grants": grants,
        "destination_provider": CONNECTOR_PKG,
        "destination_member": "destination",
        "destination_config": json.dumps({"url": url, "table": dst_table}),
        "destination_grants": grants,
        "stream": json.dumps(stream),
        "fault": fault,
    }


def _start_server(home: Path, bind: str, bootstrap_key: str, log_name: str, extra_env: dict | None = None):
    log = open(home / log_name, "a")  # noqa: SIM115 - stays open for the subprocess
    env = os.environ.copy()
    env["CLOACINA_DEFAULT_EXECUTOR"] = "fleet"
    env.update(extra_env or {})
    proc = subprocess.Popen(
        [
            "target/debug/cloacina-server",
            "--home", str(home),
            "--database-url", DB_URL,
            "--bind", bind,
            "--bootstrap-key", bootstrap_key,
            "--agent-heartbeat-interval-s", "2",
            "--agent-liveness-misses", "2",
            "--verbose",
        ],
        cwd=REPO_ROOT,
        stdout=log,
        stderr=subprocess.STDOUT,
        env=env,
    )
    _wait_http(f"http://{bind}/health", "server", proc=proc)
    return proc


def _start_agent(home: Path, server_url: str, bootstrap_key: str, providers: Path, log_name: str):
    log = open(home / log_name, "a")  # noqa: SIM115 - stays open for the subprocess
    env = os.environ.copy()
    env["CLOACINA_PROVIDER_PATH"] = str(providers)
    proc = subprocess.Popen(
        [
            "target/debug/cloacina-agent",
            "--server", server_url,
            "--api-key", bootstrap_key,
            "--max-concurrency", "2",
        ],
        cwd=REPO_ROOT,
        stdout=log,
        stderr=subprocess.STDOUT,
        env=env,
    )
    time.sleep(3.0)
    if proc.poll() is not None:
        raise AssertionError(f"agent exited early: code={proc.returncode}; see {home / log_name}")
    return proc


def _fire(home: Path, sync: str, params: dict) -> str:
    """Fire the sync by hand: the manual-run route goes through the instance
    fire hook, which runs a pass and returns its first execution."""
    return _poll_run_workflow(home, "sync", timeout_s=60.0, context=params)


def _wait_cursor(sync: str, src_table: str, expect_cursor: str, timeout_s: float) -> None:
    deadline = time.time() + timeout_s
    last = None
    while time.time() < deadline:
        last = _psql(
            f"SELECT cursor || '|' || exhausted::text FROM sync_stream_state "
            f"WHERE sync_name = '{sync}' AND stream = '{src_table}'"
        )
        if last == f"{expect_cursor}|t":
            return
        time.sleep(2.0)
    raise AssertionError(f"sync {sync!r} did not reach cursor {expect_cursor} exhausted; last row: {last!r}")


def _seed(src_table: str, dst_table: str, rows: int) -> None:
    _psql(f"DROP TABLE IF EXISTS {src_table}; DROP TABLE IF EXISTS {dst_table}")
    _psql(f"CREATE TABLE {src_table} (n BIGINT PRIMARY KEY, name TEXT)")
    values = ", ".join(f"({i}, 'row {i}')" for i in range(1, rows + 1))
    _psql(f"INSERT INTO {src_table} (n, name) VALUES {values}")


def _dump(label: str, path: Path):
    try:
        print(f"--- {label} (tail) ---\n{path.read_text()[-6000:]}\n--- end {label} ---")
    except Exception as exc:  # noqa: BLE001
        print(f"(failed to read {path}: {exc})")


@test()
@e2e()
@angreal.command(
    name="sync",
    about="end-to-end Sync resume test: kill the agent mid-write, kill the server before the commit (CLOACINA-T-0980)",
    when_to_use=["proving a sync survives an agent or server crash on a live fleet"],
    when_not_to_use=["unit testing", "running without docker"],
)
def sync():
    print_section_header("Sync e2e: resume after an agent kill and a server kill")
    _assert_ports_free(18084)
    _build_binaries()
    _start_postgres()

    bootstrap_key = "test-bootstrap-sync-e2e"
    bind = "127.0.0.1:18084"
    server_url = f"http://{bind}"
    home = Path(tempfile.mkdtemp(prefix="sync-e2e-"))
    providers = home / "providers"
    print(f"sync-e2e home: {home}")
    _stage_connector(providers)
    print("  ok: Postgres connector staged")

    server = None
    agent = None
    try:
        server = _start_server(home, bind, bootstrap_key, "server.log")
        _cloacinactl(home, "config", "profile", "set", "local", server_url, "--api-key", bootstrap_key, "--default")
        agent = _start_agent(home, server_url, bootstrap_key, providers, "agent.log")
        print("  ok: server (fleet) and agent up")

        # The sync definition is an instance of the Sync workflow; the route
        # accepts the params the workflow declares.
        params_a = _sync_params("e2e-agent-kill", "e2e_src_a", "e2e_dst_a", "slow_write_ms:20000")
        params_path = home / "params-a.json"
        params_path.write_text(json.dumps(params_a))
        _cloacinactl(home, "instance", "create", "sync", "e2e-agent-kill", "--params", str(params_path))
        print("  ok: sync definition created as an instance of `sync`")

        # ---- A. kill the agent after the destination wrote, before the task returned ----
        _seed("e2e_src_a", "e2e_dst_a", 5)
        _fire(home, "e2e-agent-kill", params_a)
        time.sleep(8.0)  # read and map are quick; the write is now held by the fault
        written = _psql("SELECT count(*) FROM e2e_dst_a")
        assert written == "5", f"the destination should hold the segment before the agent dies, got {written!r}"
        agent.send_signal(signal.SIGKILL)
        agent.wait(timeout=10)
        print("  agent killed mid-write; starting a new agent")
        agent = _start_agent(home, server_url, bootstrap_key, providers, "agent2.log")
        _wait_cursor("e2e-agent-kill", "e2e_src_a", "5", timeout_s=180.0)
        assert _psql("SELECT count(*) FROM e2e_dst_a") == "5", "upsert keeps the destination equal to the source"
        print("  ok: A. the reclaimed task wrote the segment again; cursor 5, five rows")

        # ---- B. kill the server with the result in hand, before the commit ----
        _kill(server)
        server = _start_server(
            home, bind, bootstrap_key, "server-b.log",
            extra_env={"CLOACINA_FAULT_EXIT_BEFORE_COMPLETION_WRITES": "1"},
        )
        # The agent reconnects to the restarted server on its own; give it a moment.
        time.sleep(5.0)
        _seed("e2e_src_b", "e2e_dst_b", 5)
        params_b = _sync_params("e2e-server-kill", "e2e_src_b", "e2e_dst_b", "")
        _fire(home, "e2e-server-kill", params_b)
        deadline = time.time() + 120.0
        while server.poll() is None and time.time() < deadline:
            time.sleep(1.0)
        assert server.poll() == 86, f"the server should have exited through the injected fault, got {server.poll()!r}"
        print("  server exited before the completion writes; restarting without the fault")
        server = _start_server(home, bind, bootstrap_key, "server-b2.log")
        time.sleep(5.0)
        assert _psql("SELECT count(*) FROM e2e_dst_b") == "5", "the segment was written before the server died"
        assert _psql("SELECT count(*) FROM sync_stream_state WHERE sync_name = 'e2e-server-kill'") == "0", "nothing committed yet"
        # The next fire waits for the outstanding execution: its stale claim is
        # swept and re-dispatched, the write runs again, and the commit lands.
        _fire(home, "e2e-server-kill", params_b)
        _wait_cursor("e2e-server-kill", "e2e_src_b", "5", timeout_s=300.0)
        assert _psql("SELECT count(*) FROM e2e_dst_b") == "5", "upsert absorbed the second write"
        print("  ok: B. the outstanding execution finished after the restart; cursor 5, five rows")

        print_final_success("Sync e2e passed: a sync survives an agent kill mid-write and a server kill before the commit")
    except Exception:
        for name in ("server.log", "server-b.log", "server-b2.log", "agent.log", "agent2.log"):
            if (home / name).exists():
                _dump(name, home / name)
        raise
    finally:
        _kill(agent)
        _kill(server)
