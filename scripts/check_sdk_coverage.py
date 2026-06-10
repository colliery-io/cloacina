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

"""SDK coverage rule (CLOACI-I-0113 / T-0648).

Every operation in the committed OpenAPI document must be reachable from
every SDK, and every documented delivery-WS message variant must appear
in every SDK's WS implementation. This is the static half of the
coverage rule; the live half is each SDK's contract suite.

Detection per SDK:
- TypeScript: the literal spec path appears in src/client.ts
  (openapi-fetch calls use spec-literal paths).
- Rust: the path skeleton (parameters wildcarded) appears among string
  literals in src/lib.rs.
- Python: the generated module for the operationId is imported by
  _client.py.

Reactor WS variants are documented (JSON Schemas) but not yet wrapped by
any SDK — they need a running graph fixture; tracked as follow-up, not
checked here.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DELIVERY_WS_VARIANTS = ["welcome", "push", "hello", "ack"]

# Impl + contract suite per SDK: the variant must be handled by the
# implementation or asserted by the live suite (typed impls reference
# variants as enum identifiers, hence the case-insensitive search).
WS_SOURCES = {
    "typescript": [
        ROOT / "clients/typescript/src/ws.ts",
        ROOT / "clients/typescript/test/contract.test.ts",
    ],
    "python": [
        ROOT / "clients/python/src/cloacina_client/_ws.py",
        ROOT / "clients/python/tests/test_contract.py",
    ],
    "rust": [
        ROOT / "crates/cloacina-client/src/ws.rs",
        ROOT / "crates/cloacina-client/tests/contract.rs",
    ],
}


def skeleton(path: str) -> str:
    """Wildcard the parameter segments: /v1/tenants/{tenant_id}/x → /v1/tenants/*/x."""
    return re.sub(r"\{[^}]+\}", "*", path)


def rust_skeletons(source: str) -> set[str]:
    out = set()
    for lit in re.findall(r'"([^"]*)"', source):
        if lit.startswith("/"):
            out.add(skeleton(lit.replace("{t}", "{x}")))
    return out


def main() -> int:
    spec = json.loads((ROOT / "docs/static/openapi.json").read_text())
    ts_src = (ROOT / "clients/typescript/src/client.ts").read_text()
    rust_src = (ROOT / "crates/cloacina-client/src/lib.rs").read_text()
    py_src = (ROOT / "clients/python/src/cloacina_client/_client.py").read_text()

    rust_paths = rust_skeletons(rust_src)

    failures = []
    operations = 0
    for path, methods in spec["paths"].items():
        for method, op in methods.items():
            operations += 1
            op_id = op.get("operationId", "")

            if path not in ts_src:
                failures.append(f"typescript missing {method.upper()} {path}")
            if skeleton(path) not in rust_paths:
                failures.append(f"rust missing {method.upper()} {path}")
            if not re.search(rf"\b{re.escape(op_id)}\b", py_src):
                failures.append(
                    f"python missing operation '{op_id}' ({method.upper()} {path})"
                )

    for sdk, ws_paths in WS_SOURCES.items():
        ws_src = "\n".join(p.read_text() for p in ws_paths).lower()
        for variant in DELIVERY_WS_VARIANTS:
            if variant not in ws_src:
                failures.append(f"{sdk} WS handling missing delivery variant '{variant}'")

    if failures:
        print("SDK COVERAGE RULE VIOLATIONS:")
        for f in failures:
            print(f"  ✗ {f}")
        return 1

    print(
        f"Coverage rule satisfied: {operations} spec operations reachable from all "
        f"3 SDKs; {len(DELIVERY_WS_VARIANTS)} delivery-WS variants present in all "
        f"3 WS implementations."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
