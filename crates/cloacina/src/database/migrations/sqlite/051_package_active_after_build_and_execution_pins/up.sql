-- CLOACINA-T-1018: a package version is active only after its build succeeds,
-- and an execution is pinned to the version it started with. See the sibling
-- postgres migration 051 for the reasons.
DROP INDEX IF EXISTS idx_active_package_per_name;
CREATE UNIQUE INDEX idx_active_package_per_name
    ON workflow_packages(package_name)
    WHERE superseded = 0 AND build_status = 'success';

-- UUID as BLOB, TIMESTAMP as TEXT (RFC3339).
CREATE TABLE workflow_execution_package_pins (
    workflow_execution_id BLOB NOT NULL,
    package_name TEXT NOT NULL,
    package_id BLOB NOT NULL,
    version TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (workflow_execution_id, package_name)
);
