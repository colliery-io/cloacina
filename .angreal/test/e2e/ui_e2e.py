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
import urllib.error
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
# Tenants the lane creates (the acme auth specs connect into `acme`).
LANE_TENANTS = ("acme",)
DB_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"
TARGET_DIR = str(PROJECT_ROOT / "target")

FIXTURES_DIR = PROJECT_ROOT / "examples" / "fixtures"
# Lane-owned archive dir, wiped on each run: the seed harness uploads every
# `.cloacina` in it, so a shared dir let stale archives from other runs decide
# what the lane tested. (CLOACINA-T-0939)
FIXTURES_DIST = FIXTURES_DIR / "dist" / "ui-e2e"
DEMO_FIXTURES = ["demo-slow-rust", "demo-fail-rust"]
# Pure-Python packages: no cargo build (the compiler skips them, the server
# imports them through its embedded cloaca), so a computation graph costs the
# lane seconds, not a cold CG dylib build. (CLOACINA-T-0939)
PY_FIXTURES = ["demo-py-graph"]
# The graph that PY_FIXTURES registers; the lane waits for it before the specs.
LANE_GRAPH = "demo_py_graph"
UI_DIR = PROJECT_ROOT / "ui"
HARNESS_DIR = UI_DIR / "harness"
SDK_DIR = PROJECT_ROOT / "clients" / "typescript"


def _run(cmd, cwd=PROJECT_ROOT, env=None):
    subprocess.run(cmd, check=True, cwd=str(cwd), env=env)


