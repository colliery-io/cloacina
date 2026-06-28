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

"""Cloacina web UI demo tasks (CLOACI-I-0117 / I-0124).

The web-UI demo runs entirely on the docker compose demo stack
(`docker/docker-compose.demo.yml`): Postgres + Kafka + a CORS-enabled
cloacina-server + the in-cluster compiler + a 3-agent execution fleet + the
fixtures packer/seed harness + a live computation-graph producer + the UI.

These commands are thin wrappers around that stack. There is no host-process
demo: the fixture set is defined by `docker/pack-demo-fixtures.sh` (the single
source of truth), and seeding/producing run as compose services, not host
processes.
"""

import json
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = PROJECT_ROOT / "docker" / "docker-compose.demo.yml"
# CLOACI-T-0816: fleet-actuator override (Docker actuator provisions agents
# dynamically instead of the static `agent`/`agent-acme` services).
FLEET_OVERRIDE = PROJECT_ROOT / "docker" / "docker-compose.demo.fleet.yml"

UI_URL = "http://localhost:8082"
SERVER_URL = "http://localhost:8080"
DEMO_BOOTSTRAP_KEY = "clk_demo_bootstrap_key_0001"

ui = angreal.command_group(name="ui", about="commands for the Cloacina web UI demo")


def _compose(*args) -> int:
    """Run `docker compose` against the demo stack from the repo root."""
    return subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE_FILE), *args],
        cwd=str(PROJECT_ROOT),
        check=False,
    ).returncode


def _compose_fleet(*args) -> int:
    """Run `docker compose` against the fleet-actuator demo variant (base +
    override) from the repo root."""
    return subprocess.run(
        ["docker", "compose", "-f", str(COMPOSE_FILE), "-f", str(FLEET_OVERRIDE), *args],
        cwd=str(PROJECT_ROOT),
        check=False,
    ).returncode


def _api(method, path, key=DEMO_BOOTSTRAP_KEY, expect=(200, 201, 409)):
    """Minimal REST helper against the demo server. Returns (status, body)."""
    req = urllib.request.Request(f"{SERVER_URL}{path}", method=method)
    req.add_header("Authorization", f"Bearer {key}")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            raw = resp.read().decode()
            code = resp.status
    except urllib.error.HTTPError as exc:
        raw = exc.read().decode()
        code = exc.code
    try:
        body = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        body = raw
    return code, body


def _wait_health(timeout_s=180):
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        try:
            code, _ = _api("GET", "/health")
            if code == 200:
                return True
        except Exception:
            pass
        time.sleep(2)
    return False


def _provision(tenant, n):
    """Provision a tenant's fleet to at least `n` desired agents (idempotent-ish:
    POST provision is +1 each, capped by the effective limit)."""
    code, fleet = _api("GET", f"/v1/tenants/{tenant}/fleet")
    have = fleet.get("desired_count", 0) if isinstance(fleet, dict) else 0
    for _ in range(max(0, n - have)):
        code, body = _api("POST", f"/v1/tenants/{tenant}/fleet/provision")
        if code == 409:
            break  # at effective limit
    code, fleet = _api("GET", f"/v1/tenants/{tenant}/fleet")
    return fleet.get("desired_count") if isinstance(fleet, dict) else None


def _reap_managed_agents():
    """Force-remove any leftover actuator-managed agent containers."""
    out = subprocess.run(
        ["docker", "ps", "-aq", "--filter", "label=cloacina.managed"],
        capture_output=True, text=True, check=False,
    ).stdout.split()
    if out:
        subprocess.run(["docker", "rm", "-f", *out], check=False)
    return len(out)


@ui()
@angreal.command(
    name="up",
    about="bring up the web-UI demo stack (docker compose)",
    long_about=(
        "Builds and starts the docker compose demo stack "
        "(docker/docker-compose.demo.yml): Postgres + Kafka + a CORS-enabled "
        "cloacina-server + in-cluster compiler + a 3-agent execution fleet + "
        "the fixtures packer/seed harness + a live computation-graph producer + "
        f"the UI on {UI_URL}. Open the UI and connect with the printed demo "
        "bootstrap key and tenant `public`. First `up` builds images and "
        "compiles the demo packages, so it takes a few minutes; `--no-build` "
        "skips the image rebuild for a fast restart."
    ),
    when_to_use=["running or demoing the web UI", "UI development against the full stack"],
    when_not_to_use=["production"],
)
@angreal.argument(
    name="no_build", long="no-build", help="skip image rebuilds (fast restart)",
    required=False, takes_value=False, is_flag=True,
)
def up(no_build: bool = False):
    args = ["up", "-d"]
    if not no_build:
        args.append("--build")
    rc = _compose(*args)
    if rc == 0:
        print("\n" + "=" * 60)
        print("  UI:      " + UI_URL)
        print("  Server:  " + SERVER_URL)
        print("  Connect with →  server:  " + SERVER_URL)
        print("                  api key: " + DEMO_BOOTSTRAP_KEY)
        print("                  tenant:  public")
        print("  Logs: `angreal ui logs`   Stop: `angreal ui down`")
        print("=" * 60 + "\n")
    return rc


