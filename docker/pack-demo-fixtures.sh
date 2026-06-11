#!/bin/sh
#
# Stage + pack the demo workflow fixtures for the demo compose profile
# (CLOACI-I-0117 / T-0660). Rewrites the `__WORKSPACE__` placeholder to the
# baked /workspace so the packed source resolves against this image's (and the
# cloacina-compiler image's) workspace. Writes `.cloacina` archives to /out.
set -eu

OUT="${OUT_DIR:-/out}"
mkdir -p "$OUT"

for fx in demo-slow-rust demo-fail-rust; do
    src="/workspace/examples/fixtures/${fx}"
    staged="/tmp/staged-${fx}"
    rm -rf "$staged"
    mkdir -p "$staged/src"
    for rel in package.toml Cargo.toml build.rs src/lib.rs; do
        sed 's|__WORKSPACE__|/workspace|g' "$src/$rel" > "$staged/$rel"
    done
    echo "packing ${fx} → ${OUT}/${fx}.cloacina"
    cloacinactl --home /workspace/.cloacina package pack "$staged" --out "${OUT}/${fx}.cloacina"
done

echo "demo fixtures packed to ${OUT}:"
ls -la "$OUT"
