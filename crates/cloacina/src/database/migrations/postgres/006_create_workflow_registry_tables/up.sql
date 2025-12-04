-- Create workflow registry tables for PostgreSQL
--
-- This migration creates the two-table registry system:
-- 1. workflow_registry: Simple key-value storage for binary .so data
-- 2. workflow_packages: Rich metadata with foreign key to registry
--
-- This separation enables efficient metadata queries and future migration
-- to object storage for binary data.

-- Simple key-value table for binary workflow storage
CREATE TABLE workflow_registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    data BYTEA NOT NULL
);

-- Rich metadata table with foreign key to registry
CREATE TABLE workflow_packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    registry_id UUID NOT NULL REFERENCES workflow_registry(id) ON DELETE CASCADE,
    package_name VARCHAR(255) NOT NULL,
    version VARCHAR(100) NOT NULL,
    description TEXT,
    author VARCHAR(255),
    metadata TEXT NOT NULL DEFAULT '{}', -- Package metadata (tasks, schedules, etc.)
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Ensure unique package name + version combinations
    UNIQUE(package_name, version)
);

-- Indexes for efficient querying
CREATE INDEX idx_workflow_packages_name_version ON workflow_packages(package_name, version);
CREATE INDEX idx_workflow_packages_registry_id ON workflow_packages(registry_id);
CREATE INDEX idx_workflow_packages_created_at ON workflow_packages(created_at);
CREATE INDEX idx_workflow_registry_created_at ON workflow_registry(created_at);
