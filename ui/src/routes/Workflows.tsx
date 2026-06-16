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

import { Badge, Button, Group, Stack, Table, Text, Title, Tooltip } from "@mantine/core";
import { Link, useNavigate } from "react-router-dom";

import { useWorkflows } from "../api/workflows";
import { Empty, ErrorState, Loading } from "../components/states/States";
import { formatTimestamp } from "../util/format";

/**
 * Workflows list (T-0652 / REQ-003 read half). Establishes the list view
 * pattern: query hook → loading/empty/error states → table with row → detail.
 */
export function Workflows() {
  const navigate = useNavigate();
  const { data, isPending, isError, error, refetch } = useWorkflows();

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
      ) : data.items.length === 0 ? (
        <Empty message="No workflows uploaded yet." />
      ) : (
        <Table highlightOnHover stickyHeader>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Package</Table.Th>
              <Table.Th>Version</Table.Th>
              <Table.Th>Tasks</Table.Th>
              <Table.Th>Created</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {data.items.map((w) => (
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
                <Table.Td>
                  {w.tasks.length > 0 ? (
                    w.tasks.length
                  ) : (
                    // A package with no workflow tasks is a computation-graph
                    // package — show what it is instead of a "0" that reads as
                    // broken (CLOACI-I-0124 / WS-7).
                    <Tooltip
                      label="Computation-graph package — it has no workflow tasks. See the Graphs view for its nodes."
                      multiline
                      w={260}
                      withArrow
                    >
                      <Badge variant="light" color="grape">
                        graph
                      </Badge>
                    </Tooltip>
                  )}
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
