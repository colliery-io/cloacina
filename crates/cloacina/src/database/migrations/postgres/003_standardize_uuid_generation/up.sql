-- Standardize UUID generation to use gen_random_uuid()
-- This migration updates existing tables to use the built-in PostgreSQL 13+ function
-- instead of the uuid-ossp extension

-- Update contexts table
ALTER TABLE contexts
    ALTER COLUMN id SET DEFAULT gen_random_uuid();

-- Note: pipeline_executions and other tables already use gen_random_uuid()
-- so they don't need updating
