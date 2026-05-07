---
id: t1-d-add-org-id-column-to-package
level: task
title: "T1 (D): Add org_id column to package_signatures on both backends"
short_code: "CLOACI-T-0566"
created_at: 2026-05-06T12:50:41.681429+00:00
updated_at: 2026-05-07T01:18:51.425548+00:00
parent: CLOACI-I-0103
blocked_by: []
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0103
---

# T1 (D): Add org_id column to package_signatures on both backends

## Context

Per CLOACI-I-0103 decision D4: add an `org_id` column to the `package_signatures` table so signature records are scoped to a trusted organization. This is the prerequisite for D1's verification logic (which compares the signature's `org_id` against the configured `verification_org_id`).

**Constraint**: project rule forbids DROP+CREATE migrations on SQLite. Use `ALTER TABLE ADD COLUMN`.

## What to do

- New migration: `crates/cloacina/migrations/postgres/...` — `ALTER TABLE package_signatures ADD COLUMN org_id UUID;` (nullable).
- New migration: `crates/cloacina/migrations/sqlite/...` — `ALTER TABLE package_signatures ADD COLUMN org_id TEXT;` (nullable).
- Update Diesel schema (`crates/cloacina/src/dal/schema*.rs` — wherever `package_signatures` is declared).
- Update DAL read/write functions for `package_signatures` to handle the new field. Existing rows return `None` for `org_id`.

## Acceptance

- New rows can persist `org_id` on both backends.
- Existing rows return `None` for `org_id` (NULL preserved).
- `angreal test integration` passes on both backends after the migration.
- Diesel schema compiles (`angreal check all-crates`).

## References

- Parent: CLOACI-I-0103 (D4)
- Project rule: SQLite migrations avoid DROP+CREATE.
- Existing `package_signatures` schema: `crates/cloacina/migrations/` and `crates/cloacina/src/dal/schema*.rs`

## Status Updates

*To be added during implementation*
