-- Add storage_type column to workflow_packages for runtime storage backend selection
-- This allows registry_id to reference either a database row or a filesystem path
--
-- storage_type values:
--   'database' - binary stored in workflow_registry table (default for backward compatibility)
--   'filesystem' - binary stored on disk at {storage_dir}/{registry_id}.so
--
-- SQLite doesn't support DROP CONSTRAINT, so we recreate the table without the FK

-- Create new table without FK constraint, with storage_type column
CREATE TABLE workflow_packages_new (
    id BLOB PRIMARY KEY NOT NULL,
    registry_id BLOB NOT NULL,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    storage_type TEXT NOT NULL DEFAULT 'database',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(package_name, version)
);

-- Copy existing data (all existing rows are database storage type)
INSERT INTO workflow_packages_new (id, registry_id, package_name, version, description, author, metadata, storage_type, created_at, updated_at)
SELECT id, registry_id, package_name, version, description, author, metadata, 'database', created_at, updated_at
FROM workflow_packages;

-- Drop old table
DROP TABLE workflow_packages;

-- Rename new table
ALTER TABLE workflow_packages_new RENAME TO workflow_packages;

-- Recreate indexes
CREATE INDEX idx_workflow_packages_name_version ON workflow_packages(package_name, version);
CREATE INDEX idx_workflow_packages_registry_id ON workflow_packages(registry_id);
CREATE INDEX idx_workflow_packages_created_at ON workflow_packages(created_at DESC);
CREATE INDEX idx_workflow_packages_storage_type ON workflow_packages(storage_type);
