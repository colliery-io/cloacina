-- SQLite migration: Create workflow registry tables for packaged workflows
-- UUID stored as BLOB (16 bytes), TIMESTAMP stored as TEXT (RFC3339 format)

-- Simple key-value table for binary workflow storage
CREATE TABLE workflow_registry (
    id BLOB PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,                           -- RFC3339 format
    data BLOB NOT NULL
);

-- Rich metadata table with foreign key to registry
CREATE TABLE workflow_packages (
    id BLOB PRIMARY KEY NOT NULL,
    registry_id BLOB NOT NULL REFERENCES workflow_registry(id) ON DELETE CASCADE,
    package_name TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT,
    author TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',                -- Package metadata (tasks, schedules, etc.) as JSON
    created_at TEXT NOT NULL,                           -- RFC3339 format
    updated_at TEXT NOT NULL,                           -- RFC3339 format

    -- Ensure unique package name + version combinations
    UNIQUE(package_name, version)
);

-- Index for efficient package lookups
CREATE INDEX idx_workflow_packages_name_version
ON workflow_packages(package_name, version);

-- Index for foreign key performance
CREATE INDEX idx_workflow_packages_registry_id
ON workflow_packages(registry_id);

-- Index for time-based queries
CREATE INDEX idx_workflow_packages_created_at
ON workflow_packages(created_at DESC);

-- Index for registry cleanup operations
CREATE INDEX idx_workflow_registry_created_at
ON workflow_registry(created_at DESC);
