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

import os
import shutil
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
COMPILER_PORT = 9000
SERVER_URL = f"http://127.0.0.1:{SERVER_PORT}"
UI_URL = f"http://localhost:{UI_PORT}"
DEV_BOOTSTRAP_KEY = "clk_dev_ui_bootstrap_key_0001"
DB_URL = "postgres://cloacina:cloacina@localhost:5432/cloacina"

# Seed/demo harness (T-0660). The demo fixtures compile to these packages.
FIXTURES_DIR = PROJECT_ROOT / "examples" / "fixtures"
FIXTURES_DIST = FIXTURES_DIR / "dist"
DEMO_FIXTURES = [
    "demo-slow-rust",
    "demo-fail-rust",
    # CLOACI-I-0124 WS-8: richer fixtures so the UI shows real structure —
    # a cron trigger (demo-cron-rust) and a computation graph with a reactor +
    # accumulator (demo-kafka-stream-rust). Rust-only here; `_stage_and_pack`
    # stages Cargo.toml/lib.rs (Python CG fixtures need a separate path).
    "demo-cron-rust",
    "demo-kafka-stream-rust",
    # WS-6: a package with a non-cron (custom-poll) trigger, so the Triggers
    # view shows a `trigger`-type schedule alongside the cron one — poll/event
    # triggers were invisible in the demo because every seeded schedule was cron.
    "demo-poll-rust",
]
HARNESS_DIR = PROJECT_ROOT / "ui" / "harness"

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
    compiler_bin = PROJECT_ROOT / "target" / "debug" / "cloacina-compiler"

    print("=== Cloacina UI dev stack ===")
    print("[1/6] Postgres…")
    docker_up()
    _wait_postgres()
    if not keep_db:
        print("      resetting database (fresh slate)")
        _reset_database()

    if not no_build:
        print("[2/6] Building cloacina-server + cloacina-compiler…")
        _run(["cargo", "build", "-p", "cloacina-server", "-p", "cloacina-compiler"])
        print("[3/6] Building @cloacina/client SDK…")
        _run(["npm", "install"], cwd=PROJECT_ROOT / "clients" / "typescript")
        _run(["npm", "run", "build"], cwd=PROJECT_ROOT / "clients" / "typescript")
        print("[4/6] Installing UI deps…")
        _run(["npm", "install"], cwd=PROJECT_ROOT / "ui")
    else:
        print("[2-4/6] --no-build: skipping cargo/npm setup")

    if not server_bin.exists():
        print(f"error: {server_bin} not found — run without --no-build first", file=sys.stderr)
        return 1

    home = tempfile.mkdtemp(prefix="cloacina-ui-dev-")
    print(f"[5/6] Starting server ({SERVER_URL})…")
    server = subprocess.Popen(
        [str(server_bin),
         "--bind", f"127.0.0.1:{SERVER_PORT}",
         "--database-url", DB_URL,
         "--bootstrap-key", DEV_BOOTSTRAP_KEY,
         "--cors-allowed-origins", UI_URL,
         "--home", home],
        cwd=str(PROJECT_ROOT),
    )
    compiler = None
    try:
        _wait_health(server)
        # Start the compiler AFTER the server has run migrations — both run
        # migrations on boot, and racing them on a fresh DB collides. The
        # compiler builds uploaded packages (pending → success) so workflows
        # uploaded through the UI actually become executable. `--cargo-flag
        # build --lib` drops the default `--frozen` (the demo fixtures carry
        # no committed Cargo.lock); CARGO_TARGET_DIR reuses the warm workspace
        # cache so package builds are fast.
        if compiler_bin.exists():
            print(f"[6/6] Starting compiler ({COMPILER_PORT}) + UI ({UI_URL})…")
            compiler = subprocess.Popen(
                [str(compiler_bin),
                 "--bind", f"127.0.0.1:{COMPILER_PORT}",
                 "--database-url", DB_URL,
                 "--poll-interval-ms", "1000",
                 "--cargo-target-dir", str(PROJECT_ROOT / "target"),
                 "--cargo-flag=build", "--cargo-flag=--lib",
                 "--home", home],
                cwd=str(PROJECT_ROOT),
            )
        else:
            print("warning: cloacina-compiler not built — uploaded packages won't build",
                  file=sys.stderr)
        print("\n" + "=" * 60)
        print("  UI:      " + UI_URL)
        print("  Server:  " + SERVER_URL)
        print("  Connect with →  server: " + SERVER_URL)
        print("                  api key: " + DEV_BOOTSTRAP_KEY)
        print("                  tenant:  public")
        print("  Seed demo workload: `angreal ui seed` (or `--loop`) in another shell.")
        print("  Ctrl-C to stop (Postgres stays up; `angreal ui down` to stop it).")
        print("=" * 60 + "\n")
        # Vite in the foreground — blocks until the user Ctrl-C's.
        subprocess.run(["npm", "run", "dev"], cwd=str(PROJECT_ROOT / "ui"))
    except KeyboardInterrupt:
        pass
    finally:
        print("\nStopping server + compiler…")
        for proc in (compiler, server):
            if proc is None:
                continue
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
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


# ---------------------------------------------------------------------------
# Seed / demo harness (CLOACI-T-0660)
# ---------------------------------------------------------------------------


