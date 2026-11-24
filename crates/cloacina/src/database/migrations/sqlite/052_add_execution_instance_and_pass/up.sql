-- COLLIERY-I-0635 / CLOACINA-T-1000: an execution names the workflow
-- instance whose fire started it, and the Sync pass it is a segment of.
-- Both stay NULL for an execution that is neither.
ALTER TABLE workflow_executions ADD COLUMN instance_name TEXT;
ALTER TABLE workflow_executions ADD COLUMN pass_id BLOB;
CREATE INDEX idx_workflow_executions_instance_name ON workflow_executions (instance_name);
CREATE INDEX idx_workflow_executions_pass_id ON workflow_executions (pass_id);
