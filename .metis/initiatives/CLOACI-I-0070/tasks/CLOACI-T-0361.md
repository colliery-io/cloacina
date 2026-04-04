---
id: code-generator-and-compile-time
level: task
title: "Code generator and compile-time validation"
short_code: "CLOACI-T-0361"
created_at: 2026-04-04T19:51:02.449706+00:00
updated_at: 2026-04-04T19:51:02.449706+00:00
parent: CLOACI-I-0070
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0070
---

# Code generator and compile-time validation

## Objective

The core of the macro — take the Graph IR from T-0360, validate it, and emit the compiled async function as a `TokenStream`. This is the most complex task: it generates the nested match arms, wires cache deserialization, handles `Option<T>` branch short-circuiting, and `#[node(blocking)]` wrapping.

## Acceptance Criteria

- [ ] Code generator takes Graph IR and module `ItemFn` list, emits a compiled async function
- [ ] Linear edges generate sequential `let x = node(input).await;` calls
- [ ] Routing edges generate `match node(input).await { Variant(v) => ..., }` with nested arms
- [ ] `Option<T>` return on intermediate nodes generates branch short-circuit (early return for `None`)
- [ ] Fan-out generates multiple downstream calls from same output
- [ ] Fan-in assembles multiple upstream values as parameters to the receiving node
- [ ] `#[node(blocking)]` detected on functions, wrapped in `tokio::task::spawn_blocking(...).await`
- [ ] Cache deserialization at function entry — reads from `InputCache` with bincode (release) / JSON (debug)
- [ ] Terminal node outputs collected into `GraphResult::Completed`
- [ ] Completeness validation: orphan function in module → compile error with helpful message
- [ ] Completeness validation: dangling node reference in graph → compile error
- [ ] Enum variant coverage: every variant of a routing enum must appear in the graph declaration
- [ ] Type safety: where possible at macro-time, validate return types match downstream input types (some checks may need to defer to generated code + rustc)
- [ ] Generated function compiles and is callable

## Implementation Notes

Use `quote` for token stream generation. The key challenge is generating the nested match structure from a potentially deep graph — recursive code generation following the topological order.

Type checking at macro-time is limited — proc macros can see token streams but not resolved types. Some validation (like "does this enum variant's inner type match the downstream function's parameter type") will be enforced by the generated code failing to compile if types don't match, with a helpful `compile_error!` where we can detect mismatches.

### Dependencies
T-0359 (parser), T-0360 (Graph IR + topological sort)

## Status Updates

*To be added during implementation*
