"""
Development tools for Cloacina.
"""

import angreal  # type: ignore
import subprocess
import sys
import time
from pathlib import Path

from utils import docker_up, docker_down

# Define command group
dev = angreal.command_group(name="dev", about="commands for development tools")


@dev()
@angreal.command(name="docs", about="generate and serve Rust documentation")
def docs():
    """Generate and serve Rust documentation."""
    subprocess.run(["cargo", "doc", "--no-deps", "--open"])
    return 0


@dev()
@angreal.command(name="schema", about="generate `src/schema.rs` from current migrations")
def schema():
    """Generate schema.rs from current migrations."""
    try:
        # Start Docker services
        exit_code = docker_up()
        if exit_code != 0:
            return exit_code

        # Wait for services to be ready
        print("Waiting for services to be ready...")
        time.sleep(30)

        db_url = "postgres://cloacina:cloacina@localhost:5432/cloacina"

        # Run migrations
        subprocess.run(
            f"diesel migration run --database-url {db_url}",
            cwd=str(Path(angreal.get_root()).parent / "cloacina/src/database"),
            shell=True,
            check=True
        )

        # Generate schema
        subprocess.run(
            f"diesel print-schema --database-url {db_url} > schema.rs",
            cwd=str(Path(angreal.get_root()).parent / "cloacina/src/database"),
            shell=True,
            check=True
        )

        print("Schema generated successfully!")
        return 0
    except subprocess.CalledProcessError as e:
        print(f"Schema generation failed with error: {e}", file=sys.stderr)
        return e.returncode
    finally:
        # Stop Docker services
        docker_down()
