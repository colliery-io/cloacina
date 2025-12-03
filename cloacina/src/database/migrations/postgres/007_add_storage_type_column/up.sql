-- Add storage_type column to workflow_packages for runtime storage backend selection
-- This allows registry_id to reference either a database row or a filesystem path
--
-- storage_type values:
--   'database' - binary stored in workflow_registry table (default for backward compatibility)
--   'filesystem' - binary stored on disk at {storage_dir}/{registry_id}.so

-- Drop the foreign key constraint since registry_id can now reference filesystem storage
ALTER TABLE workflow_packages DROP CONSTRAINT workflow_packages_registry_id_fkey;

-- Add storage_type column with default 'database' for existing rows
ALTER TABLE workflow_packages ADD COLUMN storage_type VARCHAR(20) NOT NULL DEFAULT 'database';

-- Add index for filtering by storage type
CREATE INDEX idx_workflow_packages_storage_type ON workflow_packages(storage_type);
