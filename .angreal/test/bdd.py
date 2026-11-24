"""`angreal test bdd`: the behaviour specs (CLOACINA-A-0015).

The Gherkin features in crates/cloacina-bdd/features run in process through
cucumber, once per runner database: SQLite, then a Postgres schema of their
own. The scenarios' tables live in the Postgres at CLOACINA_BDD_PG_URL (the
dev Postgres on 15432 by default; `angreal services up` starts it).
"""

import os
import socket
import subprocess
from urllib.parse import urlparse

import angreal  # type: ignore

from ._utils import PROJECT_ROOT, print_final_success, print_section_header, wait_for_postgres_stable

test = angreal.command_group(name="test", about="Cloacina test suites (unit, integration, e2e, soak)")

DEFAULT_PG_URL = "postgres://cloacina:cloacina@localhost:15432/cloacina"


def _postgres_reachable(url):
    parsed = urlparse(url)
    try:
        with socket.create_connection((parsed.hostname or "localhost", parsed.port or 5432), timeout=3):
            return True
    except OSError:
        return False


@test()
@angreal.command(
    name="bdd",
    about="run the BDD features (Gherkin, in process) on SQLite and Postgres runners",
    when_to_use=["checking a product behaviour", "after changing Sync", "writing a new scenario"],
    when_not_to_use=["unit-level checks", "full server and agent e2e"],
)
@angreal.argument(
    name="runner",
    long="runner",
    required=False,
    help="the runner database: sqlite or postgres (default: both, one after the other)",
)
@angreal.argument(
    name="name",
    long="name",
    required=False,
    help="run only the scenarios whose name matches this regex",
)
@angreal.argument(
    name="tags",
    long="tags",
    required=False,
    help="run only the scenarios matching this tag expression, e.g. '@wip'",
)
@angreal.argument(
    name="start_postgres",
    long="start-postgres",
    takes_value=False,
    is_flag=True,
    help="start the compose Postgres first and wait until it is stable (the CI lane)",
)
@angreal.argument(
    name="junit",
    long="junit",
    required=False,
    help="write a JUnit report per runner, at <path>-<runner>.xml",
)
def bdd(runner=None, name=None, tags=None, start_postgres=False, junit=None):
    runners = [runner] if runner else ["sqlite", "postgres"]
    for r in runners:
        if r not in ("sqlite", "postgres"):
            raise RuntimeError(f"--runner is `{r}`: use sqlite or postgres")

    if start_postgres:
        print_section_header("Starting the compose Postgres")
        compose = PROJECT_ROOT / ".angreal/docker-compose.yaml"
        subprocess.run(["docker", "compose", "-f", str(compose), "up", "-d", "postgres"], check=True)
        wait_for_postgres_stable(compose_file=compose, cwd=PROJECT_ROOT)

    pg_url = os.environ.get("CLOACINA_BDD_PG_URL", DEFAULT_PG_URL)
    if not _postgres_reachable(pg_url):
        raise RuntimeError(
            f"No Postgres at {urlparse(pg_url).hostname}:{urlparse(pg_url).port}. "
            "Start the dev Postgres with `angreal services up`, or set CLOACINA_BDD_PG_URL."
        )

    cucumber_args = []
    if name:
        cucumber_args += ["--name", name]
    if tags:
        cucumber_args += ["--tags", tags]

    for r in runners:
        print_section_header(f"BDD features, runner on {r}")
        env = dict(os.environ, CLOACINA_BDD_RUNNER=r, CLOACINA_BDD_PG_URL=pg_url)
        if junit:
            # cargo runs the test binary in the crate's directory.
            env["CLOACINA_BDD_JUNIT"] = os.path.abspath(f"{junit}-{r}.xml")
        cmd = ["cargo", "test", "-p", "cloacina-bdd", "--test", "bdd", "--", *cucumber_args]
        print(f"Running: {' '.join(cmd)}")
        result = subprocess.run(cmd, cwd=str(PROJECT_ROOT), env=env, check=False)
        if result.returncode != 0:
            raise RuntimeError(f"BDD features failed with the runner on {r}")

    print_final_success("BDD features passed")
