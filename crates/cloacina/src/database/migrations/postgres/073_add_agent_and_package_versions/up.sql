-- CLOACINA-T-1086: versions of agents and packages.
-- fleet_agents: what an agent reported at register: its release, its
-- protocol number and the plugin interface version of the Rust packages it
-- loads. NULL on a row of an agent from before the fields.
-- workflow_packages: the plugin interface version of a Rust build, written
-- by the compiler that built it. NULL for Python packages and for builds from
-- before the column; dispatch sends those as before.
-- Nullable columns only. Migration 073: 069 to 072 are taken by other open
-- work.
ALTER TABLE fleet_agents ADD COLUMN agent_version TEXT;
ALTER TABLE fleet_agents ADD COLUMN protocol_version INTEGER;
ALTER TABLE fleet_agents ADD COLUMN plugin_interface_version INTEGER;
ALTER TABLE workflow_packages ADD COLUMN plugin_interface_version INTEGER;
