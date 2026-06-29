---
id: operators-reusable-polyglot
level: initiative
title: "Operators — reusable polyglot configured-instance factories for cloacina primitives (task/trigger/accumulator/reactor)"
short_code: "CLOACI-I-0132"
created_at: 2026-06-28T23:53:34.203685+00:00
updated_at: 2026-06-28T23:53:34.203685+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: operators-reusable-polyglot
---

# Operators — reusable polyglot configured-instance factories for cloacina primitives Initiative

## Context

Cloacina users hand-author every **task**, **trigger**, **accumulator**, and **reactor** via the
`#[task]` / `#[trigger]` / `#[accumulator]` / `#[reactor]` macros — there is no reusable,
parameterized building-block layer (the "operators" Airflow's ecosystem is built on).

This initiative adds **operators**: reusable, parameterized, **polyglot factories** that, given
config, produce a configured instance of an *existing* cloacina primitive. The mechanism is
**fidius "configured instances"** (v0.5.0) — bind config once, call many; `#[plugin_impl(Trait,
config = C)]` authored identically across Rust / Python / WASM. Operators run as **WASM
components** (sandboxed, distributable, no host recompile) and are distributed as signed fidius
packages ("providers").

Scoped through a design discussion (key decisions below). It **supersedes most of CLOACI-I-0116**
(parameterized workflow instances): parameterization *is* the fidius configured-instance binding.

## Goals & Non-Goals

**Goals:**
- A reusable, parameterized **operator** layer: a factory that produces a configured cloacina
  primitive — **task, trigger, accumulator, or reactor**.
- Operators run as **WASM components** (sandboxed, distributable, no host recompile), **configured**
  via fidius (`load_wasm_configured`).
- Authored with the **existing Rust/Python macros** (which emit the operator contract); distributed
  as **fidius provider packages**.
- A seed **built-in library** (>=1 operator per primitive) + the instantiation/authoring ergonomics
  to wire an operator into a workflow.

**Non-Goals:**
- **CG-node operators** — deferred (future, only if there's demand).
- **Other-language authoring** (Go/JS/...) — deferred; Rust/Python first. The contract is *designed*
  to allow it later, but it's not built here.
- Not replacing the macros or hand-authoring — operators are an *additional*, reusable layer.

## Requirements

### Functional Requirements
- REQ-001: An operator declares its **parameters** (config schema), **which primitive** it produces
  (task | trigger | accumulator | reactor), and dependencies, via a contract the loader can read.
- REQ-002: Operators are authored with the existing macros (Rust/Python); the macro **emits the
  operator contract** (a fidius interface + manifest) — no new authoring language.
- REQ-003: Operators compile to **WASM components** and are loaded **configured**
  (`load_wasm_configured`) — config bound once at instantiation.
- REQ-004: Cloacina's loader reads an operator's contract and **registers the configured primitive**
  into the runtime (the dynamic analog of the macro).
- REQ-005: A workflow author **instantiates** an operator with config and wires it in (Rust + Python).
- REQ-006: Operators are **distributed as fidius packages** (signed, versioned) and loaded through
  the registry ("provider" packages).
- REQ-007: A **seed built-in library** ships >=1 operator per supported primitive.

### Non-Functional Requirements
- NFR-001: Operators execute **sandboxed** (WASM deny-all) — defense for third-party/provider code.
- NFR-002: **No host recompile** to add/run an operator (the cdylib pain) — operators load dynamically.
- NFR-003: The operator contract is **language-neutral** so non-Rust/Python authoring can be added
  later with no cloacina change.
- NFR-004: Operator **use** is identical regardless of the operator's authoring language (a workflow
  author sees only config).

## Architecture

### Overview — two front-ends, one contract, one loader
- **Authoring**: a Rust/Python operator authored via the macros -> the macro emits a **fidius interface
  + manifest** (params, primitive kind, deps) -> compiled to a **WASM component**, packaged as a fidius
  provider.
- **Loading**: cloacina's loader `load_wasm_configured(component, &config)` -> reads the operator's
  contract -> **registers the configured primitive** into the runtime registry. (This metadata->register
  path already exists for packaged workflows via the `CloacinaPlugin` interface, incl.
  `get_reactor_metadata`.)
- **Execution**: the runtime/scheduler invokes the configured WASM operator as its bound primitive —
  `execute()` for a task, `poll()` for a trigger, the accumulator event loop, the reactor firing.

### The factory mechanism (fidius configured instances, v0.5.0)
`#[plugin_impl(Trait, config = C)]` + a `configure(cfg)` constructor; the host binds config once via
`load_wasm_configured` and calls methods on the configured instance. `configure_in_process` (cdylib)
is **in-process only** — which is *why* operators are WASM: WASM/Python are the only **distributable +
configurable** fidius paths.

## Detailed Design

The detailed contract + loader + execution shapes are settled in Phase B, **gated on the Phase A
spike** (does the Rust->WASM-component + configured-load path actually work for a macro-authored
operator). Sketch:
- **Contract**: a fidius interface per primitive kind (`execute` / `poll` / accumulator / reactor) +
  a manifest section (cloacina-defined schema): operator name, version, **primitive kind**, **param
  schema** (reuse the I-0128 `InputSlot` descriptors), dependencies.
- **Macro emits the contract**: the existing `#[task]`/`#[trigger]`/`#[accumulator]`/`#[reactor]`
  macros (or a thin `operator`-flavored variant) generate the interface + manifest for a Rust/Python
  operator, the same way they already generate packaged-workflow metadata.
- **Loader + registry**: extend the package loader to `load_wasm_configured`, read the manifest, and
  register the configured primitive into `Runtime` (task / trigger / accumulator / reactor registries).
- **Execution**: bridge each primitive's runtime trait (e.g. `Task::execute`, `Trigger::poll`) to a
  call into the configured WASM instance.

## Alternatives Considered
- **In-process Rust operators (cdylib + `configure_in_process`)** — cheap + typed, but config binding
  is **in-process only**, so NOT distributable-polyglot. Rejected as the primary path; may survive as
  an embedded convenience.
- **Clone Airflow (action/sensor taxonomy + a new "sensor" concept)** — rejected; cloacina already has
  triggers (= sensors). Operators are factories for the *existing* primitives, not new concepts.
- **Build it in the Task/Workflow layer (extend CLOACI-I-0116)** — subsumed: the fidius
  configured-instance binding *is* the parameterization; no parallel param system.
- **CG-node operators first** — deferred; the four primitives are where the demand is.

## Design Decision (converged, 2026-06-28 — human check-in)
- Operators = **factories** for cloacina's primitives; **start with task / trigger / accumulator /
  reactor** (CG nodes deferred).
- **WASM execution substrate** ("better operationally no matter what") — sandboxed, distributable, no
  host recompile.
- **Authored via the existing Rust/Python macros**; the macro **emits the contract**; **other-language
  authoring deferred**.
- The enabler is a **fidius upgrade 0.2.1 -> 0.5.4** (configured instances), done first.

## Implementation Plan

**Phase A — Enabler + de-risk**
- Fidius upgrade `0.2.1 -> 0.5.4` (the enabler).
- WASM-operator spike — Rust->WASM-component + configured-load (the linchpin; gates Phase B).

**Phase B — Operator framework**
- Operator contract + manifest schema (the macros emit it).
- Operator loader + registry (`load_wasm_configured` -> read contract -> register the primitive).
- Configured WASM execution in the runtime/scheduler (invoke as task/trigger/accumulator/reactor).

**Phase C — Library + ergonomics + distribution**
- Seed built-in operators (>=1 per primitive).
- Authoring + instantiation surface (Rust + Python).
- Distribution as fidius provider packages + the registry load path.

## Child Tasks
(Decomposed into the CLOACI-T-08xx tasks parented to this initiative — see below.)
