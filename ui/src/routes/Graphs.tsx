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

import { Badge, Card, Group, Stack, Table, Text, Title, Tooltip } from "@mantine/core";
import { useMemo } from "react";
import { useNavigate } from "react-router-dom";

import { useAccumulators, useGraphs } from "../api/health";
import { GraphHealth } from "../components/GraphHealth";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { explainToken } from "../util/vocab";

/** Color a graph/accumulator state for an at-a-glance dot (CLOACI-I-0124 / WS-10). */
function stateColor(state: string | undefined): string {
  switch ((state ?? "").toLowerCase()) {
    case "live":
    case "running":
    case "healthy":
      return "green";
    case "warming":
    case "connecting":
    case "starting":
    case "socket_only":
      return "yellow";
    case "degraded":
      return "orange";
    case "crashed":
    case "stopped":
    case "failed":
      return "red";
    default:
      return "gray";
  }
}

function StateDot({ state }: { state: string }) {
  return (
    <span
      style={{
        width: 10,
        height: 10,
        borderRadius: "50%",
        background: `var(--mantine-color-${stateColor(state)}-6)`,
        display: "inline-block",
        flex: "0 0 auto",
      }}
    />
  );
}

/**
 * Computation-graph health (T-0655, OQ-4 → own top-level view): graphs +
 * accumulators visible to the key. Graph rows → per-graph detail.
 */
export function Graphs() {
  const navigate = useNavigate();
  const graphs = useGraphs();
  const accs = useAccumulators();

  // name → status, so a graph row can show its accumulators' health at a glance.
  const accStatus = useMemo(() => {
    const m = new Map<string, string>();
    for (const a of accs.data?.items ?? []) m.set(a.name, String(a.status ?? ""));
    return m;
  }, [accs.data]);

  return (
    <Stack>
      <Title order={2}>Computation graphs</Title>

      <Card withBorder padding="lg">
        <Title order={4} mb="sm">
          Graphs
        </Title>
        {graphs.isPending ? (
          <Loading label="Loading graphs…" />
        ) : graphs.isError ? (
          <ErrorState error={graphs.error} onRetry={graphs.refetch} />
        ) : graphs.data.items.length === 0 ? (
          <Empty message="No graphs loaded." />
        ) : (
          <Table highlightOnHover>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Name</Table.Th>
                <Table.Th>Health</Table.Th>
                <Table.Th>Accumulators</Table.Th>
                <Table.Th>Paused</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {graphs.data.items.map((g) => (
                <Table.Tr
                  key={g.name}
                  style={{ cursor: "pointer" }}
                  onClick={() => navigate(`/graphs/${encodeURIComponent(g.name)}`)}
                >
                  <Table.Td>
                    <Text fw={500}>{g.name}</Text>
                  </Table.Td>
                  <Table.Td>
                    <GraphHealth value={g.health} />
                  </Table.Td>
                  <Table.Td>
                    {g.accumulators.length === 0 ? (
                      <Text size="sm" c="dimmed">
                        —
                      </Text>
                    ) : (
                      <Group gap={6} wrap="nowrap">
                        {g.accumulators.map((name) => {
                          const status = accStatus.get(name) ?? "unknown";
                          return (
                            <Tooltip
                              key={name}
                              label={`${name}: ${explainToken(status).label}`}
                              withArrow
                              openDelay={150}
                            >
                              <span style={{ display: "inline-flex" }}>
                                <StateDot state={status} />
                              </span>
                            </Tooltip>
                          );
                        })}
                        <Text size="xs" c="dimmed">
                          {g.accumulators.length}
                        </Text>
                      </Group>
                    )}
                  </Table.Td>
                  <Table.Td>
                    {g.paused ? (
                      <Badge color="orange" variant="light">
                        paused
                      </Badge>
                    ) : (
                      <Text c="dimmed" size="sm">
                        —
                      </Text>
                    )}
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>

      <Card withBorder padding="lg">
        <Title order={4} mb="sm">
          Accumulators
        </Title>
        {accs.isPending ? (
          <Loading label="Loading accumulators…" />
        ) : accs.isError ? (
          <ErrorState error={accs.error} onRetry={accs.refetch} />
        ) : accs.data.items.length === 0 ? (
          <Empty message="No accumulators registered." />
        ) : (
          <Table>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Name</Table.Th>
                <Table.Th>Status</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {accs.data.items.map((a) => (
                <Table.Tr key={a.name}>
                  <Table.Td>{a.name}</Table.Td>
                  <Table.Td>
                    <GraphHealth value={a.status} />
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  );
}
