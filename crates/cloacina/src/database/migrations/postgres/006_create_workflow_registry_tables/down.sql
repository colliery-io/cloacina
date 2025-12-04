-- Rollback workflow registry tables for PostgreSQL
--
-- This migration removes the workflow registry system tables:
-- - workflow_packages (metadata)
-- - workflow_registry (binary storage)

-- Drop indexes first
DROP INDEX IF EXISTS idx_workflow_registry_created_at;
DROP INDEX IF EXISTS idx_workflow_packages_created_at;
DROP INDEX IF EXISTS idx_workflow_packages_registry_id;
DROP INDEX IF EXISTS idx_workflow_packages_name_version;

-- Drop tables (workflow_packages first due to foreign key constraint)
DROP TABLE IF EXISTS workflow_packages;
DROP TABLE IF EXISTS workflow_registry;
