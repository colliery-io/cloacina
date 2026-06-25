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

"""UI acceptance e2e lane (CLOACI-I-0117 / T-0661).

Orchestrates the full stack and drives the SPA with Playwright:

  postgres (fresh DB) → cloacina-server (CORS) → cloacina-compiler
  → build + serve the SPA → seed the workload (T-0660) → Playwright.

The seed harness writes its execution IDs to a summary file which we forward
to Playwright (E2E_*), so the specs can open the in-flight + failed runs
directly. `--smoke` runs just the @smoke subset (the PR gate); the full suite
is the nightly gate — same fast-PR / full-nightly split as the SDK matrix.
"""

import contextlib
import json
import os
import shutil
import subprocess
import tempfile
import time
import urllib.request
from pathlib import Path

import angreal  # type: ignore

from .._utils import print_section_header, print_final_success
from .cli import _start_postgres

test = angreal.command_group(
    name="test", about="Cloacina test suites (unit, integration, e2e, soak)"
)

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = Path(angreal.get_root()) / "docker-compose.yaml"

SERVER_BIND = "127.0.0.1:18085"
SERVER_URL = f"http://{SERVER_BIND}"
COMPILER_BIND = "127.0.0.1:19001"
PREVIEW_PORT = 4173
PREVIEW_URL = f"http://localhost:{PREVIEW_PORT}"
BOOTSTRAP_KEY = "ui-e2e-bootstrap-key"
DB_URL = "postgres://cloacina:cloacina@localhost:5432/cloacina"
TARGET_DIR = str(PROJECT_ROOT / "target")

FIXTURES_DIR = PROJECT_ROOT / "examples" / "fixtures"
FIXTURES_DIST = FIXTURES_DIR / "dist"
DEMO_FIXTURES = ["demo-slow-rust", "demo-fail-rust"]
UI_DIR = PROJECT_ROOT / "ui"
HARNESS_DIR = UI_DIR / "harness"
SDK_DIR = PROJECT_ROOT / "clients" / "typescript"


def _run(cmd, cwd=PROJECT_ROOT, env=None):
    subprocess.run(cmd, check=True, cwd=str(cwd), env=env)


def _fresh_database():
    # `_start_postgres` waits on `pg_isready`, but Postgres can still bounce
    # during its init-restart window right after that passes — so a single
    # DROP/CREATE can race a not-yet-stable server and fail with a transient
    # connection error (exit 56). Retry a few times rather than killing the
    # whole UI e2e run on a flake; the SQL is idempotent so re-running is safe.
    last = None
    for _ in range(10):
        last = subprocess.run(
            ["docker", "compose", "-f", str(COMPOSE_FILE), "exec", "-T", "postgres",
             "psql", "-U", "cloacina", "-d", "postgres",
             "-c", "DROP DATABASE IF EXISTS cloacina WITH (FORCE);",
             "-c", "CREATE DATABASE cloacina OWNER cloacina;"],
            capture_output=True,
        )
        if last.returncode == 0:
            return
        time.sleep(2)
    raise subprocess.CalledProcessError(
        last.returncode, last.args, output=last.stdout, stderr=last.stderr
    )


def _wait_http(url, timeout_s=60, proc=None):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        if proc is not None and proc.poll() is not None:
            raise RuntimeError(f"process exited {proc.returncode} before {url} came up")
        try:
            with urllib.request.urlopen(url, timeout=1.0):
                return
        except Exception:
            time.sleep(0.5)
    raise RuntimeError(f"{url} never came up")


def _build():
    print("Building cloacina-server + cloacina-compiler + cloacinactl…")
    _run(["cargo", "build", "-p", "cloacina-server", "-p", "cloacina-compiler",
          "-p", "cloacinactl"])


def _pack_fixtures(home: Path):
    """Stage (rewrite __WORKSPACE__ → repo) + pack the demo fixtures."""
    FIXTURES_DIST.mkdir(parents=True, exist_ok=True)
    cloacinactl = PROJECT_ROOT / "target" / "debug" / "cloacinactl"
    for fx in DEMO_FIXTURES:
        src = FIXTURES_DIR / fx
        staged = home / f"staged-{fx}"
        if staged.exists():
            shutil.rmtree(staged)
        (staged / "src").mkdir(parents=True)
        for rel in ("package.toml", "Cargo.toml", "build.rs", "src/lib.rs"):
            text = (src / rel).read_text().replace("__WORKSPACE__", str(PROJECT_ROOT))
            (staged / rel).write_text(text)
        archive = FIXTURES_DIST / f"{fx}.cloacina"
        print(f"  packing {fx}…")
        _run([str(cloacinactl), "--home", str(home), "package", "pack",
              str(staged), "--out", str(archive)])


def _build_ui():
    print("Building @cloacina/client SDK + UI…")
    if not (SDK_DIR / "node_modules").exists():
        _run(["npm", "ci"], cwd=SDK_DIR)
    _run(["npm", "run", "build"], cwd=SDK_DIR)
    if not (UI_DIR / "node_modules").exists():
        _run(["npm", "install"], cwd=UI_DIR)
    _run(["npm", "run", "build"], cwd=UI_DIR)
    if not (HARNESS_DIR / "node_modules").exists():
        _run(["npm", "install"], cwd=HARNESS_DIR)
    print("Installing Playwright chromium…")
    _run(["npx", "playwright", "install", "chromium"], cwd=UI_DIR)


@contextlib.contextmanager
def _process(cmd, log_path, cwd=PROJECT_ROOT, env=None):
    log = open(log_path, "wb")
    proc = subprocess.Popen(cmd, stdout=log, stderr=log, cwd=str(cwd), env=env)
    try:
        yield proc
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
        log.close()


