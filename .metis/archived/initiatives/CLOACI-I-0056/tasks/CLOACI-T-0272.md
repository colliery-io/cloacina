---
id: add-triggerdefinitionv2-to
level: task
title: "Add TriggerDefinitionV2 to ManifestV2 schema"
short_code: "CLOACI-T-0272"
created_at: 2026-03-28T02:16:55.865481+00:00
updated_at: 2026-03-28T04:03:50.711559+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: true

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Add TriggerDefinitionV2 to ManifestV2 schema

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Add a `TriggerDefinitionV2` struct and an optional `triggers` field to `ManifestV2` so that `.cloacina` packages can declare triggers alongside tasks.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `TriggerDefinitionV2` struct with fields: `name`, `trigger_type`, `workflow`, `poll_interval`, `allow_concurrent`, `config` (arbitrary JSON)
- [ ] `ManifestV2` gains `triggers: Vec<TriggerDefinitionV2>` (default empty, so existing packages without triggers still validate)
- [ ] Validation: trigger names unique within package, `workflow` references a valid task/workflow name in the manifest, `poll_interval` parses via `parse_duration_str`
- [ ] Serialization roundtrip tests pass (JSON)
- [ ] Existing manifest tests still pass (no triggers = empty vec)

## Implementation Notes

### Files to modify
- `crates/cloacina/src/packaging/manifest_v2.rs` — add `TriggerDefinitionV2`, add `triggers` field to `ManifestV2`, extend `validate()`

### Design
- `trigger_type` is a string discriminator (e.g. `"rust"`, `"python"`, `"webhook"`, `"http_poll"`, `"file_watch"` or any user-defined string). Not an enum — keep it open for custom types.
- `config` is `Option<serde_json::Value>` for trigger-specific configuration (e.g. URL for http_poll, path for file_watch)
- `poll_interval` stored as string (e.g. `"30s"`) to match existing `parse_duration_str` pattern

## Status Updates

**2026-03-27**: Implementation complete, awaiting test confirmation.

### Changes made:
1. **`crates/cloacina/src/packaging/manifest_v2.rs`**:
   - Added `TriggerDefinitionV2` struct with fields: `name`, `trigger_type`, `workflow`, `poll_interval`, `allow_concurrent`, `config`
   - Added `triggers: Vec<TriggerDefinitionV2>` to `ManifestV2` with `#[serde(default)]` for backward compat
   - Added 3 validation error variants: `DuplicateTriggerName`, `InvalidTriggerWorkflow`, `InvalidTriggerPollInterval`
   - Added validation: unique trigger names, workflow references package name or task IDs, poll_interval parses via `parse_duration_str`
   - Added `triggers: vec![]` to existing test helpers
   - Added 10 new tests: validates with triggers, no triggers still validates, duplicate name, invalid workflow, references task ID, invalid poll interval, poll interval variants, serialization roundtrip, no config, deserialization without triggers field

2. **`crates/cloacina/src/packaging/mod.rs`** — Added `TriggerDefinitionV2` to re-exports

3. **`crates/cloacina/src/registry/loader/python_loader.rs`** — Added `triggers: vec![]` to test helper

4. **`crates/cloacina/tests/integration/python_package.rs`** — Added `triggers: vec![]` to test helper

`angreal cloacina unit` — all 341 tests pass (0 failed).

**Bonus fix**: PyO3 `extension-module` feature was hardcoded in `Cargo.toml`, preventing test binary linking. Moved to an opt-in feature flag `extension-module = ["pyo3/extension-module"]` so tests link against libpython properly. This was a pre-existing bug introduced in `a85b6d0`.
