ALTER TABLE workflow_packages DROP COLUMN IF EXISTS plugin_interface_version;
ALTER TABLE fleet_agents DROP COLUMN IF EXISTS plugin_interface_version;
ALTER TABLE fleet_agents DROP COLUMN IF EXISTS protocol_version;
ALTER TABLE fleet_agents DROP COLUMN IF EXISTS agent_version;