def _seed(home: Path, summary_file: Path, step_seconds: int):
    env = {
        **os.environ,
        "HARNESS_SERVER_URL": SERVER_URL,
        "HARNESS_API_KEY": BOOTSTRAP_KEY,
        "HARNESS_TENANT": "public",
        "HARNESS_PACKAGE_DIR": str(FIXTURES_DIST),
        "HARNESS_MODE": "seed",
        "HARNESS_STEP_SECONDS": str(step_seconds),
        "HARNESS_SUMMARY_FILE": str(summary_file),
    }
    _run(["node", "src/main.mjs"], cwd=HARNESS_DIR, env=env)


def _run_playwright(summary_file: Path, bad_pkg: Path, smoke: bool):
    summary = json.loads(summary_file.read_text())
    env = {
        **os.environ,
        "E2E_BASE_URL": PREVIEW_URL,
        "E2E_SERVER_URL": SERVER_URL,
        "E2E_API_KEY": BOOTSTRAP_KEY,
        "E2E_TENANT": "public",
        "E2E_INFLIGHT_EXECUTION_ID": summary["inflight"]["execution_id"],
        "E2E_FAILED_EXECUTION_ID": summary["failed"]["execution_id"],
        "E2E_VALID_PACKAGE": str(FIXTURES_DIST / "demo-slow-rust.cloacina"),
        "E2E_BAD_PACKAGE": str(bad_pkg),
        "CI": os.environ.get("CI", ""),
    }
    # The @visual suite (CLOACI-T-0771) is a pixel gate with its own committed
    # baselines + environment (the demo stack), run by the `ui-visual` workflow —
    # exclude it here so the functional e2e isn't coupled to screenshot baselines.
    cmd = ["npx", "playwright", "test", "--reporter=list", "--grep-invert", "@visual"]
    if smoke:
        cmd += ["--grep", "@smoke"]
    _run(cmd, cwd=UI_DIR, env=env)


def _ui_e2e(smoke: bool) -> int:
    label = "smoke subset" if smoke else "full suite"
    print_section_header(f"UI acceptance e2e ({label})")
    _build()
    _start_postgres()
    _fresh_database()

    with tempfile.TemporaryDirectory(prefix="cloacina-ui-e2e-") as tmp:
        home = Path(tmp)
        _pack_fixtures(home)
        _build_ui()

        server_cmd = [
            "target/debug/cloacina-server",
            "--home", str(home),
            "--database-url", DB_URL,
            "--bind", SERVER_BIND,
            "--bootstrap-key", BOOTSTRAP_KEY,
            "--cors-allowed-origins", PREVIEW_URL,
        ]
        # Compiler is started AFTER the server has migrated the fresh DB —
        # racing migrations collides. `build --lib` drops the default --frozen
        # (demo fixtures carry no Cargo.lock); the shared target dir keeps
        # package builds fast.
        compiler_cmd = [
            "target/debug/cloacina-compiler",
            "--home", str(home),
            "--database-url", DB_URL,
            "--bind", COMPILER_BIND,
            "--poll-interval-ms", "1000",
            "--cargo-target-dir", TARGET_DIR,
            "--cargo-flag=build", "--cargo-flag=--lib",
        ]
        preview_cmd = ["npx", "vite", "preview", "--port", str(PREVIEW_PORT), "--strictPort"]

        # Seed the demo tenants/keys the auth specs connect with — the acme
        # tenant-admin (clk_demo_acme_key_0002) + public — matching
        # docker-compose.demo.yml. Without this the acme tenant/key never exist,
        # so tenant-admin.spec.ts / local-auth.spec.ts can't sign in and connect
        # never reaches Overview. (CLOACI-T-0787)
        server_env = {
            **os.environ,
            "CLOACINA_DEMO_TENANT_KEYS": (
                "public:clk_demo_public_key_0003:admin,"
                "acme:clk_demo_acme_key_0002:admin"
            ),
        }

        with _process(server_cmd, home / "server.log", env=server_env) as server:
            _wait_http(f"{SERVER_URL}/health", proc=server)
            with _process(compiler_cmd, home / "compiler.log"), \
                 _process(preview_cmd, home / "preview.log", cwd=UI_DIR) as preview:
                _wait_http(PREVIEW_URL, proc=preview)

                summary_file = home / "seed-summary.json"
                # ~40s in-flight window so Playwright reliably opens the slow
                # run while it's still streaming.
                _seed(home, summary_file, step_seconds=8)

                bad_pkg = home / "bad.cloacina"
                bad_pkg.write_text("this is not a valid cloacina package")

                _run_playwright(summary_file, bad_pkg, smoke)

    print_final_success(f"UI acceptance e2e passed ({label})")
    return 0


@test()
@angreal.command(
    name="ui-e2e",
    about="UI acceptance e2e — Playwright over the seeded stack (CLOACI-T-0661)",
    long_about=(
        "Boots postgres (fresh DB) + cloacina-server + cloacina-compiler, "
        "builds + serves the SPA, seeds a deterministic workload (T-0660), and "
        "runs the Playwright acceptance suite against it. Full suite by "
        "default; --smoke runs the @smoke subset for the PR gate."
    ),
    when_to_use=["validating the UI end-to-end", "release validation", "nightly"],
    when_not_to_use=["unit testing", "running without docker/node"],
)
@angreal.argument(
    name="smoke", long="smoke", help="run only the @smoke subset (PR gate)",
    required=False, takes_value=False, is_flag=True,
)
def ui_e2e(smoke: bool = False):
    return _ui_e2e(smoke)