def _stage_and_pack(fixture: str, home: Path) -> Path:
    """Stage a fixture (rewriting `__WORKSPACE__` to this checkout) and pack
    it into examples/fixtures/dist/<fixture>.cloacina via cloacinactl.
    Mirrors the e2e harness's staging (.angreal/test/e2e/compiler.py)."""
    src = FIXTURES_DIR / fixture
    staged = home / f"staged-{fixture}"
    if staged.exists():
        shutil.rmtree(staged)
    (staged / "src").mkdir(parents=True)
    ws = str(PROJECT_ROOT)
    for rel in ("package.toml", "Cargo.toml", "build.rs", "src/lib.rs"):
        text = (src / rel).read_text().replace("__WORKSPACE__", ws)
        (staged / rel).write_text(text)

    FIXTURES_DIST.mkdir(parents=True, exist_ok=True)
    archive = FIXTURES_DIST / f"{fixture}.cloacina"
    cloacinactl = PROJECT_ROOT / "target" / "debug" / "cloacinactl"
    subprocess.run(
        [str(cloacinactl), "--home", str(home),
         "package", "pack", str(staged), "--out", str(archive)],
        check=True, cwd=str(PROJECT_ROOT),
    )
    return archive


@ui()
@angreal.command(
    name="build-fixtures",
    about="compile the demo .cloacina fixtures for the seed/demo harness",
    long_about=(
        "Stages and packs the demo workflow fixtures (demo-slow-rust, "
        "demo-fail-rust) into examples/fixtures/dist/*.cloacina. The seed "
        "harness and the demo compose profile upload these. Run once; "
        "re-run after changing a fixture's source."
    ),
    when_to_use=["before `angreal ui seed`", "before the demo compose profile"],
    when_not_to_use=["fixtures already built and unchanged"],
)
def build_fixtures():
    cloacinactl = PROJECT_ROOT / "target" / "debug" / "cloacinactl"
    if not cloacinactl.exists():
        print("Building cloacinactl…")
        _run(["cargo", "build", "-p", "cloacinactl"])
    home = tempfile.mkdtemp(prefix="cloacina-fixtures-")
    for fx in DEMO_FIXTURES:
        print(f"Packing {fx}…")
        archive = _stage_and_pack(fx, Path(home))
        print(f"  → {archive}")
    print(f"\nFixtures ready in {FIXTURES_DIST}")
    return 0


def _ensure_harness_ready():
    """SDK built + harness deps installed so `node src/main.mjs` resolves
    `@cloacina/client`."""
    sdk = PROJECT_ROOT / "clients" / "typescript"
    if not (sdk / "dist" / "index.js").exists():
        print("Building @cloacina/client SDK…")
        _run(["npm", "install"], cwd=sdk)
        _run(["npm", "run", "build"], cwd=sdk)
    if not (HARNESS_DIR / "node_modules").exists():
        print("Installing harness deps…")
        _run(["npm", "install"], cwd=HARNESS_DIR)


@ui()
@angreal.command(
    name="seed",
    about="drive a running server with demo workload (seed or loop mode)",
    long_about=(
        "Runs the seed/demo harness against a reachable cloacina-server: "
        "ensures the tenant, uploads the demo fixtures, and runs executions. "
        "Default seed mode produces a deterministic completed/failed/in-flight "
        "state for UAT; --loop fires runs continuously for a live demo. "
        "Assumes a server is up (e.g. `angreal ui up` in another terminal) and "
        "that fixtures are built (`angreal ui build-fixtures`)."
    ),
    when_to_use=["seeding a dev server for the UI", "driving a live demo"],
    when_not_to_use=["no server running", "production"],
)
@angreal.argument(
    name="loop", long="loop", help="continuous loop mode instead of one-shot seed",
    required=False, takes_value=False, is_flag=True,
)
@angreal.argument(
    name="server", long="server", help=f"server URL (default {SERVER_URL})",
    required=False, takes_value=True, is_flag=False,
)
@angreal.argument(
    name="key", long="key", help="API key (default the dev bootstrap key)",
    required=False, takes_value=True, is_flag=False,
)
@angreal.argument(
    name="tenant", long="tenant", help="tenant (default public)",
    required=False, takes_value=True, is_flag=False,
)
def seed(loop: bool = False, server: str = None, key: str = None, tenant: str = None):
    if not FIXTURES_DIST.exists() or not any(FIXTURES_DIST.glob("*.cloacina")):
        print("No compiled fixtures — running `build-fixtures` first…")
        build_fixtures()

    _ensure_harness_ready()

    env = os.environ.copy()
    env["HARNESS_SERVER_URL"] = server or SERVER_URL
    env["HARNESS_API_KEY"] = key or DEV_BOOTSTRAP_KEY
    env["HARNESS_TENANT"] = tenant or "public"
    env["HARNESS_PACKAGE_DIR"] = str(FIXTURES_DIST)
    env["HARNESS_MODE"] = "loop" if loop else "seed"

    print(f"Driving {env['HARNESS_SERVER_URL']} in {env['HARNESS_MODE']} mode…")
    proc = subprocess.run(
        ["node", "src/main.mjs"], cwd=str(HARNESS_DIR), env=env,
    )
    return proc.returncode
