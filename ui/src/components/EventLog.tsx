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

import { Badge, Code, Group, Stack, Table, Text } from "@mantine/core";
import type { schemas } from "@cloacina/client";

import { formatTimestamp } from "../util/format";
import { describeEvent, meaningfulData } from "../util/eventLabels";

export type ExecutionEvent = schemas["ExecutionEvent"];

/**
 * Ordered execution event log (T-0653; CLOACI-I-0124 / WS-9 readability pass).
 * Pure presentation over a normalized event array sorted by `sequence_num` —
 * **the data model T-0656 builds on**: the live hook appends WS events into the
 * same array (deduped by `sequence_num`) and re-renders this unchanged.
 *
 * WS-9: events are shown as humanized labels with a status color and the
 * **task name** they're about (when task-scoped); empty `{}` payloads are
 * hidden; the row number is a per-execution ordinal, not the global
 * `sequence_num` (which is a server-wide counter and meaningless here).
 */
export function EventLog({ events }: { events: ExecutionEvent[] }) {
  if (events.length === 0) {
    return (
      <Text c="dimmed" size="sm">
        No events yet.
      </Text>
    );
  }
  const sorted = [...events].sort((a, b) => a.sequence_num - b.sequence_num);

  return (
    <Table striped withRowBorders={false} verticalSpacing="xs">
      <Table.Thead>
        <Table.Tr>
          <Table.Th w={48}>#</Table.Th>
          <Table.Th>Event</Table.Th>
          <Table.Th w={200}>Time</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>
        {sorted.map((e, i) => {
          const data = meaningfulData(e.event_data);
          const { label, color } = describeEvent(e.event_type);
          // `task_name` is optional on the SDK type (workflow-scoped events and
          // live WS events may lack it); shorten the namespaced name like the
          // task table does.
          const taskName = (e as { task_name?: string | null }).task_name;
          const localTask = taskName ? taskName.split("::").pop() : null;
          return (
            <Table.Tr key={`${e.sequence_num}-${e.id}`}>
              <Table.Td>
                <Text c="dimmed" size="sm">
                  {i + 1}
                </Text>
              </Table.Td>
              <Table.Td>
                <Stack gap={4}>
                  <Group gap="xs">
                    <Badge variant="light" color={color} size="sm">
                      {label}
                    </Badge>
                    {localTask && (
                      <Text size="sm" c="dimmed">
                        {localTask}
                      </Text>
                    )}
                  </Group>
                  {data && (
                    <Code block fz="xs">
                      {data}
                    </Code>
                  )}
                </Stack>
              </Table.Td>
              <Table.Td>
                <Text c="dimmed" size="sm">
                  {formatTimestamp(e.created_at)}
                </Text>
              </Table.Td>
            </Table.Tr>
          );
        })}
      </Table.Tbody>
    </Table>
  );
}
