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

"""Dependency vendoring using ``uv`` for platform-specific wheel resolution."""

from __future__ import annotations

import re
import subprocess
import tempfile
import zipfile
from dataclasses import dataclass, field
from pathlib import Path


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------

class VendoringError(Exception):
    """Base class for vendoring errors."""


class DependencyResolutionError(VendoringError):
    """Failed to resolve dependencies."""


class SdistOnlyError(VendoringError):
    """Package only has source distribution, no wheel."""


class DownloadError(VendoringError):
    """Failed to download wheel."""


# ---------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------

@dataclass
class ResolvedDependency:
    """A single resolved dependency with version and hash."""

    name: str
    version: str
    hash_sha256: str


@dataclass
class VendorResult:
    """Result of the vendoring pipeline."""

    vendor_dir: Path
    dependencies: list[ResolvedDependency] = field(default_factory=list)
    lock_file: Path | None = None


# Map our platform targets to uv's --platform argument.
_UV_PLATFORM_MAP: dict[str, str] = {
    "linux-x86_64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "macos-x86_64": "x86_64-apple-darwin",
    "macos-arm64": "aarch64-apple-darwin",
}

_REQ_LINE_RE = re.compile(
    r"^(?P<name>[\w._-]+)==(?P<version>[\w._-]+)"
)
_HASH_RE = re.compile(r"--hash=sha256:([0-9a-f]+)")


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------

def check_uv_available() -> str:
    """Verify ``uv`` is installed. Returns version string or raises."""
    try:
        result = subprocess.run(
            ["uv", "--version"],
            capture_output=True,
            text=True,
            check=True,
        )
        return result.stdout.strip()
    except FileNotFoundError as exc:
        raise RuntimeError(
            "uv is not installed. Install with: "
            "curl -LsSf https://astral.sh/uv/install.sh | sh"
        ) from exc


def vendor_dependencies(
    project_dir: Path,
    vendor_dir: Path,
    targets: list[str],
    *,
    python_version: str = "3.11",
    verbose: bool = False,
) -> VendorResult:
    """Run the full vendoring pipeline.

    1. Resolve dependencies with ``uv pip compile``
    2. Download platform-specific wheels with ``uv pip download``
    3. Extract wheels into *vendor_dir*
    4. Write ``requirements.lock``

    Returns a :class:`VendorResult` with paths and dependency metadata.
    """
    check_uv_available()

    pyproject_path = project_dir / "pyproject.toml"
    if not pyproject_path.exists():
        raise VendoringError(f"pyproject.toml not found in {project_dir}")

    # For multi-target we use the first target for resolution (pure-python
    # deps are platform-agnostic; platform-specific wheels must be fetched
    # per target but we only generate one vendor dir for now).
    target = targets[0]

    # 1. Resolve
    if verbose:
        print(f"Resolving dependencies for {target}...")
    deps = _resolve_dependencies(pyproject_path, target, python_version)

    if not deps:
        vendor_dir.mkdir(parents=True, exist_ok=True)
        return VendorResult(vendor_dir=vendor_dir)

    # 2. Download
    if verbose:
        print(f"Downloading {len(deps)} wheel(s)...")
    wheels = _download_wheels(deps, target, python_version)

    # 3. Extract
    vendor_dir.mkdir(parents=True, exist_ok=True)
    _extract_wheels(wheels, vendor_dir)

    # Record what was vendored
    record = vendor_dir / "VENDORED.txt"
    record.write_text(
        "\n".join(f"{d.name}=={d.version}" for d in deps) + "\n",
        encoding="utf-8",
    )

    # 4. Lock file
    lock_file = vendor_dir.parent / "requirements.lock"
    _generate_lock_file(deps, lock_file, target)

    return VendorResult(
        vendor_dir=vendor_dir,
        dependencies=deps,
        lock_file=lock_file,
    )


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------

