-- CLOACINA-T-1130: a package version delete reads the pins of the version
-- (by package name and version) to see whether an unfinished execution runs
-- on it. Without this index each delete scans the pins table.
CREATE INDEX IF NOT EXISTS idx_execution_pins_package_version
    ON workflow_execution_package_pins (package_name, version);
