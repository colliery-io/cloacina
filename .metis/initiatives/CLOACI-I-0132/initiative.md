---
id: constructors-reusable-polyglot
level: initiative
title: "Constructors — reusable polyglot configured-instance factories for cloacina primitives (task/trigger/accumulator/reactor)"
short_code: "CLOACI-I-0132"
created_at: 2026-06-28T23:53:34.203685+00:00
updated_at: 2026-07-05T02:29:13.556221+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: XL
initiative_id: constructors-reusable-polyglot
---

# Constructors — reusable polyglot configured-instance factories for cloacina primitives Initiative

## Context

Cloacina users hand-author every **task**, **trigger**, **accumulator**, and **reactor** via the
`#[task]` / `#[trigger]` / `#[accumulator]` / `#[reactor]` macros — there is no reusable,
parameterized building-block layer (the "constructors" Airflow's ecosystem is built on).

This initiative adds **constructors**: reusable, parameterized, **polyglot factories** that, given
config, produce a configured instance of an *existing* cloacina primitive. The mechanism is
**fidius "configured instances"** (v0.5.0) — bind config once, call many; `#[plugin_impl(Trait,
config = C)]` authored identically across Rust / Python / WASM. Constructors run as **WASM
components** (sandboxed, distributable, no host recompile) and are distributed as signed fidius
packages ("providers").

Scoped through a design discussion (key decisions below). It **supersedes most of CLOACI-I-0116**
(parameterized workflow instances): parameterization *is* the fidius configured-instance binding.

## Goals & Non-Goals

**Goals:**
- A reusable, parameterized **constructor** layer: a factory that produces a configured cloacina
  primitive — **task, trigger, accumulator, or reactor**.
- Constructors run as **WASM components** (sandboxed, distributable, no host recompile), **configured**
  via fidius (`load_wasm_configured`).
- Authored with the **existing Rust/Python macros** (which emit the constructor contract); distributed
  as **fidius provider packages**.
- A seed **built-in library** (>=1 constructor per primitive) + the instantiation/authoring ergonomics
  to wire an constructor into a workflow.

**Non-Goals:**
- **CG-node constructors** — deferred (future, only if there's demand).
- **Other-language authoring** (Go/JS/...) — deferred; Rust/Python first. The contract is *designed*
  to allow it later, but it's not built here.
- Not replacing the macros or hand-authoring — constructors are an *additional*, reusable layer.

## Requirements

### Functional Requirements
- REQ-001: An constructor declares its **parameters** (config schema), **which primitive** it produces
  (task | trigger | accumulator | reactor), and dependencies, via a contract the loader can read.
- REQ-002: Constructors are authored with the existing macros (Rust/Python); the macro **emits the
  constructor contract** (a fidius interface + manifest) — no new authoring language.
- REQ-003: Constructors compile to **WASM components** and are loaded **configured**
  (`load_wasm_configured`) — config bound once at instantiation.
- REQ-004: Cloacina's loader reads an constructor's contract and **registers the configured primitive**
  into the runtime (the dynamic analog of the macro).
- REQ-005: A workflow author **instantiates** an constructor with config and wires it in (Rust + Python).
- REQ-006: Constructors are **distributed as fidius packages** (signed, versioned) and loaded through
  the registry ("provider" packages).
- REQ-007: A **seed built-in library** ships >=1 constructor per supported primitive.

### Non-Functional Requirements
- NFR-001: Constructors execute **sandboxed** (WASM deny-all) — defense for third-party/provider code.
- NFR-002: **No host recompile** to add/run an constructor (the cdylib pain) — constructors load dynamically.
- NFR-003: The constructor contract is **language-neutral** so non-Rust/Python authoring can be added
  later with no cloacina change.
- NFR-004: Constructor **use** is identical regardless of the constructor's authoring language (a workflow
  author sees only config).

## Architecture

### Overview — two front-ends, one contract, one loader
- **Authoring**: a Rust/Python constructor authored via the macros -> the macro emits a **fidius interface
  + manifest** (params, primitive kind, deps) -> compiled to a **WASM component**, packaged as a fidius
  provider.
- **Loading**: cloacina's loader `load_wasm_configured(component, &config)` -> reads the constructor's
  contract -> **registers the configured primitive** into the runtime registry. (This metadata->register
  path already exists for packaged workflows via the `CloacinaPlugin` interface, incl.
  `get_reactor_metadata`.)
- **Execution**: the runtime/scheduler invokes the configured WASM constructor as its bound primitive —
  `execute()` for a task, `poll()` for a trigger, the accumulator event loop, the reactor firing.

### The factory mechanism (fidius configured instances, v0.5.0)
`#[plugin_impl(Trait, config = C)]` + a `configure(cfg)` constructor; the host binds config once via
`load_wasm_configured` and calls methods on the configured instance. `configure_in_process` (cdylib)
is **in-process only** — which is *why* constructors are WASM: WASM/Python are the only **distributable +
configurable** fidius paths.

## Detailed Design

The detailed contract + loader + execution shapes are settled in Phase B, **gated on the Phase A
spike** (does the Rust->WASM-component + configured-load path actually work for a macro-authored
constructor). Sketch:
- **Contract**: a fidius interface per primitive kind (`execute` / `poll` / accumulator / reactor) +
  a manifest section (cloacina-defined schema): constructor name, version, **primitive kind**, **param
  schema** (reuse the I-0128 `InputSlot` descriptors), dependencies.
- **Macro emits the contract**: the existing `#[task]`/`#[trigger]`/`#[accumulator]`/`#[reactor]`
  macros (or a thin `constructor`-flavored variant) generate the interface + manifest for a Rust/Python
  constructor, the same way they already generate packaged-workflow metadata.
- **Loader + registry**: extend the package loader to `load_wasm_configured`, read the manifest, and
  register the configured primitive into `Runtime` (task / trigger / accumulator / reactor registries).
- **Execution**: bridge each primitive's runtime trait (e.g. `Task::execute`, `Trigger::poll`) to a
  call into the configured WASM instance.

## Alternatives Considered
- **In-process Rust constructors (cdylib + `configure_in_process`)** — cheap + typed, but config binding
  is **in-process only**, so NOT distributable-polyglot. Rejected as the primary path; may survive as
  an embedded convenience.
- **Clone Airflow (action/sensor taxonomy + a new "sensor" concept)** — rejected; cloacina already has
  triggers (= sensors). Constructors are factories for the *existing* primitives, not new concepts.
- **Build it in the Task/Workflow layer (extend CLOACI-I-0116)** — subsumed: the fidius
  configured-instance binding *is* the parameterization; no parallel param system.
- **CG-node constructors first** — deferred; the four primitives are where the demand is.

## Design Decision (converged, 2026-06-28 — human check-in)
- Constructors = **factories** for cloacina's primitives; **start with task / trigger / accumulator /
  reactor** (CG nodes deferred).
- **WASM execution substrate** ("better operationally no matter what") — sandboxed, distributable, no
  host recompile.
- **Authored via the existing Rust/Python macros**; the macro **emits the contract**; **other-language
  authoring deferred**.
- The enabler is a **fidius upgrade 0.2.1 -> 0.5.4** (configured instances), done first.

## Implementation Plan

**Phase A — Enabler + de-risk**
- Fidius upgrade `0.2.1 -> 0.5.4` (the enabler).
- WASM-constructor spike — Rust->WASM-component + configured-load (the linchpin; gates Phase B).

**Phase B — Constructor framework**
- Constructor contract + manifest schema (the macros emit it).
- Constructor loader + registry (`load_wasm_configured` -> read contract -> register the primitive).
- Configured WASM execution in the runtime/scheduler (invoke as task/trigger/accumulator/reactor).

**Phase C — Library + ergonomics + distribution**
- Seed built-in constructors (>=1 per primitive).
- Authoring + instantiation surface (Rust + Python).
- Distribution as fidius provider packages + the registry load path.

## Child Tasks
(Decomposed into the CLOACI-T-08xx tasks parented to this initiative — see below.)

## Status (2026-07-04 — bookkeeping snapshot)
**COMPLETED (Phases A + B + most of C):** T-0820 (fidius upgrade) · T-0821 (spike) · T-0822 (contract) · T-0823 (loader) · T-0824 (runtime execution) · T-0826 (authoring) · T-0827 (provider packaging) · T-0828 (acc/reactor execution) · T-0829 (consumption surface) · T-0830 (reactor→CG scheduler) · **T-0832** (packaged support — Step 5b live) · **T-0834** (capability layer, A-0009/S-0014) · **T-0836** (build-side bundling, A-0010/S-0015) · **T-0837** (provider-as-suite, A-0011).
**Landmark:** the full packaged chain verified LIVE on the demo stack 7/7 — author → compile-time provider discovery+bundling → hermetic store → server resolution → grant-gated sandboxed execution (`constructor_demo` Completed). Both Rust (`constructor!`) and Python (`cloaca.constructor()`) consumer surfaces shipped. All on branch `feat/i0132-constructors` (~35 commits, in review).
**REMAINING:** T-0825 (seed built-in provider library) · T-0831 (packaged-Python live demo; embedded surface DONE) · T-0833 (semver version pinning) · T-0835 (post-ABI-bump recompile signal — freshly motivated by the live stale-artifact episode) · **T-0838** (fleet/agent constructor execution — live finding #4).
**ADRs decided:** A-0009 (capabilities) · A-0010 (Cargo distribution) · A-0011 (suites). Spec S-0015 drafting → should advance with T-0836 done.

## Status (2026-07-04 — INITIATIVE COMPLETE: the last five tasks landed on feat/i0132-completion)
- **T-0833** — `@version` pins ENFORCED at load (segment-prefix semantics matching the build side).
- **T-0825** — seed provider library: fs (task) · sensor/file_present (trigger, grant-gated) · extract (accumulator) · quorum (reactor); wasm lane test 3/3.
- **T-0835** — stale-artifact recompile signal: the reconciler flips ABI/interface-mismatched packages back to `pending` for the compiler to rebuild from retained source (self-healing; once-per-package-per-process guard).
- **T-0831** — packaged-Python LIVE: `demo-constructor-py` (`[metadata.providers]` + module-level `cloaca.constructor`) built, bundled, loaded, and **Completed on the fleet**.
- **T-0838** — agents execute constructor nodes: `GET /v1/agent/providers/{digest}` + agent-side Step-5b twin (Rust + Python paths, fail-closed both ways).

**FINAL LIVE PROOF (virgin stack, fresh images, STOCK fleet executor):** all 17 packages built clean; `constructor_demo` (Rust) AND `constructor_demo_py` both **Completed with execution on agents** — both final contexts carry the AGENT container's own hostname read inside the WASM sandbox through the `ro:/etc` grant. Every REQ (001–007) and NFR (001–004) is implemented, tested, and live-verified in both languages, embedded + server + fleet. All child tasks completed.
