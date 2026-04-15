-- Rename pipeline_executions table and columns to workflow_executions.
-- Part of the pipeline-to-workflow terminology migration (CLOACI-I-0094).

ALTER TABLE pipeline_executions RENAME TO workflow_executions;
ALTER TABLE workflow_executions RENAME COLUMN pipeline_name TO workflow_name;
ALTER TABLE workflow_executions RENAME COLUMN pipeline_version TO workflow_version;

-- Rename pipeline_execution_id FK columns in related tables.
ALTER TABLE task_executions RENAME COLUMN pipeline_execution_id TO workflow_execution_id;
ALTER TABLE recovery_events RENAME COLUMN pipeline_execution_id TO workflow_execution_id;
ALTER TABLE execution_events RENAME COLUMN pipeline_execution_id TO workflow_execution_id;
ALTER TABLE task_execution_metadata RENAME COLUMN pipeline_execution_id TO workflow_execution_id;
ALTER TABLE schedule_executions RENAME COLUMN pipeline_execution_id TO workflow_execution_id;
