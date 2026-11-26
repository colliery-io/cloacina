"""A live fleet stack for the fleet e2e tests: Postgres, `cloacina-server` with
every task sent to the fleet, `cloacina-compiler`, and agents on demand.

Use it as a context manager. It stops every process on exit and prints the
logs when the body raises.

An agent takes its tenant from its API key, and dispatch only picks an agent of
the task's tenant. So `start_agent` gives the agent a key scoped to a tenant;
an agent on the bootstrap admin key has no tenant and gets no work.
"""

import json
import os
import re
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

from .compiler import (
    REPO_ROOT,
    _assert_ports_free,
    _cloacinactl,
    _kill,
    _poll_build_status,
    _stage_fixture,
    _start_postgres,
    _upload,
    _wait_http,
)

DB_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"
_ANSI = re.compile(r"\x1b\[[0-9;]*m")


def build_binaries():
    print("Building cloacina-server + cloacina-compiler + cloacinactl + cloacina-agent (debug)...")
    for pkg in ("cloacina-server", "cloacina-compiler", "cloacinactl", "cloacina-agent"):
        subprocess.run(["cargo", "build", "-p", pkg], cwd=REPO_ROOT, check=True)


class FleetStack:
    def __init__(self, name: str, server_port: int, compiler_port: int):
        self.name = name
        self.bootstrap_key = f"test-bootstrap-{name}"
        self.server_bind = f"127.0.0.1:{server_port}"
        self.compiler_bind = f"127.0.0.1:{compiler_port}"
        self.server_url = f"http://{self.server_bind}"
        self.compiler_url = f"http://{self.compiler_bind}"
        self.ports = (server_port, compiler_port)
        self.home = Path(tempfile.mkdtemp(prefix=f"{name}-"))
        self._procs: list[subprocess.Popen] = []
        self._logs: list = []
        self._agents = 0

    # -- lifecycle ---------------------------------------------------------

    def __enter__(self):
        build_binaries()
        _start_postgres()
        _assert_ports_free(*self.ports)
        print(f"{self.name} home: {self.home}")

        env = os.environ.copy()
        env["CLOACINA_DEFAULT_EXECUTOR"] = "fleet"
        server = self._spawn(
            "server.log",
            [
                "target/debug/cloacina-server",
                "--home", str(self.home),
                "--database-url", DB_URL,
                "--bind", self.server_bind,
                "--bootstrap-key", self.bootstrap_key,
                "--verbose",
            ],
            env,
        )
        _wait_http(f"{self.server_url}/health", "server", proc=server)
        print("  ok: server up, default executor = fleet")

        shared_target = REPO_ROOT / "target" / "compiler-e2e-cache"
        shared_target.mkdir(parents=True, exist_ok=True)
        compiler = self._spawn(
            "compiler.log",
            [
                "target/debug/cloacina-compiler",
                "--home", str(self.home),
                "--database-url", DB_URL,
                "--bind", self.compiler_bind,
                "--poll-interval-ms", "500",
                "--cargo-target-dir", str(shared_target),
                "--cargo-flags-replace=build",
                "--cargo-flags-replace=--lib",
                "--verbose",
            ],
            None,
        )
        _wait_http(f"{self.compiler_url}/health", "compiler", timeout_s=60.0, proc=compiler)
        print("  ok: compiler up")

        _cloacinactl(
            self.home,
            "config", "profile", "set", "local", self.server_url,
            "--api-key", self.bootstrap_key,
            "--default",
        )
        _cloacinactl(self.home, "config", "set", "compiler.local_addr", self.compiler_bind)
        return self

    def __exit__(self, exc_type, exc, tb):
        if exc_type is not None:
            for name in ("server.log", "compiler.log"):
                self.dump(name)
            for i in range(self._agents):
                self.dump(f"agent-{i + 1}.log")
        for proc in reversed(self._procs):
            _kill(proc)
        return False

    def _spawn(self, log_name: str, argv: list[str], env) -> subprocess.Popen:
        log = open(self.home / log_name, "w")  # noqa: SIM115 - stays open for the subprocess
        self._logs.append(log)
        proc = subprocess.Popen(argv, cwd=REPO_ROOT, stdout=log, stderr=subprocess.STDOUT, env=env)
        self._procs.append(proc)
        return proc

    # -- actions -----------------------------------------------------------

    def compile_fixture(self, fixture: str) -> None:
        fixture_dir = _stage_fixture(self.home, fixture)
        print("  compiling fixture (first run: ~5-10 min cold build; cached: <30s)")
        pkg_id = _upload(self.home, fixture_dir)
        body = _poll_build_status(self.home, pkg_id, {"success"}, timeout_s=900.0)
        assert body.get("build_status") == "success", body
        print("  ok: workflow compiled")

    def mint_tenant_key(self, tenant: str) -> str:
        body = self.api("POST", f"/v1/tenants/{tenant}/keys", {"name": f"{self.name}-agent", "role": "read"})
        return body["key"]

    def start_agent(
        self,
        tenant: str | None = "public",
        max_concurrency: int = 2,
        env: dict | None = None,
        pool: str | None = None,
        cpu: str | None = None,
        memory: str | None = None,
    ):
        """Start one agent with a key scoped to `tenant`. Returns its log path.

        `tenant=None` gives the agent the bootstrap admin key: it registers
        with no tenant, has free capacity, and is eligible for no task.

        `pool` puts the agent in an agent pool (`--pool`). `cpu` and `memory`
        are the totals it reports (`--cpu`, `--memory`, Kubernetes quantities);
        with neither it reports none and is packed by its slots only.
        """
        self._agents += 1
        log_name = f"agent-{self._agents}.log"
        agent_env = os.environ.copy()
        agent_env.update(env or {})
        proc = self._spawn(
            log_name,
            [
                "target/debug/cloacina-agent",
                "--server", self.server_url,
                "--api-key", self.bootstrap_key if tenant is None else self.mint_tenant_key(tenant),
                "--max-concurrency", str(max_concurrency),
                *(["--pool", pool] if pool else []),
                *(["--cpu", cpu] if cpu else []),
                *(["--memory", memory] if memory else []),
            ],
            agent_env,
        )
        time.sleep(3.0)
        if proc.poll() is not None:
            self.dump(log_name)
            raise AssertionError(f"agent exited early: code={proc.returncode}")
        size = f", pool {pool or 'default'}, cpu {cpu}, memory {memory}" if (pool or cpu or memory) else ""
        print(f"  ok: agent registered (tenant {tenant}, capacity {max_concurrency}{size})")
        return self.home / log_name

    def api(self, method: str, path: str, body: dict | None = None) -> dict:
        req = urllib.request.Request(
            f"{self.server_url}{path}",
            data=json.dumps(body).encode() if body is not None else None,
            headers={
                "Authorization": f"Bearer {self.bootstrap_key}",
                "Content-Type": "application/json",
            },
            method=method,
        )
        with urllib.request.urlopen(req, timeout=10) as resp:
            return json.loads(resp.read())

    def tasks(self, execution_id: str, tenant: str = "public") -> list[dict]:
        return self.api("GET", f"/v1/tenants/{tenant}/executions/{execution_id}/tasks")["tasks"]

    # -- logs --------------------------------------------------------------

    def log_text(self, name: str) -> str:
        """The text of a log, with the terminal colour codes removed."""
        for log in self._logs:
            log.flush()
        return _ANSI.sub("", (self.home / name).read_text())

    def dump(self, name: str) -> None:
        try:
            print(f"--- {name} (tail) ---\n{self.log_text(name)[-6000:]}\n--- end {name} ---")
        except Exception as exc:
            print(f"(failed to read {name}: {exc})")
