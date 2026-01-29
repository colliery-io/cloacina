#  Copyright 2025-2026 Colliery Software
#
#  Licensed under the Apache License, Version 2.0 (the "License");
#  you may not use this file except in compliance with the License.
#  You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.

"""``cloaca build`` command — create a ``.cloacina`` package."""

from __future__ import annotations

import hashlib
import shutil
import tarfile
from datetime import datetime, timezone
from pathlib import Path

import click

from cloaca.discovery import discover_tasks
from cloaca.manifest import (
    Manifest,
    PackageInfo,
    PythonRuntime,
    SUPPORTED_TARGETS,
    detect_current_platform,
)
from cloaca.pyproject import parse_pyproject
from cloaca.vendoring import vendor_dependencies, VendoringError


@click.command()
@click.option(
    "-o",
    "--output",
    type=click.Path(),
    default=".",
    help="Output directory for the package.",
)
@click.option(
    "--target",
    "targets",
    multiple=True,
    help="Target platform(s). Default: current platform.",
)
@click.option("-v", "--verbose", is_flag=True, help="Verbose output.")
@click.option("--dry-run", is_flag=True, help="Show what would be built.")
def build(
    output: str,
    targets: tuple[str, ...],
    verbose: bool,
    dry_run: bool,
) -> None:
    """Build a .cloacina package from the current Python project."""
    project_dir = Path.cwd()
    pyproject_path = project_dir / "pyproject.toml"

    # 1. Parse pyproject.toml
    try:
        pyproject = parse_pyproject(pyproject_path)
    except ValueError as exc:
        raise click.ClickException(str(exc)) from exc

    # 2. Determine targets
    target_list = list(targets) if targets else [detect_current_platform()]
    for t in target_list:
        if t not in SUPPORTED_TARGETS:
            raise click.ClickException(f"Unsupported target: {t}")

    # 3. Discover tasks
    cloaca_cfg = pyproject["tool"]["cloaca"]
    entry_module: str = cloaca_cfg["entry_module"]

    try:
        tasks = discover_tasks(entry_module, project_dir)
    except FileNotFoundError as exc:
        raise click.ClickException(str(exc)) from exc

    if not tasks:
        raise click.ClickException(
            f"No @task decorated functions found in {entry_module}"
        )

    click.echo(f"Found {len(tasks)} task(s): {', '.join(t.id for t in tasks)}")

    # 4. Build manifest
    project_cfg = pyproject["project"]
    package_name: str = project_cfg["name"]
    package_version: str = project_cfg["version"]
    requires_python: str = project_cfg.get("requires-python", ">=3.10")

    manifest = Manifest(
        package=PackageInfo(
            name=package_name,
            version=package_version,
            description=project_cfg.get("description"),
            targets=target_list,
        ),
        language="python",
        python=PythonRuntime(
            requires_python=requires_python,
            entry_module=entry_module,
        ),
        tasks=tasks,
        created_at=datetime.now(timezone.utc),
    )

    if dry_run:
        click.echo("\nDry run — would create package:")
        click.echo(f"  Name:    {package_name}")
        click.echo(f"  Version: {package_version}")
        click.echo(f"  Targets: {', '.join(target_list)}")
        click.echo(f"  Tasks:   {len(tasks)}")
        return

    # 5. Stage build directory
    build_dir = project_dir / ".cloaca_build"
    if build_dir.exists():
        shutil.rmtree(build_dir)
    build_dir.mkdir(parents=True)

    # Copy workflow source
    workflow_dir = build_dir / "workflow"
    _copy_workflow_source(project_dir, workflow_dir, entry_module)

    # Write manifest
    manifest_path = build_dir / "manifest.json"
    manifest.write_to_file(manifest_path)

    # Vendor dependencies
    vendor_dir = build_dir / "vendor"
    try:
        vendor_result = vendor_dependencies(
            project_dir=project_dir,
            vendor_dir=vendor_dir,
            targets=target_list,
            verbose=verbose,
        )
    except VendoringError as exc:
        raise click.ClickException(f"Vendoring failed: {exc}") from exc

    # 6. Create archive
    output_dir = Path(output)
    output_dir.mkdir(parents=True, exist_ok=True)

    archive_name = f"{package_name}-{package_version}.cloacina"
    archive_path = output_dir / archive_name

    with tarfile.open(archive_path, "w:gz") as tar:
        tar.add(manifest_path, arcname="manifest.json")
        tar.add(workflow_dir, arcname="workflow")
        tar.add(vendor_dir, arcname="vendor")
        lock_file = build_dir / "requirements.lock"
        if vendor_result.lock_file and vendor_result.lock_file.exists():
            shutil.copy2(vendor_result.lock_file, lock_file)
        if lock_file.exists():
            tar.add(lock_file, arcname="requirements.lock")

    # 7. Compute fingerprint
    fingerprint = f"sha256:{_compute_sha256(archive_path)}"
    manifest.package.fingerprint = fingerprint

    # Rewrite manifest inside archive with fingerprint
    manifest.write_to_file(manifest_path)
    with tarfile.open(archive_path, "w:gz") as tar:
        tar.add(manifest_path, arcname="manifest.json")
        tar.add(workflow_dir, arcname="workflow")
        tar.add(vendor_dir, arcname="vendor")
        if lock_file.exists():
            tar.add(lock_file, arcname="requirements.lock")

    # Clean up build dir
    shutil.rmtree(build_dir)

    click.echo(f"\nCreated: {archive_path}")
    click.echo(f"Fingerprint: {fingerprint}")


def _copy_workflow_source(
    project_dir: Path, dest: Path, entry_module: str
) -> None:
    """Copy the entry module's package tree into *dest*."""
    top_package = entry_module.split(".")[0]
    src = project_dir / top_package

    if src.is_dir():
        shutil.copytree(src, dest / top_package, dirs_exist_ok=True)
    else:
        # Single-file module
        src_file = project_dir / f"{top_package}.py"
        dest.mkdir(parents=True, exist_ok=True)
        if src_file.exists():
            shutil.copy2(src_file, dest / src_file.name)


def _compute_sha256(path: Path) -> str:
    sha256 = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha256.update(chunk)
    return sha256.hexdigest()
