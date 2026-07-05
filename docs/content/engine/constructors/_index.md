---
title: "Constructors & Providers"
description: "Reusable, WASM-sandboxed, parameterized factories that produce configured Cloacina primitives."
weight: 45
draft: true
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

{{< toc-tree >}}
