ALTER TABLE api_keys ADD COLUMN tenant_id TEXT NULL;
ALTER TABLE api_keys ADD COLUMN is_admin BOOLEAN NOT NULL DEFAULT FALSE;

-- Mark existing bootstrap key as admin
UPDATE api_keys SET is_admin = TRUE WHERE name = 'bootstrap-admin';
