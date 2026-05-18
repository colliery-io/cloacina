---
title: "Package a Python Computation Graph"
description: "Build a `.cloacina` archive that contains a Python-authored computation graph plus its reactor and accumulators (CLOACI-I-0101 / CLOACI-I-0102)."
weight: 10
---

# Package a Python Computation Graph

<!-- TODO(DOC-G Phase 5): full content deferred. Sources to read: -->
<!--   - `examples/features/computation-graphs/python-packaged-graph/` (runnable example) -->
<!--   - `crates/cloacina-python/src/loader.rs` (Python package loader) -->
<!--   - `crates/cloacina-workflow-plugin/src/types.rs:285-340` (9-method FFI vtable per I-0102) -->
<!--   - .metis archived I-0102 for the unified `cloacina::package!()` Python equivalent -->

Per CLOACI-I-0101, a Python-authored computation graph can be packaged into a `.cloacina` archive and loaded by `cloacina-server` exactly like a Rust-authored package. Per CLOACI-I-0102, the FFI vtable is unified — the same 9 methods every package exposes.

This how-to is a placeholder pending verification against `examples/features/computation-graphs/python-packaged-graph/`. The runnable example demonstrates the end-to-end packaging flow:

```sh
angreal demos features python-packaged-graph
```

Once the recipe is verified, this page will cover:

1. Declaring the CG with `ComputationGraphBuilder` + `@cloaca.reactor`.
2. Writing the `package.toml` (Python `interface` + `interface_version`).
3. Building the archive with `maturin build` + the Cloacina packaging shell.
4. Uploading via `cloacinactl package upload`.
5. Verifying with `cloacinactl graph list`.

## See also

- [Rust · CG · Packaging]({{< ref "/computation-graphs/explanation/packaging" >}}) — the underlying packaging model is identical.
- [Packaging Python Workflows]({{< ref "/python/workflows/how-to-guides/packaging-python-workflows" >}}) — the workflow analog (already documented).
- **CLOACI-I-0101** — CG / reactor decouple + Python embedded form.
- **CLOACI-I-0102** — Unified `cloacina::package!()` shell.
