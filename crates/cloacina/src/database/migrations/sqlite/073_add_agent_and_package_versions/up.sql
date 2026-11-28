-- CLOACINA-T-1086: versions of agents and packages, as Postgres 073.
ALTER TABLE fleet_agents ADD COLUMN agent_version TEXT;
ALTER TABLE fleet_agents ADD COLUMN protocol_version INTEGER;
ALTER TABLE fleet_agents ADD COLUMN plugin_interface_version INTEGER;
ALTER TABLE workflow_packages ADD COLUMN plugin_interface_version INTEGER;
