---
id: computation-graph-computation
level: specification
title: "Computation Graph & #[computation_graph] Macro"
short_code: "CLOACI-S-0006"
created_at: 2026-04-04T17:03:46.100807+00:00
updated_at: 2026-04-04T17:03:46.100807+00:00
parent: CLOACI-I-0069
blocked_by: []
archived: false

tags:
  - "#specification"
  - "#phase/discovery"


exit_criteria_met: false
initiative_id: NULL
---

# Computation Graph & #[computation_graph] Macro

## Overview

The `#[computation_graph]` macro takes a Rust module containing async functions (nodes) and a topology declaration, and compiles them into a single async function that the reactor calls. The topology is the single source of truth for the graph's structure. Nodes are pure async functions with no framework annotations or return types.

The macro handles at compile time: topology resolution, topological sort, enum variant routing via nested match arms, type validation across edges, completeness checks, blocking node wrapping, and terminal node detection.

### Relationship to other specs

- **CLOACI-S-0004** (Accumulator Trait) — accumulators feed the graph's entry nodes via the reactor's input cache
- **CLOACI-S-0005** (Reactor) — the reactor calls the compiled function and handles the result
- **CLOACI-I-0069** (parent initiative) — defines the overall architecture and how computation graphs fit into the process model

## Macro Structure

The `#[computation_graph]` attribute macro is applied to a Rust module. It has two parts:

1. **Header**: reaction criteria + topology declaration
2. **Body**: pure async functions (nodes)

```rust
#[computation_graph(
    react = when_any(alpha, beta, gamma),   // reaction criteria
    graph = {                                // topology declaration
        decision_engine(alpha, beta, gamma) => {
            Signal -> risk_check,
            NoAction -> audit_logger,
        },
        risk_check(gamma) => {
            Approved -> output_handler,
            Blocked -> alert_handler,
        },
    }
)]
mod market_maker {
    async fn decision_engine(...) -> Option<DecisionOutcome> { ... }
    async fn risk_check(...) -> RiskCheck { ... }
    async fn output_handler(...) -> OutputConfirmation { ... }
    async fn alert_handler(...) -> AlertRecord { ... }
    async fn audit_logger(...) -> AuditRecord { ... }
}
```

## Topology Declaration Syntax

The `graph = { }` block declares the full topology. Two edge types:

### Linear edge: `->`

Single output, no enum routing. The node returns a concrete type that flows directly to the downstream node.

```rust
graph = {
    enrich(alpha) -> validate,        // enrich returns EnrichedData, validate receives it
    validate -> output_handler,        // no inputs listed = receives from upstream only
}
```

### Routing edge: `=> { Variant -> downstream }`

The node returns an enum (or `Option<Enum>`). Each variant is mapped to a downstream node.

```rust
graph = {
    decision_engine(alpha, beta) => {
        Signal -> risk_check,
        NoAction -> audit_logger,
    },
}
```

### Node inputs

Parenthesized names after the node name are accumulator inputs from the reactor's cache. These are the graph's entry points — data that comes from accumulators, not from upstream nodes.

```rust
graph = {
    // decision_engine receives alpha, beta, gamma from the cache
    // AND receives nothing from upstream (it's an entry node)
    decision_engine(alpha, beta, gamma) => { ... },

    // risk_check receives gamma from the cache
    // AND receives Signal from decision_engine (upstream)
    risk_check(gamma) => { ... },

    // output_handler receives nothing from cache
    // AND receives Approved from risk_check (upstream)
    output_handler,
}
```

A node with no parenthesized inputs and no upstream edges is an error (orphan).

### Fan-in

Multiple edges pointing to the same node. The node's function signature must accept all incoming values:

```rust
graph = {
    validate_a(alpha) -> merge_node,
    validate_b(beta) -> merge_node,
    // merge_node receives ValidatedA + ValidatedB as parameters
}
```

### Fan-out

One node feeding multiple downstream nodes (no routing — all fire):

```rust
graph = {
    compute(alpha) -> output_handler,
    compute(alpha) -> audit_logger,
    // Both output_handler and audit_logger receive compute's output
}
```

### Terminal nodes

Nodes with no downstream edges. The macro identifies them automatically. Their outputs are collected into `GraphResult`.

## Node Functions

Nodes are pure async functions. No framework annotations (except optional `#[node(blocking)]`). No framework return types — they return standard Rust types.

### Entry nodes

Receive accumulator inputs from the cache. Parameters are `Option<&T>` for `when_any` (some inputs may not have fired yet) or `&T` for `when_all` (all inputs guaranteed present).

```rust
async fn decision_engine(
    alpha: Option<&AlphaData>,
    beta: Option<&BetaData>,
    gamma: Option<&ExposureData>,
) -> DecisionOutcome { ... }
```

### Intermediate nodes

Receive upstream output + optionally accumulator inputs from cache:

```rust
// Receives Signal from decision_engine + gamma from cache
async fn risk_check(signal: &Signal, gamma: Option<&ExposureData>) -> RiskCheck { ... }
```

### Terminal nodes

Same as any other node — they just don't have downstream edges:

