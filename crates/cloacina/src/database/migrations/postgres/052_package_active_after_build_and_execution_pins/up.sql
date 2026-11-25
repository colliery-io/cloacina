-- CLOACINA-T-1018: a package version is active only after its build succeeds,
-- and an execution is pinned to the version it started with.
--
-- 1. "Active" is now `NOT superseded AND build_status = 'success'`. An upload
--    no longer retires the active row: the new row waits as a candidate
--    (pending / building / failed, not superseded) and the build success swaps
--    the two in one transaction. The unique index follows that definition, so
--    an active row and a candidate of one name can exist together.
--    Safe on a database with rows: the new index covers a subset of the rows
--    the old one covered.
DROP INDEX IF EXISTS idx_active_package_per_name;
CREATE UNIQUE INDEX idx_active_package_per_name
    ON workflow_packages(package_name)
    WHERE NOT superseded AND build_status = 'success';

-- 2. The version an execution runs on, per package. Written at the first
--    dispatch of a task of the execution, read at each later one. No foreign
--    key: the pin must outlive a superseded or deleted package row long enough
--    to report it, and a pin without its execution is harmless.
CREATE TABLE workflow_execution_package_pins (
    workflow_execution_id UUID NOT NULL,
    package_name VARCHAR(255) NOT NULL,
    package_id UUID NOT NULL,
    version VARCHAR(100) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workflow_execution_id, package_name)
);
