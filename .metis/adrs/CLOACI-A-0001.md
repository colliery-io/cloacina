---
id: 001-runtime-database-backend-selection
level: adr
title: "Runtime Database Backend Selection via Diesel MultiConnection"
number: 1
short_code: "CLOACI-A-0001"
created_at: 2025-11-28T15:27:26.028725+00:00
updated_at: 2025-11-28T15:27:26.028725+00:00
decision_date:
decision_maker:
parent:
archived: false

tags:
  - "#adr"
  - "#phase/draft"


exit_criteria_met: false
strategy_id: NULL
initiative_id: NULL
---

# ADR-1: Runtime Database Backend Selection via Diesel MultiConnection

**Status:** Draft
**Supersedes:** Compile-time database backend selection (undocumented)
**Related Initiative:** CLOACI-I-0001

## Context

### Original Decision (Superseded)

The original architecture used **compile-time feature flags** to select between PostgreSQL and SQLite backends. This was a **deliberate decision** made early in the project based on the following reasoning:

1. **Performance**: Compile-time selection enables monomorphization and full inlining with zero runtime dispatch overhead
2. **Type safety**: Each backend could use its native types without runtime conversion overhead
3. **Simplicity**: Avoided the complexity of runtime polymorphism patterns in Rust

At the time, **we did not fully understand the operational and development overhead** this decision would impose:

- **Distribution burden**: Separate binaries required for each backend, complicating release pipelines and user installation
- **Developer experience**: Testing both backends requires full recompilation, slowing iteration cycles
- **Code duplication**: Maintaining near-identical DAL implementations for each backend (`postgres_dal/` and `sqlite_dal/`) increased maintenance burden and risk of divergence
- **Binding consumption**: Downstream consumers (Python bindings, CLI tools) must choose backend at compile time, limiting flexibility
- **CI complexity**: Separate test matrices for each backend configuration

### Current State

The codebase enforces mutual exclusivity via `compile_error!` macros in `cloacina/src/lib.rs:430-435`. This results in:

- 6+ duplicated DAL files per backend
- Separate schema definitions in `database/schema.rs`
- Separate migration directories
- Feature flag complexity propagating to all dependent crates

### Trigger for Revisiting

As we prepare for broader distribution and consider Python bindings, the rigidity of compile-time selection has become a significant impediment. The performance benefits (nanoseconds of dispatch overhead) are negligible compared to actual database I/O (milliseconds).

## Decision

Migrate from compile-time feature flags to **Diesel 2.0+ MultiConnection** pattern for runtime database backend selection.

```rust
#[derive(diesel::MultiConnection)]
pub enum AnyConnection {
    Postgres(PgConnection),
    Sqlite(SqliteConnection),
}
```

Backend selection will occur at runtime based on the connection string URL scheme:
- `postgres://...` or `postgresql://...` -> PostgreSQL backend
- `sqlite://...` or file path -> SQLite backend

## Alternatives Analysis

| Option | Pros | Cons | Risk Level | Implementation Cost |
|--------|------|------|------------|-------------------|
| **Diesel MultiConnection** | Minimal rewrite, Diesel expertise retained, type-safe | Requires Diesel 2.0+, some backend-specific matching needed | Low | Medium |
| SeaORM Migration | First-class runtime support, async-native | Complete rewrite, new query syntax, team learning curve | High | Very High |
| Trait Objects (`Box<dyn>`) | Maximum flexibility, plugin architecture | Lifetime issues with Diesel, complex error handling | Medium | High |
| `enum_dispatch` crate | Near-static performance | Requires DAL restructuring, additional dependency | Low | Medium-High |
| Status Quo (compile-time) | Zero overhead, proven stable | DX issues, distribution problems, code duplication | Low | None |

## Rationale

**Diesel MultiConnection is the optimal choice** because:

1. **Lowest migration cost**: The codebase already uses Diesel; `MultiConnection` is an incremental addition, not a rewrite
2. **Maintained type safety**: Diesel's derive macro generates type-safe dispatch code
3. **Proven pattern**: Used by projects like Plume and Rustodon that needed multi-database support
4. **Acceptable overhead**: Runtime dispatch adds ~25 nanoseconds per call; database operations take 1-100+ milliseconds. The overhead is unmeasurable in practice.
5. **Flexibility preserved**: Feature flags can still exclude backends for size-optimized single-backend builds

The original compile-time decision prioritized theoretical performance over practical usability. With hindsight and real-world usage patterns, the trade-off calculation has changed.

## Consequences

### Positive
- Single binary supports both PostgreSQL and SQLite backends
- Runtime backend selection via connection string (no recompilation)
- Unified DAL codebase eliminates duplicate maintenance
- Simplified distribution and installation for end users
- Binding consumers gain runtime flexibility
- Reduced CI complexity (single build, parameterized tests)

### Negative
- Slightly larger binary size when both backends included
- Minor runtime dispatch overhead (negligible for DB operations)
- Migration effort required (estimated: Large complexity per CLOACI-I-0001)
- Backend-specific queries require explicit `match` handling

### Neutral
- Separate migration directories retained (Diesel requirement)
- `UniversalUuid`, `UniversalTimestamp`, `UniversalBool` wrappers remain useful for type bridging
- Optional feature flags allow single-backend builds for size-sensitive deployments

## Lessons Learned

This decision supersedes an earlier undocumented architectural choice. Key takeaways:

1. **Document architectural decisions early**: The original compile-time decision was never formally recorded, making it harder to evaluate trade-offs later
2. **Consider operational overhead**: Performance micro-optimizations can impose macro-level costs in DX, distribution, and maintenance
3. **Revisit assumptions**: What seemed optimal at project inception may not hold as requirements evolve
4. **Measure before optimizing**: The nanosecond-level dispatch overhead was never measured against actual database latency
