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

import { Code, Stack, Table, Text } from "@mantine/core";
import type { schemas } from "@cloacina/client";

import { formatTimestamp } from "../util/format";

export type ExecutionEvent = schemas["ExecutionEvent"];

/**
 * Ordered execution event log (T-0653). Pure presentation over a
 * normalized event array sorted by `sequence_num` — **the data model
 * T-0656 builds on**: the live hook appends WS events into the same array
 * (deduped by `sequence_num`) and re-renders this component unchanged.
 *
 * `event_data` is a JSON-encoded string (nullable); pretty-print it when
 * it parses, else show it raw.
 */
function prettyData(raw: string | null | undefined): string | null {
  if (raw == null || raw === "") return null;
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

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
        {sorted.map((e) => {
          const data = prettyData(e.event_data);
          return (
            <Table.Tr key={`${e.sequence_num}-${e.id}`}>
              <Table.Td>
                <Text c="dimmed" size="sm">
                  {e.sequence_num}
                </Text>
              </Table.Td>
              <Table.Td>
                <Stack gap={2}>
                  <Text size="sm" fw={500}>
                    {e.event_type}
                  </Text>
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
