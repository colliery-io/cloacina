-- Revert workflow_executions back to pipeline_executions.

ALTER TABLE workflow_executions RENAME TO pipeline_executions;
ALTER TABLE pipeline_executions RENAME COLUMN workflow_name TO pipeline_name;
ALTER TABLE pipeline_executions RENAME COLUMN workflow_version TO pipeline_version;

ALTER TABLE task_executions RENAME COLUMN workflow_execution_id TO pipeline_execution_id;
ALTER TABLE recovery_events RENAME COLUMN workflow_execution_id TO pipeline_execution_id;
ALTER TABLE execution_events RENAME COLUMN workflow_execution_id TO pipeline_execution_id;
ALTER TABLE task_execution_metadata RENAME COLUMN workflow_execution_id TO pipeline_execution_id;
ALTER TABLE schedule_executions RENAME COLUMN workflow_execution_id TO pipeline_execution_id;