```rust
async fn output_handler(signal: &Signal) -> OutputConfirmation {
    publish(signal).await;
    OutputConfirmation { ... }
}
```

### Conditional propagation

An intermediate node returning `Option<T>` provides conditional propagation within the graph. `None` means that branch doesn't continue downstream. The macro generates an early return for that branch in the compiled function.

This is **not** about whether the graph fires — the reactor already decided that via reaction criteria. This is about branches within the graph that may or may not continue based on computation results (e.g., a risk check that blocks a signal).

### Blocking nodes

`#[node(blocking)]` on the function tells the macro to wrap the call in `tokio::task::spawn_blocking`:

```rust
#[node(blocking)]
async fn heavy_computation(input: &Data) -> Result {
    // This runs on the blocking thread pool
    expensive_sync_operation(input)
}
```

The topology doesn't change. The macro generates `spawn_blocking(|| heavy_computation(input)).await` instead of `heavy_computation(input).await`.

## Compiled Output

The macro generates a single async function from the topology + nodes:

```rust
// Generated from the market_maker example
async fn market_maker_compiled(cache: &InputCache) -> GraphResult {
    // Entry node — deserialize inputs from cache
    let alpha = cache.get::<AlphaData>("alpha");
    let beta = cache.get::<BetaData>("beta");
    let gamma = cache.get::<ExposureData>("gamma");

    // decision_engine — routing node, always fires (reactor decided to execute)
    match decision_engine(alpha.as_deref(), beta.as_deref(), gamma.as_deref()).await {
        DecisionOutcome::Signal(signal) => {
            // risk_check — routing node, also reads gamma from cache
            match risk_check(&signal, gamma.as_deref()).await {
                RiskCheck::Approved(approved) => {
                    // terminal
                    let r = output_handler(&approved).await;
                    GraphResult::completed(vec![Box::new(r)])
                }
                RiskCheck::Blocked(alert) => {
                    // terminal
                    let r = alert_handler(&alert).await;
                    GraphResult::completed(vec![Box::new(r)])
                }
            }
        }
        DecisionOutcome::NoAction(reason) => {
            // terminal
            let r = audit_logger(&reason).await;
            GraphResult::completed(vec![Box::new(r)])
        }
    }
}
```

Key aspects of the generated code:
- Cache deserialization at the top (bincode in release, JSON in debug — same wire format as accumulators)
- Nested match arms from the topology's routing declarations
- `Option<T>` returns on intermediate nodes become early returns for that branch on `None` (the graph still completes — other branches may produce output)
- Blocking nodes wrapped in `spawn_blocking`
- Terminal node outputs collected into `GraphResult`
- Fan-in nodes receive multiple parameters from different upstream branches
- Fan-out nodes have their output passed to multiple downstream calls

## Compile-Time Validation

The macro enforces consistency between the topology declaration and the node functions:

### Completeness
- Every function in the module must appear in the topology (no orphan functions)
- Every node referenced in the topology must exist as a function in the module (no dangling references)

### Enum variant coverage
- Every variant of every routing enum must be wired to a downstream node
- If `DecisionOutcome` has variants `Signal` and `NoAction`, both must appear in the routing block
- Unhandled variants are a compile error

### Type safety
- Accumulator input names in the topology must match registered accumulator names
- Upstream node return types must match downstream node parameter types across edges
- For routing edges: the inner type of each enum variant must match the downstream node's input parameter type
- For `Option<T>` returns: `T` (or its variants if `T` is an enum) must match downstream expectations

### Signature validation
- Entry node parameters must be `Option<&T>` for `when_any` or `&T` for `when_all` (for cache inputs)
- Intermediate node parameters must accept the upstream output type by reference
- Fan-in nodes must have parameters for all incoming edges

## GraphResult

The compiled function returns a `GraphResult`:

```rust
enum GraphResult {
    /// Graph executed to completion. Contains terminal node outputs.
    Completed {
        outputs: Vec<Box<dyn Any + Send>>,
    },
    /// Graph execution failed.
    Error(GraphError),
}
```

The graph always runs to completion when called — the reactor already decided to fire. Intermediate branches may short-circuit via `Option<T>` (conditional propagation within the graph), but the graph itself always produces a `Completed` or `Error` result.

The reactor uses this to:
- **Completed**: persist cache to DAL, signal batch accumulators to flush, emit audit events
- **Error**: log error, emit metrics, clear dirty flags (next boundary triggers fresh execution)

## Constraints

### Technical Constraints

- The topology declaration is parsed at macro expansion time. Complex topologies (many nodes, deep nesting) increase compile time but have zero runtime cost.
- The macro must be an attribute proc macro on a module (`#[computation_graph] mod name { }`). Item-level macros can't see sibling functions.
- Enum variants in routing must be tuple variants with exactly one field (the value that flows downstream). Unit variants and struct variants are not supported in routing edges.
- `#[node(blocking)]` is the only framework annotation on node functions. Everything else is inferred from the topology and function signatures.
- The generated function is `async` — it can be `.await`ed by the reactor. Blocking nodes within it use `spawn_blocking` so they don't starve the tokio runtime.
- Recursive graphs (cycles in the topology) are a compile error. The topological sort detects them.
