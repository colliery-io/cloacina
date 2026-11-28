-- CLOACINA-T-1082: an OIDC login session keeps the claims of its login, so
-- each refresh runs the current group map on them, and the IdP refresh token,
-- so the groups are checked at the IdP each hour.
--
-- * `login_id`: the rows of one login (one row per tenant the login maps to).
--   A group check updates every row of the login at once.
-- * `claims`: JSON of `sub`, `email`, `email_verified` and `groups`. NULL on a
--   row opened before this migration: such a row keeps its role until its
--   deadline.
-- * `checked_at`: when the IdP last answered for this login (the login itself
--   is the first answer).
-- * `refresh_enc` (already there) holds the IdP refresh token, AES-256-GCM
--   with associated data bound to `login_id`.
--
-- No sqlite twin: the auth tables are Postgres-only (see migration 046).
ALTER TABLE oidc_sessions ADD COLUMN login_id UUID NULL;
ALTER TABLE oidc_sessions ADD COLUMN claims TEXT NULL;
ALTER TABLE oidc_sessions ADD COLUMN checked_at TIMESTAMPTZ NULL;

CREATE INDEX idx_oidc_sessions_login_id ON oidc_sessions (login_id);
