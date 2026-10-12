#!/usr/bin/env python3
"""
Publish-tier completeness guard (CLOACI-T-0917).

The crates.io publish tiers in .github/workflows/unified_release.yml are a
hand-maintained, dependency-ordered list of `publish <crate>` shell calls.
The 0.10.0 postmortem (8961abb5) was exactly this list drifting from the
workspace: two crates (cloacina-constructor-contract, cloacina-agent) were
born during the cycle but never added, so the whole release train failed at
tag time. This check makes that drift a PR-time failure instead.

Assertions:
  1. Every workspace crate with `publish != false` appears in the tier list
     exactly once (missing => a future release will fail to resolve deps;
     duplicated => ambiguous ordering / copy-paste error).
  2. Every crate named in the tier list is a publishable workspace member
     (a stale entry => the release train publishes a ghost or fails).

Scope: only the root workspace (crates/*). Provider crates (providers/*) are
NOT workspace members and release through the provider wave workflow
(provider_release.yml), not unified_release.yml — they are out of scope here.

Stdlib-only; needs `cargo` on PATH. Run: python3 scripts/check_publish_tiers.py
"""

import json
import re
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = PROJECT_ROOT / ".github" / "workflows" / "unified_release.yml"

# A bare `publish <crate>` call inside the publish-cargo job's run script.
# Deliberately does NOT match the `publish() {` function definition (parens)
# or `cargo publish -p "$crate"` (extra tokens).
_PUBLISH_CALL = re.compile(r"^\s*publish\s+([A-Za-z0-9_-]+)\s*$", re.MULTILINE)


def tier_crates() -> list[str]:
    text = WORKFLOW.read_text()
    crates = _PUBLISH_CALL.findall(text)
    if not crates:
        raise SystemExit(
            f"check_publish_tiers: found no `publish <crate>` calls in {WORKFLOW} — "
            "the publish-cargo job's shape changed; update this parser."
        )
    return crates


def workspace_publishable() -> set[str]:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=PROJECT_ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    meta = json.loads(out)
    # `publish` is null when unrestricted, [] when `publish = false`,
    # or a registry list when restricted.
    return {p["name"] for p in meta["packages"] if p["publish"] != []}


def main() -> int:
    tiers = tier_crates()
    workspace = workspace_publishable()

    errors = []
    seen: dict[str, int] = {}
    for c in tiers:
        seen[c] = seen.get(c, 0) + 1
    for c, n in seen.items():
        if n > 1:
            errors.append(f"'{c}' appears {n} times in the publish tiers (must be exactly once)")
    for c in sorted(workspace - set(tiers)):
        errors.append(
            f"workspace crate '{c}' has publish != false but is MISSING from the "
            f"publish tiers in {WORKFLOW.name} — the release train will fail (0.10.0 postmortem class)"
        )
    for c in sorted(set(tiers) - workspace):
        errors.append(
            f"publish tier entry '{c}' is not a publishable workspace crate — "
            "stale tier entry (crate removed/renamed or publish = false)"
        )

    if errors:
        print("publish-tier completeness FAILED:", file=sys.stderr)
        for e in errors:
            print(f"  ✗ {e}", file=sys.stderr)
        print(
            "\nFix the tier list in .github/workflows/unified_release.yml "
            "(keep dependency order) or the crate's `publish` field.",
            file=sys.stderr,
        )
        return 1

    print(
        f"publish-tiers OK — {len(tiers)} tier entries cover all "
        f"{len(workspace)} publishable workspace crates exactly once"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