def _fresh_database():
    # Postgres can bounce during its init-restart window even after
    # `pg_isready` passes, so the DROP/CREATE retries transient failures
    # (exit 56). PR #145 hardened this copy first; CLOACI-T-0806 lifted the
    # pattern into the shared helper every lane now uses.
    from .._utils import psql_retry

    # Tenant roles are cluster-wide: DROP DATABASE leaves them behind, and the
    # next run's `POST /v1/tenants` then fails at `CREATE USER` ("role already
    # exists"), rolling back the tenant schema with it. Drop them with the
    # database so each run starts from nothing. (CLOACINA-T-0937)
    drop_roles = [a for t in LANE_TENANTS for a in ("-c", f"DROP ROLE IF EXISTS {t};")]
    psql_retry(
        [
            "-c", "DROP DATABASE IF EXISTS cloacina WITH (FORCE);",
            *drop_roles,
            "-c", "CREATE DATABASE cloacina OWNER cloacina;",
        ],
        compose_file=str(COMPOSE_FILE),
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
    print("Building cloacina-server (embedded-ui) + cloacina-compiler + cloacinactl…")
    # CLOACI-I-0130: the embedded UI is THE deployment path — the harness runs
    # the SPA from the server binary itself (no vite preview, one origin).
    _run(["cargo", "build", "-p", "cloacina-server", "--features", "embedded-ui"])
    # Separate invocation: the trio + embedded-ui in ONE cargo call trips a
    # feature-unification clash in cloacina-python (both pass individually).
    _run(["cargo", "build", "-p", "cloacina-compiler", "-p", "cloacinactl"])


def _pack_fixtures(home: Path):
    """Stage (rewrite __WORKSPACE__ → repo) + pack the demo fixtures."""
    if FIXTURES_DIST.exists():
        shutil.rmtree(FIXTURES_DIST)
    FIXTURES_DIST.mkdir(parents=True)
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
    for fx in PY_FIXTURES:
        _pack_python(FIXTURES_DIR / fx, FIXTURES_DIST / f"{fx}.cloacina")


def _pack_python(src: Path, archive: Path):
    """Pack a pure-Python package the way docker/pack-demo-fixtures.sh does:
    a bzip2 tar of `<name>-<version>/` holding package.toml + the module tree."""
    import re
    import tarfile

    # [package] name/version are the first two such keys in package.toml; a
    # regex keeps this free of tomllib (Python 3.11+).
    toml = (src / "package.toml").read_text()
    name = re.search(r'^name\s*=\s*"([^"]+)"', toml, re.M).group(1)
    version = re.search(r'^version\s*=\s*"([^"]+)"', toml, re.M).group(1)
    prefix = f"{name}-{version}"
    print(f"  packing {src.name} (python)…")

    def _no_cache(info):
        return None if "__pycache__" in info.name else info

    with tarfile.open(archive, "w:bz2") as tar:
        tar.add(src, arcname=prefix, filter=_no_cache)


def _wait_graph(name: str, timeout_s: int = 180):
    """Block until the server reports the graph loaded. The graph specs read
    /v1/health/graphs once and skip when it is empty, so a slow load would
    turn them into silent skips. Fail the lane instead."""
    deadline = time.time() + timeout_s
    req = urllib.request.Request(
        f"{SERVER_URL}/v1/health/graphs",
        headers={"Authorization": f"Bearer {BOOTSTRAP_KEY}"},
    )
    while time.time() < deadline:
        try:
            with urllib.request.urlopen(req) as resp:
                items = json.loads(resp.read()).get("items", [])
            if any(g.get("name") == name for g in items):
                return
        except (urllib.error.URLError, ValueError):
            pass
        time.sleep(2)
    raise RuntimeError(f"graph '{name}' did not register within {timeout_s}s")


def _build_ui():
    # CLOACI-I-0141 (T-0932): the UI is a Leptos crate — trunk builds it.
    # node remains only for the Playwright tooling (ui/package.json) and the
    # visual harness.
    print("Building the Leptos UI (trunk)…")
    _run(["trunk", "build", "--release"], cwd=UI_DIR)
    # The seed harness links @cloacina/client from clients/typescript, whose
    # dist/ is built, not committed — build it before the harness installs
    # (fresh runners have no dist/; T-0938 release follow-through).
    ts_client = PROJECT_ROOT / "clients" / "typescript"
    if not (ts_client / "dist" / "index.js").exists():
        print("Building @cloacina/client (tsup)…")
        _run(["npm", "ci"], cwd=ts_client)
        _run(["npm", "run", "build"], cwd=ts_client)
    if not (UI_DIR / "node_modules").exists():
        _run(["npm", "install"], cwd=UI_DIR)
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


def _run_playwright(
    summary_file: Path, bad_pkg: Path, smoke: bool, visual: bool, update_baselines: bool
):
    summary = json.loads(summary_file.read_text())
    env = {
        **os.environ,
        "E2E_BASE_URL": SERVER_URL,  # embedded UI: SPA served by the server itself
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
    # baselines — exclude it here so the functional e2e isn't coupled to
    # screenshot baselines. `--visual` (or CLOACINA_E2E_VISUAL=1, CLOACI-I-0130)
    # runs the @visual suite instead of the functional one, on the same seeded
    # embedded-server stack, in the light and the dark theme (CLOACINA-T-0943).
    # `--update-baselines` writes new baseline images instead of asserting.
    if visual or os.environ.get("CLOACINA_E2E_VISUAL") == "1":
        # The detail route the spec shoots: the seeded lane packs demo-slow-rust
        # (the spec's default is the demo stack's demo-py-workflow).
        env["E2E_VISUAL_WORKFLOW"] = "demo-slow-rust"
        env["E2E_VISUAL_GRAPH"] = LANE_GRAPH
        cmd = ["npx", "playwright", "test", "visual.spec.ts", "--reporter=list"]
        if update_baselines:
            cmd.append("--update-snapshots")
    else:
        # @audit = manual UX-walk/screenshot helpers ("not a CI test" per their
        # headers) — they crash-loop on resource-starved runners and gate
        # nothing the tagged parity specs don't already cover.
        cmd = ["npx", "playwright", "test", "--reporter=list", "--grep-invert", "@visual|@audit"]
    if smoke:
        cmd += ["--grep", "@smoke"]
    _run(cmd, cwd=UI_DIR, env=env)


def _create_tenant(name: str):
    # The acme auth specs (tenant-admin/local-auth) connect into the `acme`
    # tenant. CLOACINA_DEMO_TENANT_KEYS seeds the scoped key but NOT the tenant
    # schema — the demo provisions that with a separate `harness-acme` run. The
    # specs only do key/account management (no workflow content), so creating the
    # empty tenant with the bootstrap (god) key is enough for connect to reach
    # Overview. (CLOACI-T-0787)
    req = urllib.request.Request(
        f"{SERVER_URL}/v1/tenants",
        data=json.dumps({"name": name}).encode(),
        headers={
            "Authorization": f"Bearer {BOOTSTRAP_KEY}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req) as resp:
            resp.read()
    except urllib.error.HTTPError as e:
        # The server answers every creation failure with 400, so the status
        # alone does not say "already exists". Accept the error only when the
        # tenant is really there; otherwise fail the lane here, not later as a
        # search_path 500 on every connect into the tenant. (CLOACINA-T-0937)
        detail = e.read().decode(errors="replace")
        if name not in _tenant_names():
            raise RuntimeError(
                f"creating tenant '{name}' failed: HTTP {e.code}: {detail}"
            ) from e


def _tenant_names() -> set:
    req = urllib.request.Request(
        f"{SERVER_URL}/v1/tenants",
        headers={"Authorization": f"Bearer {BOOTSTRAP_KEY}"},
    )
    with urllib.request.urlopen(req) as resp:
        return {t["name"] for t in json.loads(resp.read())["items"]}


def _ui_e2e(smoke: bool, visual: bool = False, update_baselines: bool = False) -> int:
    if visual:
        label = "visual baselines" if update_baselines else "visual suite"
    else:
        label = "smoke subset" if smoke else "full suite"
    print_section_header(f"UI acceptance e2e ({label})")
    # UI first: the embedded-ui cargo build below runs `trunk build` in ui/
    # via build.rs; prebuilding here also installs the Playwright tooling a
    # fresh CI checkout lacks.
    _build_ui()
    _build()
    _start_postgres()
    _fresh_database()

    with tempfile.TemporaryDirectory(prefix="cloacina-ui-e2e-") as tmp:
        home = Path(tmp)
        _pack_fixtures(home)

        server_cmd = [
            "target/debug/cloacina-server",
            "--home", str(home),
            "--database-url", DB_URL,
            "--bind", SERVER_BIND,
            "--bootstrap-key", BOOTSTRAP_KEY,
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
            "--cargo-flags-replace=build", "--cargo-flags-replace=--lib",
        ]

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
            # Secrets are part of the UI surface (wave4-admin.spec.ts drives
            # create/rotate/delete) — the lane's server must have a KEK, same
            # value as the demo stack.
            "CLOACINA_SECRET_KEK": "ZGVtby1rZWstZGVtby1rZWstZGVtby1rZWstMDAwMSE=",
        }

        with _process(server_cmd, home / "server.log", env=server_env) as server:
            _wait_http(f"{SERVER_URL}/health", proc=server)
            # Create the `acme` tenant schema the auth specs connect into (the
            # scoped key is seeded above, but the tenant itself is not).
            for tenant in LANE_TENANTS:
                _create_tenant(tenant)
            with _process(compiler_cmd, home / "compiler.log"):
                _wait_http(SERVER_URL, proc=server)  # SPA at the server origin

                summary_file = home / "seed-summary.json"
                # ~40s in-flight window so Playwright reliably opens the slow
                # run while it's still streaming.
                _seed(home, summary_file, step_seconds=8)
                # The harness uploaded the Python graph during the seed; it is
                # normally loaded by now, so this returns at once.
                _wait_graph(LANE_GRAPH)

                bad_pkg = home / "bad.cloacina"
                bad_pkg.write_text("this is not a valid cloacina package")

                _run_playwright(summary_file, bad_pkg, smoke, visual, update_baselines)

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
        "default; --smoke runs the @smoke subset for the PR gate; --visual "
        "runs the @visual pixel suite in both themes (--update-baselines "
        "refreshes its images)."
    ),
    when_to_use=["validating the UI end-to-end", "release validation", "nightly"],
    when_not_to_use=["unit testing", "running without docker/node"],
)
@angreal.argument(
    name="smoke", long="smoke", help="run only the @smoke subset (PR gate)",
    required=False, takes_value=False, is_flag=True,
)
@angreal.argument(
    name="visual", long="visual",
    help="run the @visual pixel suite (light and dark) instead of the functional one",
    required=False, takes_value=False, is_flag=True,
)
@angreal.argument(
    name="update_baselines", long="update-baselines",
    help="with --visual: write new baseline images instead of asserting",
    required=False, takes_value=False, is_flag=True,
)
def ui_e2e(smoke: bool = False, visual: bool = False, update_baselines: bool = False):
    return _ui_e2e(smoke, visual, update_baselines)
