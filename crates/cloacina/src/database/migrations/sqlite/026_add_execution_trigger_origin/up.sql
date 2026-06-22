-- CLOACI-T-0776: how a workflow execution was triggered, so the UI can mark
-- manual operator runs. NULL = pre-migration / origin unknown. Set to 'manual'
-- by the REST execute endpoint; left NULL for cron/trigger/reactor-driven runs.
ALTER TABLE workflow_executions ADD COLUMN trigger_origin TEXT;
