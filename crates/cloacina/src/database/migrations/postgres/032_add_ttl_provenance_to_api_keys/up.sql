-- CLOACI-T-0792: short-TTL minted keys (OIDC / local login).
-- `expires_at` bounds a minted key's lifetime (NULL = no expiry, the manual-key
-- default); `issued_via` records provenance (e.g. `oidc:<issuer>:<sub>` or
-- `local:<account_id>`, NULL for manually-created keys).
ALTER TABLE api_keys ADD COLUMN expires_at TIMESTAMPTZ NULL;
ALTER TABLE api_keys ADD COLUMN issued_via TEXT NULL;
