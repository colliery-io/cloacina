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

import { Badge, Card, Group, Loader, SimpleGrid, Stack, Table, Text, Title } from "@mantine/core";

import { useLiveOpsMetrics } from "../api/operations";
import { formatAgo } from "../util/activity";

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
 * Operations / deployment health (CLOACI-I-0124 / WS-2, made event-driven in
 * CLOACI-T-0718). One place to answer "is my deployment healthy" — server
 * liveness/readiness, the build pipeline, the registry reconciler, and the
 * execution-agent fleet. All tiles update from a single WS push (no polling);
 * the server publishes a fresh snapshot every few seconds while this page is
 * open.
 */
export function Operations() {
  const m = useLiveOpsMetrics(true);

  return (
    <Stack>
      <Group justify="space-between" align="center">
        <Title order={2}>Operations</Title>
        <Group gap="xs">
          <Badge color={m ? "green" : "gray"} variant="dot">
            {m ? "live" : "connecting…"}
          </Badge>
          {m && (
            <Text size="xs" c="dimmed">
              updated {formatAgo(m.ts)}
            </Text>
          )}
        </Group>
      </Group>
      <Text c="dimmed" size="sm">
        Deployment health for the connected server, pushed live over the control-plane
        WebSocket.
      </Text>

      {!m ? (
        <Group gap="xs" mt="lg">
          <Loader size="sm" />
          <Text c="dimmed" size="sm">
            Subscribing to operational metrics…
          </Text>
        </Group>
      ) : (
        <>
          <SimpleGrid cols={{ base: 1, sm: 2, lg: 4 }}>
            {/* Server */}
            <Card withBorder padding="lg">
              <Group justify="space-between" mb="sm">
                <Title order={4}>Server</Title>
                <Badge color={m.server.alive ? "green" : "red"} variant="light">
                  {m.server.alive ? "alive" : "down"}
                </Badge>
              </Group>
              <Stack gap={6}>
                <Stat label="Readiness">
                  <Badge size="sm" variant="light" color={m.server.ready ? "green" : "red"}>
                    {m.server.ready ? "ready" : (m.server.reason ?? "not ready")}
                  </Badge>
                </Stat>
                <Text size="xs" c="dimmed">
                  Liveness + readiness (DB pool + graph health).
                </Text>
              </Stack>
            </Card>

            {/* Compiler / build pipeline */}
            <Card withBorder padding="lg">
              <Group justify="space-between" mb="sm">
                <Title order={4}>Compiler</Title>
                <Badge color={COMPILER_COLOR[m.compiler.status] ?? "gray"} variant="light">
                  {m.compiler.status}
                </Badge>
              </Group>
              <Stack gap={6}>
                <Stat label="Pending">{m.compiler.pending}</Stat>
                <Stat label="Building">{m.compiler.building}</Stat>
                <Stat label="Last success">{fmtTime(m.compiler.last_success_at)}</Stat>
                <Stat label="Last failure">{fmtTime(m.compiler.last_failure_at)}</Stat>
              </Stack>
            </Card>

            {/* Reconciler / package availability (absorbs T-0717) */}
            <Card withBorder padding="lg">
              <Group justify="space-between" mb="sm">
                <Title order={4}>Reconciler</Title>
                <Badge color={m.reconciler.failed > 0 ? "red" : "green"} variant="light">
                  {m.reconciler.status}
                </Badge>
              </Group>
              <Stack gap={6}>
                <Stat label="Built / available">{m.reconciler.built}</Stat>
                <Stat label="Failed builds">
                  <Text span c={m.reconciler.failed > 0 ? "red" : undefined} fw={500} size="sm">
                    {m.reconciler.failed}
                  </Text>
                </Stat>
                <Stat label="Last built">{fmtTime(m.reconciler.last_built_at)}</Stat>
              </Stack>
            </Card>

            {/* Fleet summary */}
            <Card withBorder padding="lg">
              <Group justify="space-between" mb="sm">
                <Title order={4}>Fleet</Title>
                <Badge color={m.fleet.length > 0 ? "green" : "gray"} variant="light">
                  {m.fleet.length} agent{m.fleet.length === 1 ? "" : "s"}
                </Badge>
              </Group>
              <Text size="xs" c="dimmed">
                Execution agents registered against this server. Empty means work runs on the
                in-process executor.
              </Text>
            </Card>
          </SimpleGrid>

          {/* Fleet roster table — only when agents are present */}
          {m.fleet.length > 0 && (
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
                  {m.fleet.map((a) => {
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
        </>
      )}
    </Stack>
  );
}
