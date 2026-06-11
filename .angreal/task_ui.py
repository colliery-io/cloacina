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

"""Cloacina web UI dev tasks (CLOACI-I-0117).

`angreal ui up` is the one-command "bring it up": postgres + a CORS-enabled
cloacina-server + the TypeScript SDK + the UI dev server, wired so you can
open the browser and use the UI against a live server.
"""

import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from utils import docker_up, docker_down

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = Path(angreal.get_root()) / "docker-compose.yaml"

# Local-dev knobs. The CORS origin must match the Vite dev port.
UI_PORT = 5173
SERVER_PORT = 8080
SERVER_URL = f"http://127.0.0.1:{SERVER_PORT}"
UI_URL = f"http://localhost:{UI_PORT}"
DEV_BOOTSTRAP_KEY = "clk_dev_ui_bootstrap_key_0001"
DB_URL = "postgres://cloacina:cloacina@localhost:5432/cloacina"

ui = angreal.command_group(name="ui", about="commands for the Cloacina web UI")


def _run(cmd, cwd=PROJECT_ROOT):
    subprocess.run(cmd, check=True, cwd=str(cwd))


def _wait_postgres(timeout_s=30):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        r = subprocess.run(
            ["docker", "compose", "-f", str(COMPOSE_FILE), "exec", "-T",
             "postgres", "pg_isready", "-U", "cloacina"],
            capture_output=True,
        )
        if r.returncode == 0:
            return
        time.sleep(1)
    raise RuntimeError("Postgres did not become ready")


def _reset_database():
    """Drop + recreate the `cloacina` database for a clean slate. (The server
    currently ignores the dbname in --database-url — CLOACI-T-0649 — so a
    fresh start has to happen here.)"""
    subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE_FILE), "exec", "-T", "postgres",
         "psql", "-U", "cloacina", "-d", "postgres",
         "-c", "DROP DATABASE IF EXISTS cloacina WITH (FORCE);",
         "-c", "CREATE DATABASE cloacina OWNER cloacina;"],
        check=True, capture_output=True,
    )


def _wait_health(proc, timeout_s=60):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"server exited {proc.returncode} before /health")
        try:
            with urllib.request.urlopen(f"{SERVER_URL}/health", timeout=1.0):
                return
        except Exception:
            time.sleep(0.5)
    raise RuntimeError("server never became healthy")


@ui()
@angreal.command(
    name="up",
    about="bring up the full local UI stack (postgres + server + SDK + UI dev server)",
    long_about=(
        "Starts Postgres, builds + runs a CORS-enabled cloacina-server on "
        f"{SERVER_URL}, builds the @cloacina/client SDK, installs the UI, and "
        f"runs the Vite dev server on {UI_URL}. Ctrl-C stops the UI and the "
        "server; Postgres is left running for next time (`angreal ui down` to "
        "stop it). Connect in the browser with the server URL, the printed "
        "bootstrap key, and tenant `public`."
    ),
    when_to_use=["local UI development", "manually exercising the UI against a live server"],
    when_not_to_use=["CI", "production", "automated tests (use the e2e/UAT lanes)"],
)
@angreal.argument(
    name="no_build", long="no-build", help="skip cargo/npm builds (fast restart)",
    required=False, takes_value=False, is_flag=True,
)
@angreal.argument(
    name="keep_db", long="keep-db", help="don't drop/recreate the database",
    required=False, takes_value=False, is_flag=True,
)
def up(no_build: bool = False, keep_db: bool = False):
    server_bin = PROJECT_ROOT / "target" / "debug" / "cloacina-server"

    print("=== Cloacina UI dev stack ===")
    print("[1/5] Postgres…")
    docker_up()
    _wait_postgres()
    if not keep_db:
        print("      resetting database (fresh slate)")
        _reset_database()

    if not no_build:
        print("[2/5] Building cloacina-server…")
        _run(["cargo", "build", "-p", "cloacina-server"])
        print("[3/5] Building @cloacina/client SDK…")
        _run(["npm", "install"], cwd=PROJECT_ROOT / "clients" / "typescript")
        _run(["npm", "run", "build"], cwd=PROJECT_ROOT / "clients" / "typescript")
        print("[4/5] Installing UI deps…")
        _run(["npm", "install"], cwd=PROJECT_ROOT / "ui")
    else:
        print("[2-4/5] --no-build: skipping cargo/npm setup")

    if not server_bin.exists():
        print(f"error: {server_bin} not found — run without --no-build first", file=sys.stderr)
        return 1

    home = tempfile.mkdtemp(prefix="cloacina-ui-dev-")
    print(f"[5/5] Starting server ({SERVER_URL}) + UI ({UI_URL})…")
    server = subprocess.Popen(
        [str(server_bin),
         "--bind", f"127.0.0.1:{SERVER_PORT}",
         "--database-url", DB_URL,
         "--bootstrap-key", DEV_BOOTSTRAP_KEY,
         "--cors-allowed-origins", UI_URL,
         "--home", home],
        cwd=str(PROJECT_ROOT),
    )
    try:
        _wait_health(server)
        print("\n" + "=" * 60)
        print("  UI:      " + UI_URL)
        print("  Server:  " + SERVER_URL)
        print("  Connect with →  server: " + SERVER_URL)
        print("                  api key: " + DEV_BOOTSTRAP_KEY)
        print("                  tenant:  public")
        print("  Ctrl-C to stop (Postgres stays up; `angreal ui down` to stop it).")
        print("=" * 60 + "\n")
        # Vite in the foreground — blocks until the user Ctrl-C's.
        subprocess.run(["npm", "run", "dev"], cwd=str(PROJECT_ROOT / "ui"))
    except KeyboardInterrupt:
        pass
    finally:
        print("\nStopping server…")
        server.terminate()
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
    return 0


@ui()
@angreal.command(
    name="down",
    about="stop the backing services started by `ui up`",
    when_to_use=["done with local UI development", "freeing the Postgres container"],
    when_not_to_use=["mid-session — `ui up` leaves Postgres up intentionally for fast restarts"],
)
def down():
    docker_down()
    return 0
