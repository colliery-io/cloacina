DROP INDEX IF EXISTS idx_oidc_sessions_login_id;
ALTER TABLE oidc_sessions DROP COLUMN IF EXISTS checked_at;
ALTER TABLE oidc_sessions DROP COLUMN IF EXISTS claims;
ALTER TABLE oidc_sessions DROP COLUMN IF EXISTS login_id;
