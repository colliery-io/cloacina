-- Add org_id column to package_signatures (per CLOACI-I-0103 decision D4 / task T-0566).
-- Scopes signature records to a trusted organization so the server-side
-- verification gate (T-0568) can compare against the configured
-- verification_org_id without a join.
--
-- Nullable: existing rows pre-date this column. When --require-signatures
-- is on, NULL = untrusted (won't pass verification); operators must re-sign
-- existing packages as part of the upgrade path.

ALTER TABLE package_signatures ADD COLUMN org_id BLOB;

CREATE INDEX idx_signatures_org ON package_signatures(org_id);
