# Copyright 2026 Cloacina Contributors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""SDK live-server contract matrix (CLOACI-I-0113 / T-0648).

Boots cloacina-server against a FRESH database and runs each SDK's
contract suite against it — the explicit drift gate between utoipa
annotations and actual handler behavior (REQ-007). Also enforces the
static coverage rule (every spec endpoint referenced by every SDK) via
scripts/check_sdk_coverage.py.
"""

import contextlib
import os
import subprocess
import tempfile
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success
from .cli import _build_binaries, _start_postgres, _wait_for_health

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)

BIND = "127.0.0.1:18084"
BASE_URL = f"http://{BIND}"
BOOTSTRAP_KEY = "sdk-contract-bootstrap-key"
PROJECT_ROOT = Path(angreal.get_root()).parent


def _fresh_database():
    """Drop + recreate the `cloacina` database so prior state can't leak
    into contract assertions (the server currently hardcodes the dbname —
    CLOACI-T-0649 — so isolation has to happen at this level)."""
    subprocess.run(
        [
            "docker", "compose", "-f", ".angreal/docker-compose.yaml",
            "exec", "-T", "postgres",
            "psql", "-U", "cloacina", "-d", "postgres",
            "-c", "DROP DATABASE IF EXISTS cloacina WITH (FORCE);",
            "-c", "CREATE DATABASE cloacina OWNER cloacina;",
        ],
        check=True,
        capture_output=True,
    )


@contextlib.contextmanager
def _sdk_server():
    """Build + boot cloacina-server on a fresh DB; yield (base_url, env)."""
    _build_binaries()
    _start_postgres()
    _fresh_database()

    env = {
        **os.environ,
        "CLOACINA_SERVER_URL": BASE_URL,
        "CLOACINA_API_KEY": BOOTSTRAP_KEY,
    }

    with tempfile.TemporaryDirectory() as home:
        stderr_log = open(Path(home) / "server-stderr.log", "wb")
        server = subprocess.Popen(
            [
                "target/debug/cloacina-server",
                "--home", home,
                "--database-url", "postgres://cloacina:cloacina@localhost:5432/cloacina",
                "--bind", BIND,
                "--bootstrap-key", BOOTSTRAP_KEY,
            ],
            stdout=subprocess.DEVNULL,
            stderr=stderr_log,
        )
        try:
            _wait_for_health(BASE_URL, server_proc=server)
            yield env
        finally:
            server.terminate()
            try:
                server.wait(timeout=10)
            except subprocess.TimeoutExpired:
                server.kill()
            stderr_log.close()


def _coverage_check():
    print("\n--- static coverage rule (spec endpoints x SDKs) ---")
    subprocess.run(
        ["python3", "scripts/check_sdk_coverage.py"],
        check=True,
        cwd=str(PROJECT_ROOT),
    )


def _version_check():
    print("\n--- SDK version lockstep ---")
    subprocess.run(
        ["python3", "scripts/check_sdk_versions.py"],
        check=True,
        cwd=str(PROJECT_ROOT),
    )


def _run_rust(env):
    print("\n--- Rust (cloacina-client) contract suite ---")
    subprocess.run(
        ["cargo", "test", "-p", "cloacina-client"],
        check=True,
        env=env,
        cwd=str(PROJECT_ROOT),
    )


def _run_python(env):
    print("\n--- Python (cloacina-client) contract suite ---")
    subprocess.run(
        ["uv", "run", "pytest", "-q"],
        check=True,
        env=env,
        cwd=str(PROJECT_ROOT / "clients" / "python"),
    )


def _run_ts(env):
    print("\n--- TypeScript (@cloacina/client) contract suite ---")
    ts_dir = PROJECT_ROOT / "clients" / "typescript"
    if not (ts_dir / "node_modules").exists():
        subprocess.run(["npm", "ci"], check=True, cwd=str(ts_dir))
    subprocess.run(
        ["npm", "run", "test:contract"],
        check=True,
        env=env,
        cwd=str(ts_dir),
    )


def _run_matrix(runners):
    _version_check()
    _coverage_check()
    with _sdk_server() as env:
        for run in runners:
            run(env)
    print_final_success("SDK contract matrix passed")
    return 0


@test()
@angreal.command(
    name="sdk-contract",
    about="run all three SDK contract suites against a live server (drift gate)",
    when_to_use=[
        "after changing server routes, DTOs, or any SDK",
        "release validation",
        "verifying the OpenAPI annotations match handler behavior",
    ],
    when_not_to_use=["unit testing", "running without docker"],
)
def sdk_contract():
    print_section_header("SDK contract matrix (rust + python + typescript)")
    return _run_matrix([_run_rust, _run_python, _run_ts])


@test()
@angreal.command(
    name="sdk-contract-rust",
    about="run the Rust cloacina-client contract suite against a live server",
    when_to_use=["iterating on the Rust client or server handlers"],
    when_not_to_use=["full release validation — use sdk-contract"],
)
def sdk_contract_rust():
    print_section_header("Rust SDK contract suite")
    return _run_matrix([_run_rust])


@test()
@angreal.command(
    name="sdk-contract-python",
    about="run the Python cloacina-client contract suite against a live server",
    when_to_use=["iterating on the Python SDK or server handlers"],
    when_not_to_use=["full release validation — use sdk-contract"],
)
def sdk_contract_python():
    print_section_header("Python SDK contract suite")
    return _run_matrix([_run_python])


@test()
@angreal.command(
    name="sdk-contract-ts",
    about="run the TypeScript @cloacina/client contract suite against a live server",
    when_to_use=["iterating on the TS SDK or server handlers"],
    when_not_to_use=["full release validation — use sdk-contract"],
)
def sdk_contract_ts():
    print_section_header("TypeScript SDK contract suite")
    return _run_matrix([_run_ts])
