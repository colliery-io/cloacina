---
id: scheduling-features-triggers
level: initiative
title: "Scheduling Features — Triggers, Python Support, Pipeline Claiming"
short_code: "CLOACI-I-0050"
created_at: 2026-03-26T05:35:13.982223+00:00
updated_at: 2026-03-26T05:35:13.982223+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/discovery"


exit_criteria_met: false
estimated_complexity: L
initiative_id: scheduling-features-triggers
---

# Scheduling Features — Triggers, Python Support, Pipeline Claiming Initiative

## Context

Three major scheduling features need to be added to cloacina, building on prior work from initiatives I-0044, I-0038, and I-0034.

1. **Packaged Triggers (I-0044)** — Triggers declared in ManifestV2 manifests, auto-registered on package load. Built-in types: WebhookTrigger, HttpPollTrigger, FileWatchTrigger. Python trigger support via `@cloaca.trigger` decorator. REST endpoints for trigger management. This was fully implemented and working in the archive branch.

2. **Native Python Workflow Support (I-0038)** — Move PyO3 bindings from the cloaca bindings crate into cloacina core. Python workflows run natively. Also requires `cloacina-build` crate for downstream binaries (PyO3 rpath doesn't propagate across crate boundaries on macOS).

3. **Pipeline Claiming (I-0034)** — Horizontal scaling via pipeline claiming. Multiple runner instances can claim and execute pipelines without conflicts.

### Key Learnings from Prior Work
- PyO3 rpath needs a helper crate (`cloacina-build`) — downstream binaries crash without it on macOS
- `cloacina-build` should be a one-liner: `cloacina_build::configure()` in build.rs
- `pyo3-build-config` needs `resolve-config` feature
- `TriggerDefinitionV2` needs name, type, workflow, poll_interval, config fields
- Python is a core component, not behind a feature flag

## Goals & Non-Goals

**Goals:**
- Packaged triggers via ManifestV2
- Built-in trigger types (webhook, http_poll, file_watch)
- Python trigger support (`@cloaca.trigger` decorator)
- Trigger REST API and daemon CLI commands (`cloacinactl daemon trigger list/enable/disable`)
- Native Python workflows in core (PyO3 in cloacina, not in bindings)
- `cloacina-build` crate for downstream rpath resolution
- Pipeline claiming for horizontal scaling
- All examples compile and run with `cloacina-build`

**Non-Goals:**
- Continuous scheduling (separate initiative)
- Performance benchmarking (covered by I-0045/I-0046)

## Detailed Design

### Packaged Triggers
- `TriggerDefinitionV2` in ManifestV2 with fields: name, type, workflow, poll_interval, config
- Auto-registration on package load via the package manager
- Built-in trigger implementations: `WebhookTrigger` (HTTP callback), `HttpPollTrigger` (interval-based HTTP polling), `FileWatchTrigger` (filesystem notify)
- Python triggers via `@cloaca.trigger` decorator, evaluated in the embedded Python runtime

### Native Python Support
- Move PyO3 bindings from `crates/cloaca-bindings` into `crates/cloacina` core
- `cloacina-build` crate publishes rpath config so downstream binary crates link correctly on macOS
- Usage: add `cloacina-build` as a build dependency, call `cloacina_build::configure()` in build.rs
- `pyo3-build-config` with `resolve-config` feature for correct Python discovery

### Pipeline Claiming
- Claim-based execution model: runners atomically claim pipelines before executing
- SQLite/Postgres DAL support for claim records with expiry
- Prevents duplicate execution across multiple runner instances

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Packaged triggers: commit `da82e1b` (feat: packaged triggers — manifest-declared, auto-registered)
- Python/PyO3 in core: commit `a412e0c` (feat: native Python workflow support)
- cloacina-build crate: commit `5aaaa21` (feat: add cloacina-build crate)
- Pipeline claiming: `archive/main-pre-reset` commit `ee32916`

Key learnings:
- PyO3 rpath doesn't propagate across crate boundaries — needs `cloacina-build` helper
- `pyo3-build-config` needs `resolve-config` feature
- Python is a core component, not feature-gated
- TriggerDefinitionV2 fields: name, type (webhook/http_poll/file_watch/python), workflow, poll_interval, config

## Alternatives Considered

- **Keep PyO3 in bindings crate**: Rejected because it forces all downstream consumers to solve rpath themselves and prevents Python from being a first-class citizen in core.
- **Feature-flag Python support**: Rejected because Python is a core requirement for the trigger decorator system and workflow authoring.
- **Simple lock-based pipeline execution**: Rejected in favor of claim-based model which handles runner crashes gracefully via claim expiry.

## Acceptance Criteria

- Triggers declared in manifest auto-register on package load
- Built-in trigger types work (webhook, poll, file watch)
- Python triggers via `@cloaca.trigger` decorator work
- `cloacinactl daemon trigger list/enable/disable` commands work
- PyO3 lives in cloacina core, not in bindings
- `cloacina-build` crate published, all examples use it
- Pipeline claiming works across multiple runner instances
- All tests pass

## Implementation Plan

1. **Phase 1 — Native Python**: Move PyO3 into core, create `cloacina-build`, update all examples
2. **Phase 2 — Packaged Triggers**: Implement TriggerDefinitionV2, built-in types, REST API, CLI commands
3. **Phase 3 — Python Triggers**: `@cloaca.trigger` decorator, integration with trigger system
4. **Phase 4 — Pipeline Claiming**: Claim model in DAL, runner integration, multi-instance testing
