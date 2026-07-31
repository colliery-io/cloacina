"""
CLOACI-T-0872 — provider wave front door.

`angreal providers check` — classify every provider against crates.io and run
the candidate guards locally (what the wave will enforce).

`angreal providers wave` — pre-flight a wave: classify, guard all candidates,
preview the compat outcome, then print the exact tag commands. The tag push
itself stays a HUMAN step (same convention as core's `release bump`).
"""

import subprocess
import sys
from datetime import timezone, datetime
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
SCRIPT = PROJECT_ROOT / "scripts" / "provider_wave.py"

providers = angreal.command_group(name="providers", about="first-party provider wave tooling (T-0872)")


def _run(*args) -> int:
    return subprocess.run([sys.executable, str(SCRIPT), *args], cwd=PROJECT_ROOT).returncode


@providers()
@angreal.command(name="check", about="classify providers vs crates.io + run candidate guards")
def check():
    rc = _run("classify")
    out = subprocess.run(
        [sys.executable, str(SCRIPT), "classify", "--json"],
        cwd=PROJECT_ROOT, capture_output=True, text=True,
    )
    import json
    worst = rc
    for row in json.loads(out.stdout or "[]"):
        if row["candidate"]:
            worst = max(worst, _run("guard", row["dir"]))
    return worst


@providers()
@angreal.command(name="wave", about="pre-flight a wave and print the tag commands (tag push stays human)")
def wave():
    print("== classify ==")
    if _run("classify") != 0:
        return 1
    out = subprocess.run(
        [sys.executable, str(SCRIPT), "classify", "--json"],
        cwd=PROJECT_ROOT, capture_output=True, text=True,
    )
    import json
    rows = json.loads(out.stdout or "[]")
    candidates = [r for r in rows if r["candidate"]]
    print("\n== guards (candidates) ==")
    failed = 0
    for r in candidates:
        failed = max(failed, _run("guard", r["dir"]))
    if failed:
        print("\nguards FAILED — fix the above before cutting a wave", file=sys.stderr)
        return 1

    wave_id = datetime.now(timezone.utc).strftime("%Y.%m")
    tag = f"providers-v{wave_id}"
    print("\n== ready ==")
    print(f"candidates: {[r['name'] for r in candidates] or 'none (re-certify-only wave)'}")
    print("\nCut the wave (human step):")
    print(f"  git tag {tag} && git push origin {tag}")
    print(f"(already used {tag}? bump with a .N suffix: {tag}.2)")
    return 0