@ui()
@angreal.command(
    name="down",
    about="stop the web-UI demo stack",
    long_about=(
        "Stops the docker compose demo stack. Pass --clean to also remove the "
        "volumes (database, Kafka, packed fixtures) for a fresh slate on the "
        "next `up`."
    ),
    when_to_use=["done with the demo", "resetting to a clean slate (with --clean)"],
    when_not_to_use=[],
)
@angreal.argument(
    name="clean", long="clean",
    help="also remove volumes (DB + Kafka + fixtures) for a fresh slate",
    required=False, takes_value=False, is_flag=True,
)
def down(clean: bool = False):
    args = ["down"]
    if clean:
        args.append("-v")
    return _compose(*args)


@ui()
@angreal.command(
    name="logs",
    about="tail the web-UI demo stack logs",
    when_to_use=["watching the demo stack", "debugging a service (server, agent, compiler, ui)"],
    when_not_to_use=[],
)
@angreal.argument(
    name="service", long="service",
    help="only this service (e.g. server, agent, compiler, ui, harness, producer)",
    required=False, takes_value=True, is_flag=False,
)
def logs(service: str = None):
    args = ["logs", "-f", "--tail", "100"]
    if service:
        args.append(service)
    return _compose(*args)


@ui()
@angreal.command(
    name="fleet-up",
    about="bring up the fleet-actuator demo variant (Docker actuator, CLOACI-I-0127)",
    long_about=(
        "Brings up the demo stack with the Docker fleet ACTUATOR instead of the "
        "static agent services (docker/docker-compose.demo.yml + "
        "docker-compose.demo.fleet.yml): cloacina-server provisions "
        "`cloacina-agent` containers DYNAMICALLY, minting a tenant-scoped key per "
        "spawn and labelling each `cloacina.managed=true` / `cloacina.tenant=<t>`. "
        "After the server is healthy this provisions the `public` and `acme` "
        "realms so the actuator spawns their agents (watch with "
        "`docker ps --filter label=cloacina.managed`). Tear down with "
        "`angreal ui fleet-down` (which also reaps any managed agent containers)."
    ),
    when_to_use=[
        "demoing I-0127 agent self-management (the Docker actuator)",
        "smoke-testing the Docker actuator end to end (real container spawn)",
    ],
    when_not_to_use=["production", "the static-fleet demo (use `angreal ui up`)"],
)
@angreal.argument(
    name="no_build", long="no-build", help="skip image rebuilds (fast restart)",
    required=False, takes_value=False, is_flag=True,
)
@angreal.argument(
    name="agents", long="agents",
    help="desired agents to provision for the public realm (default 2)",
    required=False, takes_value=True, is_flag=False,
)
def fleet_up(no_build: bool = False, agents: str = None):
    n_public = int(agents) if agents else 2
    args = ["up", "-d"]
    if not no_build:
        args.append("--build")
    rc = _compose_fleet(*args)
    if rc != 0:
        return rc

    print("\n  Waiting for server /health ...")
    if not _wait_health():
        print("  ERROR: server did not become healthy; check `angreal ui logs`")
        return 1

    pub = _provision("public", n_public)
    # acme auto-provisions on tenant create (CLOACINA_INITIAL_AGENTS); ensure >=1
    # in case harness-acme hasn't created it yet (it will be reconciled later).
    code, _ = _api("GET", "/v1/tenants/acme/fleet")
    acme = _provision("acme", 1) if code == 200 else "(pending tenant create)"

    print("\n" + "=" * 64)
    print("  Fleet-actuator demo UP (Docker actuator — I-0127 self-management)")
    print("  UI:      " + UI_URL)
    print("  Server:  " + SERVER_URL + "   key: " + DEMO_BOOTSTRAP_KEY)
    print(f"  Provisioned desired_count → public={pub}  acme={acme}")
    print("  The actuator is spawning agents. Watch them appear:")
    print("    docker ps --filter label=cloacina.managed")
    print("    curl -H 'Authorization: Bearer %s' %s/v1/agents" % (DEMO_BOOTSTRAP_KEY, SERVER_URL))
    print("  Logs: `angreal ui logs`   Stop: `angreal ui fleet-down`")
    print("=" * 64 + "\n")
    return 0


@ui()
@angreal.command(
    name="fleet-down",
    about="stop the fleet-actuator demo variant + reap managed agent containers",
    long_about=(
        "Stops the fleet-actuator demo stack (base + override) and force-removes "
        "any leftover actuator-managed `cloacina-agent` containers "
        "(label cloacina.managed=true) — those are spawned OUTSIDE the compose "
        "project, so `compose down` alone does not reap them. Pass --clean to also "
        "remove volumes for a fresh slate."
    ),
    when_to_use=["done with the fleet demo", "cleaning up spawned agent containers"],
    when_not_to_use=[],
)
@angreal.argument(
    name="clean", long="clean",
    help="also remove volumes (DB + Kafka + fixtures) for a fresh slate",
    required=False, takes_value=False, is_flag=True,
)
def fleet_down(clean: bool = False):
    args = ["down"]
    if clean:
        args.append("-v")
    rc = _compose_fleet(*args)
    reaped = _reap_managed_agents()
    print(f"  Reaped {reaped} actuator-managed agent container(s).")
    return rc
