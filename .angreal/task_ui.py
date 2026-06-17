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

import subprocess
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
COMPOSE_FILE = PROJECT_ROOT / "docker" / "docker-compose.demo.yml"

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
