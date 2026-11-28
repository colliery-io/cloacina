-- CLOACINA-T-1085: a Rust package across a plugin interface change.
-- interface_checked_for: the plugin interface version a compiler (or the
-- in-process reconciler) checked the row for, with its one rebuild from kept
-- source. A row is checked once per interface version, across restarts and
-- replicas.
-- needs_new_build: why the package needs a new publish, and the step to take:
-- its rebuild from kept source still had another plugin interface, or failed.
-- NULL when the package needs nothing.
-- Nullable columns only.
ALTER TABLE workflow_packages ADD COLUMN interface_checked_for INTEGER;
ALTER TABLE workflow_packages ADD COLUMN needs_new_build TEXT;
