-- CLOACINA-T-1047: the serialized size in bytes of the context a task
-- returned, written with the row when the context is saved. NULL on rows
-- written before this column existed.

ALTER TABLE task_execution_metadata ADD COLUMN context_size_bytes BIGINT;
