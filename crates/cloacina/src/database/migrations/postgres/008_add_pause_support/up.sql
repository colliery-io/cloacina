-- Add pause support to pipeline_executions
-- Allows workflows to be paused mid-execution and resumed later

-- Update the status CHECK constraint to include 'Paused'
ALTER TABLE pipeline_executions DROP CONSTRAINT pipeline_executions_status_check;
ALTER TABLE pipeline_executions ADD CONSTRAINT pipeline_executions_status_check
    CHECK (status IN ('Pending', 'Running', 'Completed', 'Failed', 'Cancelled', 'Paused'));

-- Add pause metadata columns
ALTER TABLE pipeline_executions ADD COLUMN paused_at TIMESTAMP;
ALTER TABLE pipeline_executions ADD COLUMN pause_reason TEXT;

-- Add index for efficiently finding paused pipelines
CREATE INDEX pipeline_executions_paused_idx ON pipeline_executions(status) WHERE status = 'Paused';
