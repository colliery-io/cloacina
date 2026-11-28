-- CLOACINA-T-1074: the full metadata document of a built package.
--
-- A child process of the compiler opens the built library, or imports the
-- Python package, and writes one JSON document: tasks with their trigger
-- rules, retry policy, time limit and placement, triggers, reactors, graphs,
-- constructor nodes, declared params and secrets. The compiler stores it here.
--
-- metadata_document          the JSON document; NULL: not described yet.
-- metadata_document_version  the format version of the last describe, set
--                            also when that describe failed; NULL: never tried.
-- describe_error             why the last describe failed; NULL: it did not.

ALTER TABLE workflow_packages ADD COLUMN metadata_document TEXT;
ALTER TABLE workflow_packages ADD COLUMN metadata_document_version INTEGER;
ALTER TABLE workflow_packages ADD COLUMN describe_error TEXT;
