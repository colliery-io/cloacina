---
id: t5-upload-handler-enqueue-content
level: task
title: "T5: Upload handler — enqueue + content-hash artifact reuse"
short_code: "CLOACI-T-0523"
created_at: 2026-04-18T01:50:00+00:00
updated_at: 2026-04-19T00:23:17.593880+00:00
parent: CLOACI-I-0097
blocked_by: [CLOACI-T-0519]
archived: false

tags:
  - "#task"
  - "#phase/completed"


exit_criteria_met: false
initiative_id: CLOACI-I-0097
---

# T5: Upload handler — enqueue + content-hash artifact reuse

## Parent Initiative

CLOACI-I-0097 — Compiler Service

## Objective

Teach the upload handler (today's `register_workflow` from T-0497) about the build queue. New rows are enqueued (`build_status = pending`). When the uploaded bytes match another row's `content_hash` and that row is already `success`, copy its `compiled_data` into the new row and mark it `success` directly, skipping the queue.

## Acceptance Criteria

## Acceptance Criteria

## Acceptance Criteria

- [ ] `supersede_and_insert` (from T-0497) picks up an optional `compiled_data` + `build_status` pair so a single transaction can create a row as either `pending` or pre-populated `success`.
- [ ] Upload flow (`register_workflow`):
  1. Compute `content_hash` of package bytes (already present).
  2. Look up the active row for `name` (already present).
  3. Same hash on active → idempotent no-op (already present).
  4. **New:** look for *any* row (including superseded ones) with the same `content_hash` and `build_status = 'success'`.
     - If found: insert the new row with `build_status='success'`, `compiled_data` copied from the matching row, `compiled_at=NOW()`. Skip the queue.
     - Else: insert with `build_status='pending'`, `compiled_data=NULL`. Compiler picks it up.
- [ ] New DAL helper `find_success_by_hash(hash: &str) -> Option<(Uuid, Vec<u8>)>` on `workflow_registry` DAL.
- [ ] Unit tests in `workflow_registry/database.rs`:
  - Reuse: upload A (builds), upload B with identical content → B gets `success` + same `compiled_data` without ever entering `pending`.
  - No reuse: upload A (builds), upload B with different content → B is `pending`.
  - Reuse only triggers when the matching row is `success`, not `pending` / `building` / `failed`.

## Implementation Notes

### find_success_by_hash query

Postgres:
```sql
SELECT id, compiled_data
FROM workflow_packages
WHERE content_hash = $1
  AND build_status = 'success'
  AND compiled_data IS NOT NULL
ORDER BY compiled_at DESC NULLS LAST
LIMIT 1;
```

Returns the most recently-built artifact for that hash.

### Transaction atomicity

The whole upload path (supersede → content-reuse lookup → insert) stays inside the existing transaction from T-0497 so concurrent identical uploads land on exactly one winning row plus reuse for the loser.

### Reconciler interplay

Reconciler (post T6) only picks up `build_status = success` rows, so:
- `pending` rows uploaded before the compiler comes up just wait.
- Reused `success` rows become immediately visible to the reconciler even if the compiler is down.

## Status Updates

*To be added during implementation*
