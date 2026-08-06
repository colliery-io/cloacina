DROP INDEX IF EXISTS idx_oidc_sessions_key_id;
CREATE INDEX idx_oidc_sessions_key_id ON oidc_sessions (key_id);
DELETE FROM oidc_sessions WHERE refresh_enc IS NULL;
ALTER TABLE oidc_sessions ALTER COLUMN refresh_enc SET NOT NULL;
