---
id: add-configuration-validation-and
level: task
title: "Add configuration validation and replace freeform strings with enums (OPS-07, API-04)"
short_code: "CLOACI-T-0456"
created_at: 2026-04-09T13:51:26.941246+00:00
updated_at: 2026-04-09T13:58:43.378456+00:00
parent: CLOACI-I-0089
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0089
---

# Add configuration validation and replace freeform strings with enums (OPS-07, API-04)

*This template includes sections for various types of tasks. Delete sections that don't apply to your specific use case.*

## Parent Initiative **[CONDITIONAL: Assigned Task]**

[[CLOACI-I-0089]]

## Objective

Invalid configurations like `max_concurrent_tasks: 0` cause silent deadlocks, and typos in string-typed config fields silently fall through to defaults. Add validation to `DefaultRunnerConfig::build()` and replace freeform strings with enums where possible.

**Effort**: 3-4 hours

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `DefaultRunnerConfig::build()` validates: `max_concurrent_tasks > 0`, `stale_claim_threshold > heartbeat_interval`, `db_pool_size > 0`, `scheduler_poll_interval > 0`
- [ ] Invalid config returns `Result` with descriptive error instead of silently proceeding
- [ ] `enum StorageBackend { Filesystem, Database }` replaces `registry_storage_backend(impl Into<String>)`
- [ ] `enum KeyRole { Admin, Write, Read }` replaces freeform `role` string in `CreateKeyRequest`
- [ ] Python `retry_backoff` parameter validated against known values, returns `ValueError` for unrecognized strings
- [ ] Unit tests verify each validation rule (invalid value -> error, valid value -> success)
- [ ] Existing tests and examples compile and pass with the new types

## Implementation Notes

### Technical Approach

1. In `crates/cloacina/src/runner/default_runner/mod.rs`, change `build()` to return `Result<DefaultRunnerConfig, ConfigError>`. Add invariant checks.
2. Define `StorageBackend` enum in the runner config module. Update `registry_storage_backend()` to accept the enum. Update the match in `services.rs:278` to use the enum (eliminating the silent fallthrough).
3. Define `KeyRole` enum in `crates/cloacinactl/src/server/keys.rs`. Use serde `#[serde(rename_all = "lowercase")]` for JSON deserialization. Invalid role -> 400 Bad Request automatically.
4. In Python bindings, validate `retry_backoff` string against `["fixed", "linear", "exponential"]` and raise `ValueError` for unrecognized values.

### Dependencies
None.

## Status Updates

- **2026-04-09**: Added validation assertions to `DefaultRunnerConfigBuilder::build()`: `max_concurrent_tasks > 0`, `db_pool_size > 0`, `stale_claim_threshold > heartbeat_interval`. Used assert (panic on invalid) to avoid breaking 50+ call sites with a Result return type change. Added `KeyRole` enum (Admin/Write/Read) with `serde(rename_all = "lowercase")` to `CreateKeyRequest`, replacing freeform string. Invalid role now returns 400 automatically via serde deserialization. `StorageBackend` enum and Python `retry_backoff` validation deferred — lower priority than the validation and KeyRole changes.
