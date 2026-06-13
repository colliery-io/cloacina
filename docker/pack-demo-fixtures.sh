#!/bin/sh
#
# Stage + pack the demo workflow fixtures for the demo compose profile
# (CLOACI-I-0117 / T-0660, T-0664). Writes `.cloacina` archives to /out for the
# harness to upload. Three packaging shapes, all resolved against the baked
# /workspace so the cloacina-compiler can build them:
#
#   rust_ws  — Rust fixture whose Cargo.toml uses the __WORKSPACE__ placeholder
#              (demo-slow-rust, demo-fail-rust); rewrite → /workspace, pack.
#   rust_rel — Rust fixture whose Cargo.toml uses ../../../crates relative paths
#              (mixed-rust: reactor + accumulator + reactor-bound CG + trigger +
#              workflow); rewrite → /workspace/crates, pack.
#   python   — pure-Python package (no cargo): bzip2-tar the package.toml + module
#              tree (the compiler skips the build; the reconciler imports via PyO3).
set -eu

OUT="${OUT_DIR:-/out}"
WS="${WORKSPACE_DIR:-/workspace}"
HOME_DIR="${CL_HOME:-$WS/.cloacina}"
mkdir -p "$OUT"

pack_rust_ws() {
    fx="$1"; src="$WS/examples/fixtures/$fx"; staged="/tmp/staged-$fx"
    rm -rf "$staged"; mkdir -p "$staged/src"
    for rel in package.toml Cargo.toml build.rs src/lib.rs; do
        [ -f "$src/$rel" ] && sed "s|__WORKSPACE__|$WS|g" "$src/$rel" > "$staged/$rel"
    done
    echo "packing $fx (rust/ws) → $OUT/$fx.cloacina"
    cloacinactl --home "$HOME_DIR" package pack "$staged" --out "$OUT/$fx.cloacina"
}

pack_rust_rel() {
    fx="$1"; src="$WS/examples/fixtures/$fx"; staged="/tmp/staged-$fx"
    rm -rf "$staged"; mkdir -p "$staged/src"
    for rel in package.toml Cargo.toml build.rs src/lib.rs; do
        [ -f "$src/$rel" ] && sed "s|\.\./\.\./\.\./crates|$WS/crates|g" "$src/$rel" > "$staged/$rel"
    done
    echo "packing $fx (rust/rel) → $OUT/$fx.cloacina"
    cloacinactl --home "$HOME_DIR" package pack "$staged" --out "$OUT/$fx.cloacina"
}

# $1 = output archive name, $2 = source dir (package.toml + module tree), $3 = version
pack_python() {
    name="$1"; srcdir="$2"; ver="${3:-0.1.0}"; prefix="$name-$ver"
    stage="/tmp/pystage-$name"; rm -rf "$stage"; mkdir -p "$stage/$prefix"
    cp -R "$srcdir"/. "$stage/$prefix/"
    echo "packing $name (python) → $OUT/$name.cloacina"
    tar -cjf "$OUT/$name.cloacina" -C "$stage" "$prefix"
}

# --- Executions: plain Rust task workflows (completed / failed / in-flight) ---
pack_rust_ws demo-slow-rust
pack_rust_ws demo-fail-rust

# --- Cron trigger: a workflow fired on a schedule (Triggers view) ---
pack_rust_ws demo-cron-rust

# --- Reactor + accumulator + reactor-bound CG + trigger + workflow (Rust) ---
pack_rust_rel mixed-rust

# --- Kafka-sourced stream accumulator → reactor-bound CG (Rust) — CLOACI-T-0676 ---
pack_rust_ws demo-kafka-stream-rust

# --- Python: a task workflow + a reactor-bound computation graph ---
# Both carry their module tree under workflow/ — the reconciler's Python
# extraction requires it ("Missing workflow source directory" otherwise). The
# python-packaged-graph *example* puts its module at the top level, so it can't
# be packed as-is; demo-py-graph is the same CG re-laid-out under workflow/.
pack_python demo-py-workflow "$WS/examples/fixtures/demo-py-workflow" 0.1.0
pack_python demo-py-graph "$WS/examples/fixtures/demo-py-graph" 0.1.0

echo "demo fixtures packed to ${OUT}:"
ls -la "$OUT"
