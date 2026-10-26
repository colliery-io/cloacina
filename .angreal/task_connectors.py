"""`angreal connectors stage` — build the first-party Sync connectors and stage
them as provider packages for a local server or agent (COLLIERY-I-0635
decision 14 / CLOACINA-T-1009). The server and agent images do the same in a
build stage.
"""

import os
import subprocess
import sys
from pathlib import Path

import angreal  # type: ignore

PROJECT_ROOT = Path(angreal.get_root()).parent
SCRIPT = PROJECT_ROOT / "scripts" / "stage_connectors.py"

connectors = angreal.command_group(name="connectors", about="first-party Sync connectors")


@connectors()
@angreal.command(
    name="stage",
    about="build the Sync connectors to wasm and stage them as provider packages",
    when_to_use=["running Sync on a local server or agent", "after changing a connector"],
    when_not_to_use=["the server or agent images, which stage them in their build"],
)
@angreal.argument(
    name="out",
    long="out",
    required=False,
    help="the provider directory (default: $CLOACINA_PROVIDER_PATH, else ./providers)",
)
@angreal.argument(
    name="only",
    long="only",
    required=False,
    help="a comma-separated subset, e.g. postgres,rest",
)
def stage(out=None, only=None):
    target = Path(out or os.environ.get("CLOACINA_PROVIDER_PATH") or "providers").resolve()
    cmd = [sys.executable, str(SCRIPT), str(target)]
    if only:
        cmd += ["--only", only]
    result = subprocess.run(cmd, cwd=str(PROJECT_ROOT), check=False)
    if result.returncode == 0:
        print(f"\nStart the server or agent with CLOACINA_PROVIDER_PATH={target}")
    return result.returncode
