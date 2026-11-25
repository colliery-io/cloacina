DROP TABLE IF EXISTS workflow_execution_package_pins;

-- The old index allows one non-superseded row per name. Retire candidates of
-- a name that has an active row before it comes back.
UPDATE workflow_packages
SET superseded = TRUE
WHERE NOT superseded
  AND build_status <> 'success'
  AND package_name IN (
      SELECT package_name FROM workflow_packages
      WHERE NOT superseded AND build_status = 'success'
  );
-- Two candidates of one name with no active row: keep the newest.
UPDATE workflow_packages
SET superseded = TRUE
WHERE NOT superseded
  AND EXISTS (
      SELECT 1 FROM workflow_packages newer
      WHERE newer.package_name = workflow_packages.package_name
        AND NOT newer.superseded
        AND newer.id <> workflow_packages.id
        AND (newer.created_at > workflow_packages.created_at
             OR (newer.created_at = workflow_packages.created_at
                 AND newer.id > workflow_packages.id))
  );

DROP INDEX IF EXISTS idx_active_package_per_name;
CREATE UNIQUE INDEX idx_active_package_per_name
    ON workflow_packages(package_name)
    WHERE NOT superseded;
