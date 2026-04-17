-- Package lifecycle: content-addressed idempotency + one active row per name.
--
-- See sibling sqlite migration 018 for rationale. UNIQUE(package_name, version)
-- stays as a defense-in-depth check against version re-use under the
-- monotonic-version policy; the partial unique index enforces the real invariant
-- (one active row per name).

ALTER TABLE workflow_packages ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE workflow_packages ADD COLUMN superseded BOOLEAN NOT NULL DEFAULT FALSE;

CREATE UNIQUE INDEX idx_active_package_per_name
    ON workflow_packages(package_name)
    WHERE NOT superseded;
