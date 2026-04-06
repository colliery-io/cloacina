-- SQLite does not support DROP COLUMN prior to 3.35.0
-- Recreate without tenant_id
CREATE TABLE workflow_packages_new (
    id BLOB NOT NULL PRIMARY KEY,
    registry_id BLOB NOT NULL REFERENCES workflow_registry(id),
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    metadata TEXT NOT NULL,
    storage_type TEXT NOT NULL DEFAULT 'database',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(package_name, version)
);
INSERT INTO workflow_packages_new SELECT id, registry_id, package_name, version, description, author, metadata, storage_type, created_at, updated_at FROM workflow_packages;
DROP TABLE workflow_packages;
ALTER TABLE workflow_packages_new RENAME TO workflow_packages;
