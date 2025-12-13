-- Revert pause support from pipeline_executions
-- WARNING: This will fail if any rows have status = 'Paused'

-- Drop the paused index
DROP INDEX IF EXISTS pipeline_executions_paused_idx;

-- Remove pause metadata columns
ALTER TABLE pipeline_executions DROP COLUMN IF EXISTS pause_reason;
ALTER TABLE pipeline_executions DROP COLUMN IF EXISTS paused_at;

-- Restore original status CHECK constraint (without 'Paused')
ALTER TABLE pipeline_executions DROP CONSTRAINT pipeline_executions_status_check;
ALTER TABLE pipeline_executions ADD CONSTRAINT pipeline_executions_status_check
    CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled'));
