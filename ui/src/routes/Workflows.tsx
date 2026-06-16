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

import { Button, Group, Stack, Table, Text, Title } from "@mantine/core";
import { useMemo } from "react";
import { Link, useNavigate } from "react-router-dom";

import { useExecutions } from "../api/executions";
import { useWorkflows } from "../api/workflows";
import { RunCircles, type RunDot } from "../components/RunCircles";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

/** Bucket recent executions (newest-first) by workflow name for the run circles. */
function useRecentRunsByWorkflow(): Map<string, RunDot[]> {
  const recent = useExecutions({ limit: 200, offset: 0 });
  return useMemo(() => {
    const m = new Map<string, RunDot[]>();
    for (const e of recent.data?.items ?? []) {
      const arr = m.get(e.workflow_name) ?? [];
      arr.push({ id: e.id, status: e.status, started_at: e.started_at });
      m.set(e.workflow_name, arr);
    }
    return m;
  }, [recent.data]);
}

/**
 * Workflows list (T-0652 / REQ-003 read half). Establishes the list view
 * pattern: query hook → loading/empty/error states → table with row → detail.
 */
export function Workflows() {
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useWorkflows();
  const runsByWorkflow = useRecentRunsByWorkflow();

  // Only real workflows belong here. A package with no workflow tasks is a
  // pure computation-graph package — it lives in the Graphs view, not the
  // workflow list. (A CG wrapped in `#[workflow]` + a trigger *does* have a
  // task, so it stays.) CLOACI-I-0124 / WS-10.
  const items = (data?.items ?? []).filter((w) => w.tasks.length > 0);

  return (
    <Stack>
      <Group justify="space-between">
        <Title order={2}>Workflows</Title>
        <Button component={Link} to="/workflows/upload">
          Upload
        </Button>
      </Group>
      {isPending ? (
        <Loading label="Loading workflows…" />
      ) : isError ? (
        <ErrorState error={error} onRetry={refetch} />
      ) : items.length === 0 ? (
        <Empty message="No workflows uploaded yet." />
      ) : (
        <Table highlightOnHover stickyHeader>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Package</Table.Th>
              <Table.Th>Version</Table.Th>
              <Table.Th>Tasks</Table.Th>
              <Table.Th>Recent runs</Table.Th>
              <Table.Th>Created</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {items.map((w) => (
              <Table.Tr
                key={w.id}
                style={{ cursor: "pointer" }}
                onClick={() => navigate(`/workflows/${encodeURIComponent(w.package_name)}`)}
              >
                <Table.Td>
                  <Text fw={500}>{w.package_name}</Text>
                  {w.description && (
                    <Text size="xs" c="dimmed">
                      {w.description}
                    </Text>
                  )}
                </Table.Td>
                <Table.Td>{w.version}</Table.Td>
                <Table.Td>{w.tasks.length}</Table.Td>
                <Table.Td>
                  <RunCircles runs={runsByWorkflow.get(w.workflow_name) ?? []} />
                </Table.Td>
                <Table.Td>{formatTimestamp(w.created_at)}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      )}
    </Stack>
  );
}
