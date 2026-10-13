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
starlight-aurora. They need Node 22 or later, and plissken 0.2.1 for the API
reference: `plissken render` writes it into docs/src/content/docs/reference/api/
(see plissken.toml). Those pages are not in git; CI renders them the same way.
"""

import difflib
import hashlib
import os
import shutil
import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

# Project root for accessing docs, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent
DOCS_ROOT = PROJECT_ROOT / "docs"

# The plissken version CI uses (.github/workflows/docs.yml).
PLISSKEN_VERSION = "0.2.1"
# plissken has no --version flag. The binaries of the v0.2.1 release have these
# sha256 sums; a `cargo install` build is checked with `cargo install --list`.
PLISSKEN_SHA256 = {
    "ee88d4725c1b93e564fbf568d513401184e400b1ed243bee4cee2f15c7b37426",  # aarch64-apple-darwin
    "698903ec9017eeffa0b608153089099df1e22188a8edd281cd3cde81b60e1a83",  # x86_64-apple-darwin
    "414beb801ba0856c6423b894135e56e360d1ba0a932169cd379a99a288a6ce46",  # x86_64-unknown-linux-gnu
    "8f9c2853dc147940fe383aeff0012534b8e7b592414eaa25f61da34d4ab0edd0",  # x86_64-unknown-linux-musl
    "b97c6f68e436250e1dbaec54135fea614d0152ed9a47fddc9b777d18b1aa9087",  # aarch64-unknown-linux-gnu
}
PLISSKEN_INSTALL = (
    f"install plissken {PLISSKEN_VERSION} from "
    f"https://github.com/colliery-io/plissken/releases/tag/v{PLISSKEN_VERSION} "
    f"or with `cargo install plissken --version {PLISSKEN_VERSION} --locked`"
)

# Define command group
docs = angreal.command_group(name="docs", about="commands for documentation tasks")


def _is_cargo_install(plissken):
    cargo = shutil.which("cargo")
    if not cargo:
        return False
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    if Path(plissken).resolve() != (cargo_home / "bin" / "plissken").resolve():
        return False
    listed = subprocess.run(
        [cargo, "install", "--list"], capture_output=True, text=True, check=False
    ).stdout
    return f"plissken v{PLISSKEN_VERSION}:" in listed


def _plissken():
    """Render the API reference with plissken 0.2.1."""
    plissken = shutil.which("plissken")
    if not plissken:
        print(f"plissken not found: {PLISSKEN_INSTALL}", file=sys.stderr)
        return 1
    digest = hashlib.sha256(Path(plissken).read_bytes()).hexdigest()
    if digest not in PLISSKEN_SHA256 and not _is_cargo_install(plissken):
        print(f"{plissken} is not plissken {PLISSKEN_VERSION}: {PLISSKEN_INSTALL}", file=sys.stderr)
        return 1
    return subprocess.run([plissken, "render"], cwd=str(PROJECT_ROOT), check=False).returncode


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
    about="render the API reference and serve the docs site with live reload",
    when_to_use=["local development", "previewing docs", "testing documentation changes"],
    when_not_to_use=["production builds", "automated CI/CD"],
)
def serve():
    """Render the API reference (plissken 0.2.1), then serve the site with live reload."""
    return _plissken() or _npm("run", "dev")


@docs()
@angreal.command(
    name="api",
    about="render the API reference only (plissken render, plissken 0.2.1)",
    when_to_use=["after changing doc comments", "checking the API pages"],
    when_not_to_use=["building the whole site (use build)"],
)
def api():
    """Write the API pages and _sidebar.mjs into docs/src/content/docs/reference/api/.

    Fails when plissken is missing or is not 0.2.1. The pages are git-ignored.
    """
    return _plissken()


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
    about="render the API reference and build the docs site into docs/dist",
    when_to_use=["CI/CD pipelines", "checking that each page builds"],
    when_not_to_use=["active development (use serve)"],
)
def build():
    """Render the API reference (plissken 0.2.1), then build the site into docs/dist."""
    return _plissken() or _npm("run", "build")
