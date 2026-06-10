#!/usr/bin/env python3
# Copyright 2026 Cloacina Contributors
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

"""SDK version lockstep check (CLOACI-I-0113 / REQ-008).

SDK vX.Y.Z is built, tested, and published against server vX.Y.Z — no
independent SDK cadence. This asserts every SDK package version (and the
committed OpenAPI document) matches the workspace version. Run by
`angreal test sdk-contract` and the release verify-version job.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text()
    in_pkg = False
    for line in text.splitlines():
        if line.strip() == "[workspace.package]":
            in_pkg = True
            continue
        if in_pkg and line.startswith("["):
            break
        if in_pkg:
            m = re.match(r'version\s*=\s*"([^"]+)"', line.strip())
            if m:
                return m.group(1)
    raise SystemExit("workspace.package.version not found in Cargo.toml")


def main() -> int:
    expected = workspace_version()
    failures = []

    ts = json.loads((ROOT / "clients/typescript/package.json").read_text())
    if ts["version"] != expected:
        failures.append(f"clients/typescript/package.json: {ts['version']}")

    py = (ROOT / "clients/python/pyproject.toml").read_text()
    m = re.search(r'^version\s*=\s*"([^"]+)"', py, re.MULTILINE)
    if not m or m.group(1) != expected:
        failures.append(
            f"clients/python/pyproject.toml: {m.group(1) if m else 'missing'}"
        )

    py_init = (ROOT / "clients/python/src/cloacina_client/__init__.py").read_text()
    m = re.search(r'__version__\s*=\s*"([^"]+)"', py_init)
    if not m or m.group(1) != expected:
        failures.append(
            f"cloacina_client.__version__: {m.group(1) if m else 'missing'}"
        )

    spec = json.loads((ROOT / "docs/static/openapi.json").read_text())
    if spec["info"]["version"] != expected:
        failures.append(f"docs/static/openapi.json info.version: {spec['info']['version']}")

    if failures:
        print(f"SDK VERSION DRIFT — workspace is {expected}, but:")
        for f in failures:
            print(f"  ✗ {f}")
        print("Lockstep policy (REQ-008): bump every SDK with the workspace version.")
        return 1

    print(f"SDK versions in lockstep at {expected}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
