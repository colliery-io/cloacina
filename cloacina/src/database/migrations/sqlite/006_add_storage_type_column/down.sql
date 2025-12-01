-- Revert storage_type column addition
-- WARNING: This will lose any rows with storage_type != 'database'
-- since the FK constraint requires registry_id to exist in workflow_registry

-- Create table with FK constraint, without storage_type column
CREATE TABLE workflow_packages_old (
    id BLOB PRIMARY KEY NOT NULL,
    registry_id BLOB NOT NULL REFERENCES workflow_registry(id) ON DELETE CASCADE,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(package_name, version)
);

-- Copy data (only database storage type rows will work with FK)
INSERT INTO workflow_packages_old (id, registry_id, package_name, version, description, author, metadata, created_at, updated_at)
SELECT id, registry_id, package_name, version, description, author, metadata, created_at, updated_at
FROM workflow_packages
WHERE storage_type = 'database';

-- Drop new table
DROP TABLE workflow_packages;

-- Rename old table back
ALTER TABLE workflow_packages_old RENAME TO workflow_packages;

-- Recreate indexes
CREATE INDEX idx_workflow_packages_name_version ON workflow_packages(package_name, version);
CREATE INDEX idx_workflow_packages_registry_id ON workflow_packages(registry_id);
CREATE INDEX idx_workflow_packages_created_at ON workflow_packages(created_at DESC);
