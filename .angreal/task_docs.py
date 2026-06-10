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
"""

import difflib
import subprocess
import sys
import shutil
from pathlib import Path

import angreal  # type: ignore

# Project root for accessing docs, etc. (one level up from .angreal)
PROJECT_ROOT = Path(angreal.get_root()).parent

# Define command group
docs = angreal.command_group(name="docs", about="commands for documentation tasks")


def _clean_docs():
    """Clean the documentation build directory."""
    public_dir = PROJECT_ROOT / "docs" / "public"
    if public_dir.exists():
        print("Cleaning documentation build directory...")
        shutil.rmtree(public_dir)
        print("Clean complete!")
    return 0


def _integrate_rustdoc():
    """Generate rustdoc and integrate it with the Hugo documentation site."""
    print("Generating rustdoc...")

    # Generate rustdoc
    try:
        subprocess.run(
            ["cargo", "doc", "--no-deps"],
            check=True
        )
    except subprocess.CalledProcessError as e:
        print(f"Failed to generate rustdoc: {e}", file=sys.stderr)
        return e.returncode

    # Setup paths
    hugo_docs_dir = PROJECT_ROOT / "docs"
    rustdoc_output_dir = PROJECT_ROOT / "target/doc"
    hugo_api_dir = hugo_docs_dir / "static/api"

    # Create Hugo API directory if it doesn't exist
    hugo_api_dir.mkdir(parents=True, exist_ok=True)

    # Copy rustdoc output to Hugo static directory
    print("Copying rustdoc output to Hugo...")
    try:
        # Use rsync for better file copying (preserves permissions, handles existing files better)
        subprocess.run(
            ["rsync", "-av", "--delete", f"{rustdoc_output_dir}/", str(hugo_api_dir)],
            check=True
        )
    except subprocess.CalledProcessError as e:
        print(f"Failed to copy rustdoc output: {e}", file=sys.stderr)
        return e.returncode

    print("Rustdoc integration complete!")
    return 0


@docs()
@angreal.command(
    name="clean",
    about="clean the documentation build directory",
    when_to_use=["rebuilding docs", "clearing build cache", "fixing build issues"],
    when_not_to_use=["during active development", "wanting faster builds"]
)
def clean():
    """Clean the documentation build directory."""
    return _clean_docs()


@docs()
@angreal.command(
    name="serve",
    about="serve the documentation site locally, by default building draft documents.",
    when_to_use=["local development", "previewing docs", "testing documentation changes"],
    when_not_to_use=["production builds", "automated CI/CD", "when not actively writing docs"]
)
@angreal.argument(
    name="prod",
    long="prod",
    help="exclude draft content to match production build",
    required=False,
    takes_value=False,
    is_flag=True
)
def serve(prod: bool = False):
    """Serve the Hugo documentation site locally with integrated API docs.

    Args:
        prod: If True, excludes draft content from the build. Defaults to False.
    """
    print("=== Setting up documentation ===")

    # Clean the build directory first
    clean_result = _clean_docs()
    if clean_result != 0:
        return clean_result

    # First integrate rustdoc
    print("\nIntegrating API documentation...")
    rustdoc_result = _integrate_rustdoc()
    if rustdoc_result != 0:
        return rustdoc_result

    # Then start Hugo server
    print("\n=== Starting Hugo server ===")
    print("Documentation will be available at http://localhost:1313")
    print("Press Ctrl+C to stop the server")

    try:
        # By default include drafts (-D), unless prod flag is set
        cmd = ["hugo", "server", "-D"]
        if prod:
            cmd.remove("-D")
            print("Excluding draft content from build")
        else:
            print("Including draft content in build")

        result = subprocess.run(
            cmd,
            cwd=str(PROJECT_ROOT / "docs"),
            check=True
        )
        return result.returncode
    except subprocess.CalledProcessError as e:
        print(f"Hugo server failed: {e}", file=sys.stderr)
        return e.returncode


@docs()
@angreal.command(
    name="spec-check",
    about="verify the committed OpenAPI spec matches a fresh emit-openapi (CI drift gate)",
    when_to_use=["CI", "after changing server routes or DTOs", "before tagging a release"],
    when_not_to_use=["regenerating the spec — run emit-openapi and commit instead"]
)
def spec_check():
    """Diff docs/static/openapi.json against a freshly emitted spec.

    The committed spec is the public API contract (CLOACI-T-0643 / NFR-001).
    Any change to server routes or cloacina-api-types DTOs must be
    accompanied by a regenerated spec:

        cargo run -p cloacina-server --bin cloacina-server -- emit-openapi \\
            > docs/static/openapi.json
    """
    committed_path = PROJECT_ROOT / "docs" / "static" / "openapi.json"
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

    print("OpenAPI SPEC DRIFT — committed docs/static/openapi.json does not "
          "match emit-openapi output:\n", file=sys.stderr)
    diff = difflib.unified_diff(
        committed.splitlines(keepends=True),
        fresh.splitlines(keepends=True),
        fromfile="docs/static/openapi.json (committed)",
        tofile="emit-openapi (fresh)",
    )
    sys.stderr.writelines(diff)
    print("\nRegenerate with:\n  cargo run -p cloacina-server --bin "
          "cloacina-server -- emit-openapi > docs/static/openapi.json",
          file=sys.stderr)
    return 1


@docs()
@angreal.command(
    name="build",
    about="build the documentation site, by default excluding draft documents.",
    when_to_use=["production deployment", "CI/CD pipelines", "final documentation builds"],
    when_not_to_use=["active development", "testing draft content", "quick previews"]
)
@angreal.argument(
    name="draft",
    long="draft",
    help="include draft content in build output",
    required=False,
    takes_value=False,
    is_flag=True
)
def build(draft: bool = False):
    """Build the Hugo documentation site with integrated API docs.

    Args:
        draft: If True, includes draft content in the build. Defaults to False.
    """
    print("=== Building documentation site ===")

    # Clean the build directory first
    clean_result = _clean_docs()
    if clean_result != 0:
        return clean_result

    # First integrate rustdoc
    print("\nIntegrating API documentation...")
    rustdoc_result = _integrate_rustdoc()
    if rustdoc_result != 0:
        return rustdoc_result

    # Then build Hugo site
    print("\nBuilding Hugo site...")
    try:
        # By default exclude drafts, unless draft flag is set
        cmd = ["hugo"]
        if draft:
            cmd.append("-D")
            print("Including draft content in build")
        else:
            print("Excluding draft content from build (production mode)")

        result = subprocess.run(
            cmd,
            cwd=str(PROJECT_ROOT / "docs"),
            check=True
        )
        if result.returncode == 0:
            print("\n=== Build complete ===")
            print(f"Documentation site built in {PROJECT_ROOT}/docs/public")
        return result.returncode
    except subprocess.CalledProcessError as e:
        print(f"Failed to build documentation: {e}", file=sys.stderr)
        return e.returncode
