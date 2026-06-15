---
title: "Python Environment Variables"
description: "Environment variables visible from the Python `cloaca` module — runner configuration, registry variables, and Python-specific knobs."
weight: 10
---

# Python Environment Variables

This page documents environment variables that affect Python workflows. The Python runtime inherits the full [Rust environment variable surface]({{< ref "/reference/environment-variables" >}}).

## Inherited from Rust

The Python runner (`cloaca.DefaultRunner`) reads the same environment variables as the Rust `DefaultRunner` — DSN, log level, registry-variable namespace, multi-tenant search path. See the [Rust environment variables reference]({{< ref "/reference/environment-variables" >}}) for the full inventory.

## No separate `CLOACA_*` prefix

There is no distinct `CLOACA_*` environment-variable namespace. The Python module
reads the same variables as the Rust runtime; the only Cloacina-defined
convention is `CLOACINA_VAR_*` (the variable registry, below).

## Registry variables (`CLOACINA_VAR_*`)

The variable registry resolves named connections, secrets, and config from
`CLOACINA_VAR_<NAME>` environment variables at runtime. From Python:

```python
import cloaca

broker = cloaca.var("KAFKA_BROKER")              # reads CLOACINA_VAR_KAFKA_BROKER; raises if unset
threshold = cloaca.var_or("MODEL_THRESHOLD", "0.5")  # reads CLOACINA_VAR_MODEL_THRESHOLD, else "0.5"
```

```bash
export CLOACINA_VAR_KAFKA_BROKER=localhost:9092
export CLOACINA_VAR_MODEL_THRESHOLD=0.85
```

See the [Variable Registry how-to]({{< ref "/workflows/how-to-guides/variable-registry" >}}) for details.

## See also

- [Rust · Environment Variables]({{< ref "/reference/environment-variables" >}}).
- [Python · API Reference · Configuration]({{< ref "/reference/python-api/configuration" >}}).
