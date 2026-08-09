#!/usr/bin/env python3
"""Python-SDK generated-code drift gate (CLOACI-T-0899 follow-on / SDK pin).

The TypeScript client has had `npm run check:generated` in CI since T-0645, so
its committed types cannot drift from the spec. The Python client had no
equivalent: its generator version lived only in a README sentence, nothing
enforced it, and the committed output silently fell behind.

That is not hypothetical. Regenerating with the version the README already
specified rewrote ~100 committed model files (`from_dict(cls: type[T]) -> T`
becoming `-> Self`, plus a `typing_extensions` import that was not even a
declared dependency). The committed tree had been produced by a different
generator build than the documented pin, and nothing noticed.

This script closes that hole the same way spec-check does: regenerate into a
temp dir with the PINNED generator and diff against what is committed.

Run:  python3 scripts/check_python_sdk_generated.py
"""

from __future__ import annotations

import filecmp
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CLIENT = ROOT / "clients" / "python"
GENERATED = CLIENT / "src" / "cloacina_client" / "_generated"
SPEC = ROOT / "docs" / "static" / "openapi.json"
CONFIG = CLIENT / "generator-config.yaml"

# THE pin. Keep this in lockstep with clients/python/README.md's regeneration
# recipe — the README is documentation, this is the enforcement.
GENERATOR = "openapi-python-client@0.29.0"

# Artifacts the generator's own tooling drops next to the output (it shells out
# to ruff to format). They are never committed and are not part of the
# contract, so comparing them would make this gate fail on every clean tree.
IGNORED = {".ruff_cache", "__pycache__", ".mypy_cache", ".pytest_cache"}


def _fail(msg: str) -> int:
    print(f"PYTHON SDK DRIFT — {msg}", file=sys.stderr)
    print(
        "\nRegenerate with the pinned generator and commit the result:\n"
        f"  cd clients/python && uvx {GENERATOR} generate \\\n"
        "    --path ../../docs/static/openapi.json \\\n"
        "    --config generator-config.yaml --meta none \\\n"
        "    --output-path src/cloacina_client/_generated --overwrite",
        file=sys.stderr,
    )
    return 1


def _diff_trees(a: Path, b: Path, rel: Path = Path(".")) -> list[str]:
    """Recursively compare two trees, returning human-readable differences.

    `filecmp.dircmp` is not used directly because its `left_only`/`right_only`
    reporting is shallow-by-default and we want exact per-file content
    comparison across the whole tree.
    """
    out: list[str] = []
    cmp = filecmp.dircmp(a, b, ignore=sorted(IGNORED))
    for name in sorted(cmp.left_only):
        out.append(f"  only in committed: {rel / name}")
    for name in sorted(cmp.right_only):
        out.append(f"  only in freshly generated: {rel / name}")
    # shallow=False forces content comparison, not just stat.
    _, mismatch, errors = filecmp.cmpfiles(
        a, b, cmp.common_files, shallow=False
    )
    for name in sorted(mismatch):
        out.append(f"  content differs: {rel / name}")
    for name in sorted(errors):
        out.append(f"  could not compare: {rel / name}")
    for name in sorted(cmp.common_dirs):
        out.extend(_diff_trees(a / name, b / name, rel / name))
    return out


def main() -> int:
    if not GENERATED.is_dir():
        return _fail(f"{GENERATED} does not exist")
    if not SPEC.is_file():
        return _fail(f"{SPEC} does not exist — emit the OpenAPI spec first")

    if shutil.which("uvx") is None:
        # Do not silently pass when the gate cannot run: a skipped check that
        # reports success is how the drift survived in the first place.
        print(
            "uvx not found — cannot run the pinned generator. Install uv, or "
            "skip this check explicitly.",
            file=sys.stderr,
        )
        return 2

    with tempfile.TemporaryDirectory(prefix="cloacina-pysdk-") as tmp:
        target = Path(tmp) / "_generated"
        proc = subprocess.run(
            [
                "uvx", GENERATOR, "generate",
                "--path", str(SPEC),
                "--config", str(CONFIG),
                "--meta", "none",
                "--output-path", str(target),
                "--overwrite",
            ],
            cwd=CLIENT,
            capture_output=True,
            text=True,
        )
        if proc.returncode != 0:
            print(proc.stdout, file=sys.stderr)
            print(proc.stderr, file=sys.stderr)
            return _fail(f"generator exited {proc.returncode}")

        differences = _diff_trees(GENERATED, target)
        if differences:
            print(
                "PYTHON SDK DRIFT — the committed _generated/ tree does not "
                f"match a fresh run of {GENERATOR}:\n",
                file=sys.stderr,
            )
            for line in differences[:40]:
                print(line, file=sys.stderr)
            if len(differences) > 40:
                print(
                    f"  … and {len(differences) - 40} more", file=sys.stderr
                )
            return _fail("committed output is stale")

    print(f"Python SDK generated code matches {GENERATOR}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
