---
id: scheduling-features-triggers
level: initiative
title: "Native Python in Core — PyO3 Move, cloacina-build, Cloaca Removal"
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

# Native Python in Core — PyO3 Move, cloacina-build, Cloaca Removal

## Context

Move PyO3 bindings from the standalone `bindings/cloaca-backend` crate into cloacina core so Python workflows run natively. This also requires a `cloacina-build` helper crate for downstream binaries (PyO3 rpath doesn't propagate across crate boundaries on macOS). Once native Python is in core, `bindings/cloaca-backend` is removed.

Building on prior work from I-0038. The archive branch had a working implementation.

### Key Learnings from Prior Work
- PyO3 rpath needs a helper crate (`cloacina-build`) — downstream binaries crash without it on macOS
- `cloacina-build` should be a one-liner: `cloacina_build::configure()` in build.rs
- `pyo3-build-config` needs `resolve-config` feature
- Python is a core component, not behind a feature flag

## Goals & Non-Goals

**Goals:**
- Native Python workflows in core (PyO3 in cloacina, not in bindings)
- `cloacina-build` crate for downstream rpath resolution
- Remove `bindings/cloaca-backend` once native Python is in core
- All examples compile and run with `cloacina-build`

**Non-Goals:**
- Packageable trigger trait (separate initiative I-0056)
- Pipeline claiming / horizontal scaling (I-0055)
- Trigger REST API and daemon CLI commands (I-0049)
- Continuous scheduling (I-0053)
- Performance benchmarking (I-0054)

## Detailed Design

### Native Python Support
- Move PyO3 bindings from `bindings/cloaca-backend` into `crates/cloacina` core
- `cloacina-build` crate publishes rpath config so downstream binary crates link correctly on macOS
- Usage: add `cloacina-build` as a build dependency, call `cloacina_build::configure()` in build.rs
- `pyo3-build-config` with `resolve-config` feature for correct Python discovery

### Cloaca Removal
- Once native Python is in core, remove `bindings/cloaca-backend/`
- Update CI workflows (cloaca-matrix.yml) and angreal tasks
- Python users consume cloacina directly, no separate bindings crate

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

- PyO3 lives in cloacina core, not in bindings
- `cloacina-build` crate exists, all examples use it
- `bindings/cloaca-backend` removed
- CI updated (cloaca-matrix.yml removed or reworked)
- All tests pass

## Implementation Plan

1. **Phase 1 — Native Python**: Move PyO3 into core, create `cloacina-build`, update all examples
2. **Phase 2 — Cloaca Removal**: Remove `bindings/cloaca-backend`, update CI and angreal tasks
