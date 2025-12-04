-- Revert storage_type column addition
-- WARNING: This will fail if any rows have storage_type != 'database'
-- since the FK constraint requires registry_id to exist in workflow_registry

-- Drop the index
DROP INDEX IF EXISTS idx_workflow_packages_storage_type;

-- Remove the storage_type column
ALTER TABLE workflow_packages DROP COLUMN storage_type;

-- Restore the foreign key constraint
ALTER TABLE workflow_packages
    ADD CONSTRAINT workflow_packages_registry_id_fkey
    FOREIGN KEY (registry_id) REFERENCES workflow_registry(id) ON DELETE CASCADE;
