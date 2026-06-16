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
import { useMemo } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { useGraphs } from "../api/health";
import { useWorkflows } from "../api/workflows";
import { GraphHealth } from "../components/GraphHealth";
import { RunCircles, type RunDot } from "../components/RunCircles";
import { StatusBadge } from "../components/StatusBadge";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";
import { useGraphThroughput } from "../util/activity";

const PREVIEW = 8;

/**
 * Overview dashboard (T-0655; CLOACI-I-0124 WS-3 redesign). Real, navigable
 * lists of the two primitives an operator manages — workflows and computation
 * graphs — plus recent executions. Replaces the earlier summary "count cards"
 * (which read as totals and hid graph health). Each list previews the first
 * few and links to its full, paginated view.
 */
export function Overview() {
  const navigate = useNavigate();
  const workflows = useWorkflows();
  const graphs = useGraphs();
  // One executions query feeds both the recent-runs preview and the per-workflow
  // run circles (CLOACI-I-0124 / WS-10).
  const recent = useExecutions({ limit: 200, offset: 0 });

  // Workflows tile lists real workflows only; pure computation-graph packages
  // (no workflow tasks) appear in the Computation graphs tile instead (WS-10).
  const wfItems = (workflows.data?.items ?? []).filter((w) => w.tasks.length > 0);
  const graphItems = graphs.data?.items ?? [];
  const graphTp = useGraphThroughput(graphItems);
  const recentItems = recent.data?.items ?? [];

  const runsByWorkflow = useMemo(() => {
    const m = new Map<string, RunDot[]>();
    for (const e of recentItems) {
      const arr = m.get(e.workflow_name) ?? [];
      arr.push({ id: e.id, status: e.status, started_at: e.started_at });
      m.set(e.workflow_name, arr);
    }
    return m;
  }, [recentItems]);

  return (
    <Stack>
      <Title order={2}>Overview</Title>

      <SimpleGrid cols={{ base: 1, lg: 2 }}>
        {/* Workflows */}
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Workflows</Title>
            <Anchor component={Link} to="/workflows" size="sm">
              {wfItems.length > PREVIEW ? `All ${wfItems.length}` : "Manage"}
            </Anchor>
          </Group>
          {workflows.isPending ? (
            <Loading label="Loading…" />
          ) : workflows.isError ? (
            <ErrorState error={workflows.error} onRetry={workflows.refetch} />
          ) : wfItems.length === 0 ? (
            <Empty message="No workflows uploaded yet." />
          ) : (
            <Table highlightOnHover verticalSpacing="xs">
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Package</Table.Th>
                  <Table.Th>Tasks</Table.Th>
                  <Table.Th>Recent runs</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {wfItems.slice(0, PREVIEW).map((w) => (
                  <Table.Tr
                    key={w.id}
                    style={{ cursor: "pointer" }}
                    onClick={() => navigate(`/workflows/${encodeURIComponent(w.package_name)}`)}
                  >
                    <Table.Td>
                      <Text size="sm" fw={500}>
                        {w.package_name}
                      </Text>
                      <Text size="xs" c="dimmed">
                        {w.version}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm">{w.tasks.length}</Text>
                    </Table.Td>
                    <Table.Td>
                      <RunCircles runs={runsByWorkflow.get(w.workflow_name) ?? []} max={6} />
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          )}
        </Card>

        {/* Computation graphs */}
        <Card withBorder padding="lg">
          <Group justify="space-between" mb="sm">
            <Title order={4}>Computation graphs</Title>
            <Anchor component={Link} to="/graphs" size="sm">
              {graphItems.length > PREVIEW ? `All ${graphItems.length}` : "View"}
            </Anchor>
          </Group>
          {graphs.isPending ? (
            <Loading label="Loading…" />
          ) : graphs.isError ? (
            <ErrorState error={graphs.error} onRetry={graphs.refetch} />
          ) : graphItems.length === 0 ? (
            <Empty message="No computation graphs loaded." />
          ) : (
            <Table highlightOnHover verticalSpacing="xs">
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Name</Table.Th>
                  <Table.Th>Health</Table.Th>
                  <Table.Th>Throughput</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {graphItems.slice(0, PREVIEW).map((g) => {
                  const rate = graphTp.get(g.name);
                  return (
                  <Table.Tr
                    key={g.name}
                    style={{ cursor: "pointer" }}
                    onClick={() => navigate(`/graphs/${encodeURIComponent(g.name)}`)}
                  >
                    <Table.Td>
                      <Text size="sm" fw={500}>
                        {g.name}
                      </Text>
                    </Table.Td>
                    <Table.Td>
                      <GraphHealth value={g.health} />
                    </Table.Td>
                    <Table.Td>
                      <Text size="sm" c={rate == null ? "dimmed" : undefined}>
                        {rate == null ? "—" : `~${rate}/min`}
                      </Text>
                    </Table.Td>
                  </Table.Tr>
                  );
                })}
              </Table.Tbody>
            </Table>
          )}
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
        ) : recentItems.length === 0 ? (
          <Empty message="No executions yet." />
        ) : (
          <Table highlightOnHover>
            <Table.Tbody>
              {recentItems.slice(0, 5).map((e) => (
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
