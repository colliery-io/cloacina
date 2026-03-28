---
id: python-trigger-reconciliation-via
level: task
title: "Python trigger reconciliation via drain_python_triggers"
short_code: "CLOACI-T-0274"
created_at: 2026-03-28T02:16:58.074890+00:00
updated_at: 2026-03-28T02:16:58.074890+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Python trigger reconciliation via drain_python_triggers

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Wire the Python trigger path into the reconciler. When a Python package with `trigger_type: "python"` triggers is loaded, the reconciler should import the Python module (which causes `@cloaca.trigger` decorators to fire), call `drain_python_triggers()`, wrap each in `PythonTriggerWrapper`, and register them.

## Acceptance Criteria

- [ ] Reconciler detects `trigger_type: "python"` in manifest trigger definitions
- [ ] Python module import triggers `@cloaca.trigger` decorator registration into `PYTHON_TRIGGER_REGISTRY`
- [ ] `drain_python_triggers()` is called after module import, each def wrapped via `PythonTriggerWrapper`
- [ ] Wrapped triggers registered in global trigger registry + `TriggerSchedule` DAL records created
- [ ] Trigger names from manifest match the names collected by the decorator (validation/warning on mismatch)

## Implementation Notes

### Files to modify
- `crates/cloacina/src/registry/reconciler/loading.rs` — Python trigger loading path
- `crates/cloacina/src/registry/loader/python_loader.rs` — may need to expose module import for trigger discovery

### Key existing code
- `crates/cloacina/src/python/trigger.rs` — `drain_python_triggers()`, `PythonTriggerWrapper`, `PythonTriggerDef`
- The Python loader already imports modules for task discovery; triggers follow the same pattern

### Depends on
- T-0272 (TriggerDefinitionV2 in ManifestV2)
- T-0273 (Reconciler trigger registration — general path)

## Status Updates

*To be added during implementation*
