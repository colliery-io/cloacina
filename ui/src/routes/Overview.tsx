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

import { Anchor, Card, Group, SimpleGrid, Stack, Table, Text, Title } from "@mantine/core";
import { Link, useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { useGraphs } from "../api/health";
import { StatusBadge } from "../components/StatusBadge";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

/**
 * Overview dashboard (T-0655 / REQ-002): recent executions + a status
 * rollup + a graph-health summary tile. Each tile deep-links to its full
 * view. Composed from the existing executions/health hooks. Graph-health
 * *detail* lives at /graphs (OQ-4 decision); here it's a summary.
 */
export function Overview() {
  const navigate = useNavigate();
  const recent = useExecutions({ limit: 5, offset: 0 });
  const graphs = useGraphs();

  const rollup = (() => {
    if (!recent.data) return [];
    const counts = new Map<string, number>();
    for (const e of recent.data.items) counts.set(e.status, (counts.get(e.status) ?? 0) + 1);
    return [...counts.entries()];
  })();

  const graphSummary = graphs.data
    ? {
        total: graphs.data.items.length,
        paused: graphs.data.items.filter((g) => g.paused).length,
      }
    : null;

  return (
    <Stack>
      <Title order={2}>Overview</Title>

      <SimpleGrid cols={{ base: 1, sm: 3 }}>
        {/* Status rollup */}
        <Card withBorder padding="lg">
          <Text fw={600} mb="xs">
            Recent status
          </Text>
          {recent.isPending ? (
            <Loading label="…" />
          ) : recent.isError ? (
            <ErrorState error={recent.error} onRetry={recent.refetch} />
          ) : rollup.length === 0 ? (
            <Text c="dimmed" size="sm">
              No recent executions.
            </Text>
          ) : (
            <Group gap="xs">
              {rollup.map(([status, n]) => (
                <Group key={status} gap={4}>
                  <StatusBadge status={status} />
                  <Text size="sm">{n}</Text>
                </Group>
              ))}
            </Group>
          )}
        </Card>

        {/* Graph-health summary → /graphs */}
        <Card withBorder padding="lg">
          <Group justify="space-between">
            <Text fw={600}>Computation graphs</Text>
            <Anchor component={Link} to="/graphs" size="sm">
              View
            </Anchor>
          </Group>
          {graphs.isPending ? (
            <Loading label="…" />
          ) : graphs.isError ? (
            <ErrorState error={graphs.error} onRetry={graphs.refetch} />
          ) : (
            <Text size="sm" c="dimmed">
              {graphSummary?.total ?? 0} loaded
              {graphSummary && graphSummary.paused > 0 ? `, ${graphSummary.paused} paused` : ""}
            </Text>
          )}
        </Card>

        {/* Quick link */}
        <Card withBorder padding="lg">
          <Text fw={600} mb="xs">
            Workflows
          </Text>
          <Anchor component={Link} to="/workflows" size="sm">
            Manage workflows
          </Anchor>
        </Card>
      </SimpleGrid>

      {/* Recent executions */}
      <Card withBorder padding="lg">
        <Group justify="space-between" mb="sm">
          <Title order={4}>Recent executions</Title>
          <Anchor component={Link} to="/executions" size="sm">
            All executions
          </Anchor>
        </Group>
        {recent.isPending ? (
          <Loading label="Loading…" />
        ) : recent.isError ? (
          <ErrorState error={recent.error} onRetry={recent.refetch} />
        ) : recent.data.items.length === 0 ? (
          <Empty message="No executions yet." />
        ) : (
          <Table highlightOnHover>
            <Table.Tbody>
              {recent.data.items.map((e) => (
                <Table.Tr
                  key={e.id}
                  style={{ cursor: "pointer" }}
                  onClick={() => navigate(`/executions/${e.id}`)}
                >
                  <Table.Td>{e.workflow_name}</Table.Td>
                  <Table.Td>
                    <StatusBadge status={e.status} />
                  </Table.Td>
                  <Table.Td>{formatTimestamp(e.started_at)}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  );
}