def _resolve_dependencies(
    pyproject_path: Path,
    target: str,
    python_version: str,
) -> list[ResolvedDependency]:
    """Use ``uv pip compile`` to resolve pinned dependencies."""
    uv_platform = _UV_PLATFORM_MAP.get(target)
    if uv_platform is None:
        raise VendoringError(f"No uv platform mapping for target: {target}")

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".txt", delete=False
    ) as tmp:
        lock_path = Path(tmp.name)

    cmd = [
        "uv", "pip", "compile",
        str(pyproject_path),
        "--output-file", str(lock_path),
        "--python-version", python_version,
        "--python-platform", uv_platform,
        "--generate-hashes",
    ]

    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise DependencyResolutionError(
            f"uv pip compile failed:\n{result.stderr}"
        )

    return _parse_lock_file(lock_path)


def _parse_lock_file(path: Path) -> list[ResolvedDependency]:
    """Parse a pip-compile style requirements file into resolved deps."""
    deps: list[ResolvedDependency] = []
    current_name: str | None = None
    current_version: str | None = None
    current_hash: str | None = None

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue

        # Continuation lines with hashes
        hash_match = _HASH_RE.search(line)
        if hash_match and current_name is not None:
            current_hash = hash_match.group(1)
            # A line may also start a new requirement *and* contain a hash,
            # but typically hashes are on continuation lines.

        req_match = _REQ_LINE_RE.match(line)
        if req_match:
            # Flush previous
            if current_name is not None and current_version is not None:
                deps.append(
                    ResolvedDependency(
                        name=current_name,
                        version=current_version,
                        hash_sha256=current_hash or "",
                    )
                )
            current_name = req_match.group("name")
            current_version = req_match.group("version")
            current_hash = hash_match.group(1) if hash_match else None

    # Flush last
    if current_name is not None and current_version is not None:
        deps.append(
            ResolvedDependency(
                name=current_name,
                version=current_version,
                hash_sha256=current_hash or "",
            )
        )

    return deps


def _download_wheels(
    deps: list[ResolvedDependency],
    target: str,
    python_version: str,
) -> list[Path]:
    """Download wheels using ``uv pip download``."""
    uv_platform = _UV_PLATFORM_MAP[target]

    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".txt", delete=False
    ) as req_file:
        for dep in deps:
            req_file.write(f"{dep.name}=={dep.version}\n")
        req_path = Path(req_file.name)

    wheel_dir = Path(tempfile.mkdtemp(prefix="cloaca-wheels-"))

    cmd = [
        "uv", "pip", "download",
        "-r", str(req_path),
        "--dest", str(wheel_dir),
        "--python-platform", uv_platform,
        "--python-version", python_version,
        "--only-binary", ":all:",
    ]

    result = subprocess.run(cmd, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        if "No matching distribution" in result.stderr:
            raise SdistOnlyError(
                "Some packages only have source distributions. "
                "All Python packages must have pre-built wheels for packaging.\n"
                f"Details: {result.stderr}"
            )
        raise DownloadError(f"uv pip download failed:\n{result.stderr}")

    return list(wheel_dir.glob("*.whl"))


def _extract_wheels(wheels: list[Path], vendor_dir: Path) -> None:
    """Extract wheel contents into *vendor_dir*."""
    for whl in wheels:
        with zipfile.ZipFile(whl, "r") as zf:
            zf.extractall(vendor_dir)


def _generate_lock_file(
    deps: list[ResolvedDependency],
    output_path: Path,
    target: str,
) -> None:
    """Write a ``requirements.lock`` with pinned versions and hashes."""
    lines = [
        f"# Generated by cloaca build for {target}",
        "#",
    ]
    for dep in sorted(deps, key=lambda d: d.name.lower()):
        entry = f"{dep.name}=={dep.version}"
        if dep.hash_sha256:
            entry += f" \\\n    --hash=sha256:{dep.hash_sha256}"
        lines.append(entry)

    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
