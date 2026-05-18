---
title: "Python Environment Variables"
description: "Environment variables visible from the Python `cloaca` module — runner configuration, registry variables, and Python-specific knobs."
weight: 10
---

# Python Environment Variables

<!-- TODO(DOC-G Phase 5): full content deferred. The audit found that the existing `python/api-reference/configuration.md` claims `CLOACA_*` env vars exist but they were not verified against `crates/cloacina-python/src/bindings/`. Audit before publishing. Sources to verify: -->
<!--   - `crates/cloacina-python/src/lib.rs` -->
<!--   - `crates/cloacina-python/src/bindings/context.rs` -->
<!--   - `crates/cloacina-python/src/bindings/runner.rs` -->

This page documents environment variables that affect Python workflows. The Python runtime inherits the full [Rust environment variable surface]({{< ref "/platform/reference/environment-variables" >}}); additional Python-specific knobs are listed here.

## Inherited from Rust

The Python runner (`cloaca.DefaultRunner`) reads the same environment variables as the Rust `DefaultRunner` — DSN, log level, registry-variable namespace, multi-tenant search path. See the [Rust environment variables reference]({{< ref "/platform/reference/environment-variables" >}}) for the full inventory.

## Python-specific (TBD)

This section is intentionally empty pending verification. The earlier `python/api-reference/configuration.md` referenced a `CLOACA_*` prefix; whether such variables are observed by the Python module separately from `CLOACINA_*` is not yet documented in code. Treat any `CLOACA_*` reference as aspirational until this section is filled.

## Registry variables

`CLOACINA_VAR_*` env vars are exposed verbatim to Python tasks through the registry variable API — see the [Variable Registry how-to]({{< ref "/workflows/how-to-guides/variable-registry" >}}).

## See also

- [Rust · Environment Variables]({{< ref "/platform/reference/environment-variables" >}}).
- [Python · API Reference · Configuration]({{< ref "/python/api-reference/configuration" >}}).
