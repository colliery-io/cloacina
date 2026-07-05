---
title: "Constructors & Providers"
description: "Reusable, WASM-sandboxed, parameterized factories that produce configured Cloacina primitives."
weight: 45
---

# Constructors & Providers

A **constructor** is a reusable, parameterized factory for one of Cloacina's
primitives (task, trigger, accumulator, reactor). You author it once — a struct
plus a body — and consumers instantiate it with their own configuration and
capability grants. Constructors run **WASM-sandboxed**, so a consumer decides
exactly what the sandbox may reach (files, network, environment) via
default-closed `grants`.

A **provider** is a *suite* of constructors: one crate compiles to one WASM
component that may expose **N members**, indexed by a `provider.json`. A consumer
selects a member with `constructor = "<name>"` and references the provider with
`from = "<provider crate>"`.

## How it flows

Providers ride Cargo's dependency model — there is no separate provider registry
to operate:

1. **Author** a provider crate (`cloacina-provider-<name>`) with one or more
   `#[constructor]` members and one `constructor_provider!` declaration.
2. **Consume** it from a workflow: add the crate as an ordinary Cargo dependency
   (Rust) or declare it in `[metadata.providers]` (Python), then reference a
   member with `constructor!(from = "...", constructor = "...")` /
   `cloaca.constructor(...)`.
3. **Build**: the compiler discovers the reference, resolves the provider from
   the dependency graph, builds it to a `wasm32-wasip2` component, and **bundles
   it into the workflow package** — the deployed artifact is hermetic.
4. **Run**: the server (or an execution agent — fleet dispatch is transparent)
   unpacks the bundled provider at load, resolves the member, binds the
   consumer's config, and executes it inside a WASI sandbox scoped exactly to
   the consumer's grants.

{{< toc-tree >}}
