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

"""Parse and validate ``pyproject.toml`` for cloaca build."""

from __future__ import annotations

import tomllib
from pathlib import Path


def parse_pyproject(path: Path) -> dict:
    """Parse *pyproject.toml* and validate required fields for cloaca.

    Raises :class:`ValueError` if required sections or fields are missing.
    """
    if not path.exists():
        raise ValueError(f"pyproject.toml not found at {path}")

    with open(path, "rb") as f:
        data = tomllib.load(f)

    if "project" not in data:
        raise ValueError("Missing [project] section in pyproject.toml")

    project = data["project"]
    for field in ("name", "version"):
        if field not in project:
            raise ValueError(f"Missing project.{field} in pyproject.toml")

    if "tool" not in data or "cloaca" not in data.get("tool", {}):
        raise ValueError("Missing [tool.cloaca] section in pyproject.toml")

    cloaca_cfg = data["tool"]["cloaca"]
    if "entry_module" not in cloaca_cfg:
        raise ValueError("Missing tool.cloaca.entry_module in pyproject.toml")

    return data
