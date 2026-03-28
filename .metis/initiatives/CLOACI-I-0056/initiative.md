---
id: packageable-trigger-trait-user
level: initiative
title: "Packageable Trigger Trait — User-Defined Triggers in Workflows"
short_code: "CLOACI-I-0056"
created_at: 2026-03-26T17:25:26.506653+00:00
updated_at: 2026-03-28T12:59:28.026889+00:00
parent: CLOACI-V-0001
blocked_by: []
archived: false

tags:
  - "#initiative"
  - "#phase/completed"


exit_criteria_met: false
estimated_complexity: M
initiative_id: packageable-trigger-trait-user
---

# Packageable Trigger Trait — User-Defined Triggers in Workflows

## Context

Extracted from I-0050. The existing `Trigger` trait allows defining triggers, but they aren't first-class packageable components in ManifestV2. Cron-based triggers already work but are handled specially rather than through a general packageable trait.

This initiative generalizes the trigger system so that:
- Any trigger (cron, webhook, poll, file watch, custom) implements the same packageable trait
- Triggers can be declared in ManifestV2 manifests and auto-register on package load
- Users can define custom triggers in Rust or Python and distribute them alongside workflows
- Built-in types (webhook, http_poll, file_watch) are reference implementations, not special cases

Building on prior work from I-0044. The archive branch had a working implementation of packaged triggers.

### Key Learnings from Prior Work
- `TriggerDefinitionV2` needs name, type, workflow, poll_interval, config fields
- Python triggers via `@cloaca.trigger` decorator work well
- Cron triggers should be just another implementation of the packageable trait

## Goals & Non-Goals

**Goals:**
- Any trigger that implements the `Trigger` trait should be packageable via ManifestV2
- `TriggerDefinitionV2` in ManifestV2 for declaring triggers
- Auto-registration on package load via reconciler
- All trigger types (webhook, HTTP poll, file watch, custom) are packageable — not a fixed set of built-ins
- Users can define custom triggers in Rust or Python and package them
- Python trigger support via `@cloaca.trigger` decorator (native Python in core is landed)
- Feature parity: packaged workflows should support everything the library API supports

**Non-Goals:**
- Refactoring cron triggers — cron is already first-class and works; leave it as-is
- Trigger REST API and daemon CLI commands (I-0049 server/daemon)
- Pipeline claiming (I-0055)
- Continuous scheduling (I-0053)

## Detailed Design

### Packageable Trigger Trait
- Generalize the existing `Trigger` trait so implementations can be discovered and loaded from packages
- `TriggerDefinitionV2` in ManifestV2 with fields: name, type, workflow, poll_interval, config
- Auto-registration on package load via the package manager
- Cron-based triggers refactored to implement the same trait

### Trigger Packageability
- Any type implementing the `Trigger` trait is automatically packageable — no fixed list of "built-in" types
- Webhook, HTTP poll, file watch, and custom triggers all follow the same path
- Cron triggers are left as-is (already first-class, patterns exist)

### Python Triggers
- `@cloaca.trigger` decorator for defining triggers in Python
- Evaluated in the embedded Python runtime (native Python in core is landed via T-0271)
- Same packaging and registration flow as Rust triggers

## Prior Art

Reference implementation on `archive/cloacina-server-week1`:
- Packaged triggers: commit `da82e1b` (feat: packaged triggers — manifest-declared, auto-registered)

## Alternatives Considered

- **Fixed set of built-in triggers only**: Rejected. Users need to define domain-specific triggers (e.g., message queue, database change detection) without forking core.
- **Separate trigger registry from package registry**: Rejected. Triggers should be co-packaged with the workflows they serve.

## Scope Decisions (from discovery)

- **Python triggers in scope** — native Python in core is landed (T-0271), so `@cloaca.trigger` is feasible now
- **All triggers packageable** — the principle is "if it implements the Trigger trait, it's packageable." Not a fixed set of built-ins.
- **Don't refactor cron** — cron already works as a first-class special case with existing patterns. Leave it alone.
- **Feature parity** — packaged workflows should have parity with all ways to define workflows via the library API

## Implementation Plan (revised after code review)

**Key finding:** The Trigger trait, TriggerScheduler, DAL (trigger_schedules + trigger_executions), Python `@cloaca.trigger` decorator, and `PythonTriggerWrapper` all exist. The gap is purely ManifestV2 schema + reconciler wiring.

1. **ManifestV2 trigger schema** — Add `TriggerDefinitionV2` and `triggers: Vec<TriggerDefinitionV2>` to `ManifestV2`, update validation
2. **Reconciler trigger registration** — Extend reconciler to pick up trigger defs from manifest, create `TriggerSchedule` records and register in global trigger registry (Rust cdylib + Python paths)
3. **Python trigger reconciliation** — Wire `drain_python_triggers()` into reconciler for Python packages with trigger definitions
4. **Integration tests** — Package with triggers → reconcile → trigger registered → fires workflow
5. **Example/demo** — Packaged trigger workflow example
