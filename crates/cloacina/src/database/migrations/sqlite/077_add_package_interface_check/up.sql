-- CLOACINA-T-1085: as Postgres 077.
ALTER TABLE workflow_packages ADD COLUMN interface_checked_for INTEGER;
ALTER TABLE workflow_packages ADD COLUMN needs_new_build TEXT;
