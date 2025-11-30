import subprocess
import sys
import time
import angreal  # type: ignore

from utils import docker_up, docker_down, docker_clean

from .cloacina_utils import (
    print_section_header,
    print_final_success
)

# Define command group
cloacina = angreal.command_group(name="cloacina", about="commands for Cloacina core engine tests")


@cloacina()
@angreal.command(
    name="integration",
    about="run integration tests with backing services",
    when_to_use=["testing with real databases", "validating service integrations", "end-to-end testing"],
    when_not_to_use=["unit testing", "quick validation", "environments without Docker"]
)
@angreal.argument(
    name="filter",
    required=False,
    help="filter tests by name pattern"
)
@angreal.argument(
    name="skip_docker",
    long="skip-docker",
    help="skip Docker setup/teardown for manual service management",
    takes_value=False,
    is_flag=True
)
def integration(filter=None, skip_docker=False):
    """Run integration tests against both PostgreSQL and SQLite databases.

    Tests are compiled once with both backends enabled. The test fixture
    handles database selection internally.
    """

    if not skip_docker:
        # Start Docker services for PostgreSQL
        print_section_header("Starting Docker services")
        docker_down()
        docker_clean()
        exit_code = docker_up()
        if exit_code != 0:
            raise RuntimeError("Docker setup failed")
        # Wait for services to be ready
        print("Waiting for PostgreSQL to be ready...")
        time.sleep(30)

    try:
        print_section_header("Running integration tests")
        cmd = ["cargo", "test", "-p", "cloacina", "--test", "integration", "--features", "postgres,sqlite,macros", "--", "--test-threads=1", "--nocapture"]
        if filter:
            cmd.append(filter)

        subprocess.run(cmd, check=True)
        print_final_success("All integration tests passed!")
    except subprocess.CalledProcessError as e:
        print(f"Integration tests failed with error: {e}", file=sys.stderr)
        raise RuntimeError(f"Integration tests failed with return code {e.returncode}")
    finally:
        if not skip_docker:
            docker_down()
            docker_clean()
