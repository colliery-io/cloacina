/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

import { AppShell, Badge, Box, Group, NavLink, ScrollArea, Text, Tooltip, Button } from "@mantine/core";
import {
  IconActivity,
  IconGauge,
  IconHeartbeat,
  IconKey,
  IconClock,
  IconPackage,
  IconSettings,
  IconTopologyStar3,
} from "@tabler/icons-react";
import { NavLink as RouterNavLink, Outlet, useNavigate } from "react-router-dom";

import { useAuth } from "../auth/AuthContext";
import { useServerHealth } from "../api/operations";

/** At-a-glance server liveness dot in the header (CLOACI-I-0124 / WS-12). */
function ServerHealthDot() {
  const health = useServerHealth();
  const color = health.isPending
    ? "var(--mantine-color-gray-4)"
    : health.data?.alive
      ? "var(--mantine-color-green-6)"
      : "var(--mantine-color-red-6)";
  const label = health.isPending
    ? "Checking server…"
    : health.data?.alive
      ? health.data?.ready
        ? "Server alive & ready"
        : `Server alive, not ready${health.data?.reason ? `: ${health.data.reason}` : ""}`
      : "Server unreachable";
  return (
    <Tooltip label={label} withArrow>
      <span
        style={{
          width: 9,
          height: 9,
          borderRadius: "50%",
          background: color,
          display: "inline-block",
        }}
      />
    </Tooltip>
  );
}

const NAV = [
  { to: "/", label: "Overview", icon: IconGauge, end: true },
  { to: "/workflows", label: "Workflows", icon: IconPackage },
  { to: "/executions", label: "Executions", icon: IconActivity },
  { to: "/triggers", label: "Triggers", icon: IconClock },
  { to: "/graphs", label: "Graphs", icon: IconTopologyStar3 },
  { to: "/operations", label: "Operations", icon: IconHeartbeat },
  { to: "/keys", label: "API Keys", icon: IconKey },
  { to: "/settings", label: "Settings", icon: IconSettings },
];

/**
 * Authenticated shell (CLOACI-I-0117): left nav + a connection indicator +
 * disconnect, wrapping every in-app route.
 */
export function Shell() {
  const { connection, disconnect } = useAuth();
  const navigate = useNavigate();

  function handleDisconnect() {
    disconnect();
    navigate("/connect", { replace: true });
  }

  return (
    <AppShell header={{ height: 56 }} navbar={{ width: 240, breakpoint: "sm" }} padding="md">
      <AppShell.Header>
        <Group h="100%" px="md" justify="space-between">
          <Text fw={700}>Cloacina</Text>
          <Group gap="sm">
            {connection && <ServerHealthDot />}
            {connection && (
              <Badge variant="light" color="indigo">
                {connection.tenant} @ {connection.serverUrl}
              </Badge>
            )}
            <Button size="xs" variant="subtle" onClick={handleDisconnect}>
              Disconnect
            </Button>
          </Group>
        </Group>
      </AppShell.Header>

      <AppShell.Navbar p="xs">
        <ScrollArea>
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              component={RouterNavLink}
              to={item.to}
              end={item.end}
              label={item.label}
              leftSection={<item.icon size={18} stroke={1.5} />}
            />
          ))}
        </ScrollArea>
      </AppShell.Navbar>

      <AppShell.Main>
        {/* Cap content width so tables stay readable on wide monitors. */}
        <Box maw={1400} mx="auto">
          <Outlet />
        </Box>
      </AppShell.Main>
    </AppShell>
  );
}
