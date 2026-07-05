---
title: "Seed Providers"
description: "The built-in provider library — one canonical constructor per primitive kind."
weight: 40
---

# Seed Providers

Cloacina ships a small library of canonical providers — one per primitive kind —
under `examples/constructor-contract/`. They are real, runnable building blocks
and double as the reference implementations for authoring each kind.

| Provider | Kind | Member(s) | Config | What it does |
| --- | --- | --- | --- | --- |
| `cloacina-provider-fs` | task | `read_file`, `write_file` | `path` | Reads/writes a file through the sandbox. `write_file` takes its `contents` as a required runtime param from the upstream context. |
| `cloacina-provider-sensor` | trigger | `file_present` | `path` | The classic file sensor: fires when the path exists **inside the sandbox**, skips otherwise. Without an `fs` grant the path is invisible — the sensor fails closed by never firing. |
| `cloacina-provider-extract` | accumulator | `extract` | `field` | Projects the configured field out of each event into the boundary; events without it buffer. The everyday map/filter from an event stream into boundaries. |
| `cloacina-provider-quorum` | reactor | `quorum` | `required` | Fires the graph when at least `required` boundaries are held — N-of-M firing criteria. `required = 1` is "fire on anything". |

Example — the fs task member in a workflow:

```rust,ignore
constructor!(
    id = "reader",
    from = "cloacina-provider-fs@0.1.0",
    constructor = "read_file",
    config = { path = "/etc/hostname" },
    grants = { fs = ["ro:/etc"] },
);
```

## Consumption surface per kind

- **task** — `constructor!(...)` in a `#[workflow]` (Rust) or
  `cloaca.constructor(...)` (Python).
- **reactor** — `#[reactor(from = "...", constructor = "...", config(...))]`
  replaces the reactor's firing criteria with the WASM member's `evaluate`.
- **trigger / accumulator** — loaded through the embedded runtime registration
  API (`load_constructor` lands the member in the matching `Runtime` registry);
  declarative macro surfaces for these kinds are a follow-on.

## A note on state

The authoring model rebuilds the member instance from its bound config **per
call**, so seed accumulators are stateless transforms. Cross-event state
(count/time windows) needs runtime-held state and is a planned follow-on — if
you need windowing today, author a native `#[accumulator]`.
