---
id: add-triggerdefinitionv2-to
level: task
title: "Add TriggerDefinitionV2 to ManifestV2 schema"
short_code: "CLOACI-T-0272"
created_at: 2026-03-28T02:16:55.865481+00:00
updated_at: 2026-03-28T02:16:55.865481+00:00
parent: CLOACI-I-0056
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/todo"


exit_criteria_met: false
initiative_id: CLOACI-I-0056
---

# Add TriggerDefinitionV2 to ManifestV2 schema

## Parent Initiative

[[CLOACI-I-0056]]

## Objective

Add a `TriggerDefinitionV2` struct and an optional `triggers` field to `ManifestV2` so that `.cloacina` packages can declare triggers alongside tasks.

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

*To be added during implementation*
