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

"""Unified package manifest (v2) for Python workflow packages.

Pydantic models that mirror the Rust-side ManifestV2 schema, used by
the ``cloaca build`` CLI to generate manifest.json files.
"""

from __future__ import annotations

import json
import platform
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field, model_validator

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

SUPPORTED_TARGETS: list[str] = [
    "linux-x86_64",
    "linux-arm64",
    "macos-x86_64",
    "macos-arm64",
]

_FUNCTION_PATH_RE = re.compile(r"^[\w.]+:\w+$")


def detect_current_platform() -> str:
    """Return the current platform as a target string."""
    system = platform.system().lower()
    machine = platform.machine().lower()

    os_map = {"linux": "linux", "darwin": "macos"}
    arch_map = {"x86_64": "x86_64", "amd64": "x86_64", "aarch64": "arm64", "arm64": "arm64"}

    os_name = os_map.get(system)
    arch_name = arch_map.get(machine)

    if os_name and arch_name:
        return f"{os_name}-{arch_name}"
    return "unknown"


# ---------------------------------------------------------------------------
# Models
# ---------------------------------------------------------------------------


class TaskDefinition(BaseModel):
    """A single task within a workflow package."""

    id: str
    function: str = Field(description="Python: 'module.path:function_name'")
    dependencies: list[str] = Field(default_factory=list)
    description: str | None = None
    retries: int = 0
    timeout_seconds: int | None = None


class PythonRuntime(BaseModel):
    """Python-specific runtime requirements."""

    requires_python: str = Field(description="PEP 440 version specifier, e.g. '>=3.10'")
    entry_module: str = Field(description="Dotted module path, e.g. 'workflow.tasks'")


class RustRuntime(BaseModel):
    """Rust-specific runtime requirements."""

    library_path: str = Field(description="Relative path to .so/.dylib inside the package")


class PackageInfo(BaseModel):
    """Package metadata."""

    name: str
    version: str
    description: str | None = None
    fingerprint: str = ""
    targets: list[str] = Field(default_factory=list)


class Manifest(BaseModel):
    """Unified package manifest (v2).

    Mirrors the Rust ``ManifestV2`` struct for cross-language compatibility.
    """

    format_version: Literal["2"] = "2"
    package: PackageInfo
    language: Literal["python", "rust"]
    python: PythonRuntime | None = None
    rust: RustRuntime | None = None
    tasks: list[TaskDefinition]
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    signature: str | None = None

    @model_validator(mode="after")
    def _validate_runtime(self) -> "Manifest":
        if self.language == "python" and self.python is None:
            raise ValueError("Python package requires 'python' runtime config")
        if self.language == "rust" and self.rust is None:
            raise ValueError("Rust package requires 'rust' runtime config")
        return self

    def validate_targets(self) -> None:
        """Raise ``ValueError`` if any target is unsupported."""
        for t in self.package.targets:
            if t not in SUPPORTED_TARGETS:
                raise ValueError(f"Unsupported target platform: {t}")

    def validate_tasks(self) -> None:
        """Raise ``ValueError`` on duplicate IDs, bad deps, or bad function paths."""
        ids = set()
        for task in self.tasks:
            if task.id in ids:
                raise ValueError(f"Duplicate task ID: '{task.id}'")
            ids.add(task.id)

        for task in self.tasks:
            for dep in task.dependencies:
                if dep not in ids:
                    raise ValueError(
                        f"Task '{task.id}' depends on unknown task '{dep}'"
                    )

        if self.language == "python":
            for task in self.tasks:
                if not _FUNCTION_PATH_RE.match(task.function):
                    raise ValueError(
                        f"Invalid function path '{task.function}': "
                        "expected 'module.path:function_name'"
                    )

    def validate_all(self) -> None:
        """Run all validation checks."""
        self.validate_targets()
        self.validate_tasks()

    # -- I/O helpers --------------------------------------------------------

    def to_json(self, **kwargs: object) -> str:
        """Serialize to a JSON string."""
        return self.model_dump_json(indent=2, **kwargs)

    def write_to_file(self, path: str | Path) -> None:
        """Write manifest JSON to *path*."""
        Path(path).write_text(self.to_json(), encoding="utf-8")

    @classmethod
    def read_from_file(cls, path: str | Path) -> "Manifest":
        """Read and parse a manifest from *path*."""
        data = json.loads(Path(path).read_text(encoding="utf-8"))
        return cls.model_validate(data)
