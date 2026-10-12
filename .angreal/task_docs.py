# Copyright 2024 Cloacina Contributors
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

"""
Documentation tasks for Cloacina.

The docs are an Astro Starlight site in docs/, with the Colliery theme
starlight-aurora. They need Node 22 or later.
"""

import difflib
import shutil
import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

# Project root for accessing docs, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent
DOCS_ROOT = PROJECT_ROOT / "docs"

# Define command group
docs = angreal.command_group(name="docs", about="commands for documentation tasks")


def _npm(*args):
    npm = shutil.which("npm")
    if not npm:
        print("npm not found: install Node 22 or later", file=sys.stderr)
        return 1
    if not (DOCS_ROOT / "node_modules").exists():
        result = subprocess.run([npm, "ci", "--no-audit", "--no-fund"], cwd=str(DOCS_ROOT), check=False)
        if result.returncode != 0:
            return result.returncode
    return subprocess.run([npm, *args], cwd=str(DOCS_ROOT), check=False).returncode


@docs()
@angreal.command(
    name="clean",
    about="delete the built docs site (docs/dist)",
    when_to_use=["rebuilding docs", "fixing build issues"],
    when_not_to_use=["during active development"]
)
def clean():
    """Delete docs/dist and the Astro cache."""
    for path in (DOCS_ROOT / "dist", DOCS_ROOT / ".astro"):
        if path.exists():
            shutil.rmtree(path)
    return 0


@docs()
@angreal.command(
    name="serve",
    about="serve the docs site with live reload",
    when_to_use=["local development", "previewing docs", "testing documentation changes"],
    when_not_to_use=["production builds", "automated CI/CD"],
)
def serve():
    """Serve the Starlight site with live reload."""
    return _npm("run", "dev")


@docs()
@angreal.command(
    name="spec-check",
    about="verify the committed OpenAPI spec matches a fresh emit-openapi (CI drift gate)",
    when_to_use=["CI", "after changing server routes or DTOs", "before tagging a release"],
    when_not_to_use=["regenerating the spec — run emit-openapi and commit instead"]
)
def spec_check():
    """Diff docs/public/openapi.json against a freshly emitted spec.

    The committed spec is the public API contract (CLOACI-T-0643 / NFR-001).
    Any change to server routes or cloacina-api-types DTOs must be
    accompanied by a regenerated spec:

        cargo run -p cloacina-server --bin cloacina-server -- emit-openapi \\
            > docs/public/openapi.json
    """
    committed_path = PROJECT_ROOT / "docs" / "public" / "openapi.json"
    if not committed_path.exists():
        print(f"Committed spec missing: {committed_path}", file=sys.stderr)
        return 1

    print("Emitting OpenAPI spec from cloacina-server...")
    try:
        result = subprocess.run(
            ["cargo", "run", "-q", "-p", "cloacina-server",
             "--bin", "cloacina-server", "--", "emit-openapi"],
            check=True,
            capture_output=True,
            text=True,
            cwd=str(PROJECT_ROOT),
        )
    except subprocess.CalledProcessError as e:
        print(f"emit-openapi failed: {e}", file=sys.stderr)
        print(e.stderr, file=sys.stderr)
        return e.returncode

    fresh = result.stdout
    committed = committed_path.read_text()

    if fresh.rstrip("\n") == committed.rstrip("\n"):
        print("OpenAPI spec is in sync.")
        return 0

    print("OpenAPI SPEC DRIFT — committed docs/public/openapi.json does not "
          "match emit-openapi output:\n", file=sys.stderr)
    diff = difflib.unified_diff(
        committed.splitlines(keepends=True),
        fresh.splitlines(keepends=True),
        fromfile="docs/public/openapi.json (committed)",
        tofile="emit-openapi (fresh)",
    )
    sys.stderr.writelines(diff)
    print("\nRegenerate with:\n  cargo run -p cloacina-server --bin "
          "cloacina-server -- emit-openapi > docs/public/openapi.json",
          file=sys.stderr)
    return 1


@docs()
@angreal.command(
    name="build",
    about="build the docs site into docs/dist",
    when_to_use=["CI/CD pipelines", "checking that each page builds"],
    when_not_to_use=["active development (use serve)"],
)
def build():
    """Build the Starlight site into docs/dist."""
    return _npm("run", "build")
