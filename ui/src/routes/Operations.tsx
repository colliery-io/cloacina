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

import { Badge, Card, Group, SimpleGrid, Stack, Table, Text, Title } from "@mantine/core";

import { useCompilerStatus, useFleet, useServerHealth } from "../api/operations";

function ago(seconds: number | null): string {
  if (seconds == null) return "—";
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function fmtTime(ts: string | null): string {
  if (!ts) return "never";
  const t = Date.parse(ts);
  return Number.isNaN(t) ? "—" : new Date(t).toLocaleString();
}

/** A labelled metric line inside a tile. */
function Stat({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <Group justify="space-between" gap="xs">
      <Text size="sm" c="dimmed">
        {label}
      </Text>
      <Text size="sm" fw={500}>
        {children}
      </Text>
    </Group>
  );
}

const COMPILER_COLOR: Record<string, string> = {
  building: "blue",
  backlogged: "orange",
  idle: "gray",
};

/**
 * Operations / deployment health (CLOACI-I-0124 / WS-2). One place to answer
 * "is my deployment healthy" — server liveness/readiness, the build pipeline,
 * and the execution-agent fleet. All tiles poll every 5s.
 */
export function Operations() {
  const server = useServerHealth();
  const compiler = useCompilerStatus();
  const fleet = useFleet();

  return (
    <Stack>
      <Title order={2}>Operations</Title>
      <Text c="dimmed" size="sm">
        Deployment health for the connected server. Auto-refreshes every 5 seconds.
      </Text>

      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }}>
        {/* Server */}
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Server</Title>
            <Badge color={server.data?.alive ? "green" : "red"} variant="light">
              {server.isPending ? "…" : server.data?.alive ? "alive" : "down"}
            </Badge>
          </Group>
          <Stack gap={6}>
            <Stat label="Readiness">
              {server.isPending ? (
                "…"
              ) : (
                <Badge
                  size="sm"
                  variant="light"
                  color={server.data?.ready ? "green" : "red"}
                >
                  {server.data?.ready ? "ready" : (server.data?.reason ?? "not ready")}
                </Badge>
              )}
            </Stat>
            <Text size="xs" c="dimmed">
              Liveness <code>/health</code> · readiness <code>/ready</code> (DB pool + graph health).
            </Text>
          </Stack>
        </Card>

        {/* Compiler / build pipeline */}
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Compiler</Title>
            <Badge
              color={compiler.data ? (COMPILER_COLOR[compiler.data.status] ?? "gray") : "gray"}
              variant="light"
            >
              {compiler.isPending ? "…" : compiler.isError ? "error" : compiler.data?.status}
            </Badge>
          </Group>
          <Stack gap={6}>
            <Stat label="Pending">{compiler.data?.pending ?? "—"}</Stat>
            <Stat label="Building">{compiler.data?.building ?? "—"}</Stat>
            <Stat label="Last success">{fmtTime(compiler.data?.last_success_at ?? null)}</Stat>
            <Stat label="Last failure">{fmtTime(compiler.data?.last_failure_at ?? null)}</Stat>
          </Stack>
        </Card>

        {/* Fleet summary */}
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Execution-agent fleet</Title>
            <Badge color={fleet.data && fleet.data.length > 0 ? "green" : "gray"} variant="light">
              {fleet.isPending ? "…" : `${fleet.data?.length ?? 0} agent(s)`}
            </Badge>
          </Group>
          <Text size="xs" c="dimmed">
            Agents registered against this server. Empty means work runs on the in-process
            executor (no remote fleet).
          </Text>
        </Card>
      </SimpleGrid>

      {/* Fleet roster table — only when agents are present */}
      {fleet.data && fleet.data.length > 0 && (
        <Card withBorder padding="lg">
          <Title order={4} mb="sm">
            Agents
          </Title>
          <Table striped highlightOnHover withTableBorder verticalSpacing="xs">
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Agent</Table.Th>
                <Table.Th>Target</Table.Th>
                <Table.Th>Capacity</Table.Th>
                <Table.Th>Last heartbeat</Table.Th>
                <Table.Th>Tenant</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {fleet.data.map((a) => {
                const stale = a.seconds_since_heartbeat != null && a.seconds_since_heartbeat > 60;
                return (
                  <Table.Tr key={a.agent_id}>
                    <Table.Td>
                      <Text size="sm" fw={500}>
                        {a.agent_id}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm" c="dimmed">
                        {a.target_triple}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">
                        {a.in_flight}/{a.max_concurrency} in flight
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm" c={stale ? "red" : undefined}>
                        {ago(a.seconds_since_heartbeat)}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm" c="dimmed">
                        {a.tenant_id ?? "—"}
                      </Text>
                    </Table.Td>
                  </Table.Tr>
                );
              })}
            </Table.Tbody>
          </Table>
        </Card>
      )}
    </Stack>
  );
}
