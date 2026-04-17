-- Package lifecycle: content-addressed idempotency + one active row per name.
--
-- content_hash is SHA256 of the uploaded archive; superseded flags rows replaced
-- by a newer upload for the same package name. The partial unique index enforces
-- "one active row per name" — old row gets flagged superseded, new row is inserted
-- with the same name. The existing UNIQUE(package_name, version) stays as a
-- defense-in-depth check against version re-use under the monotonic-version policy.

ALTER TABLE workflow_packages ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
ALTER TABLE workflow_packages ADD COLUMN superseded INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX idx_active_package_per_name
    ON workflow_packages(package_name)
    WHERE superseded = 0;
