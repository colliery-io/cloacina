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

import { Box, Code, Group, Stack, Table, Text } from "@mantine/core";
import type { schemas } from "@cloacina/client";

import { formatTimestamp } from "../util/format";
import { describeEvent, meaningfulData } from "../util/eventLabels";
import { MONO, Pill } from "./aurora";
import { TOKEN } from "../util/tokens";

export type ExecutionEvent = schemas["ExecutionEvent"];

/** Map the event-label Mantine color name to an Aurora token (spec §Event log:
 *  started/snatched → ice, completed/imported → green, failed → red,
 *  scheduled/upgrade → violet, retry → gold). */
function kindToken(color: string): string {
  switch (color) {
    case "blue":
      return TOKEN.ice;
    case "green":
      return TOKEN.ok;
    case "red":
      return TOKEN.bad;
    case "salmon":
      return TOKEN.salmon;
    case "grape":
    case "violet":
      return TOKEN.violet;
    case "orange":
    case "yellow":
    case "gold":
      return TOKEN.gold;
    case "teal":
    case "cyan":
      return TOKEN.teal;
    default:
      return TOKEN.muted;
  }
}

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
  const th: React.CSSProperties = {
    fontFamily: MONO,
    fontSize: 10,
    letterSpacing: ".07em",
    textTransform: "uppercase",
    color: "var(--faint)",
    fontWeight: 500,
    textAlign: "left",
  };

  return (
    <Box style={{ background: "var(--inset)", border: "1px solid var(--border-soft)", borderRadius: 10, padding: "4px 14px" }}>
      <Table withRowBorders={false} verticalSpacing={7}>
        <Table.Thead>
          <Table.Tr>
            <Table.Th style={{ ...th, width: 48 }}>#</Table.Th>
            <Table.Th style={th}>Event</Table.Th>
            <Table.Th style={{ ...th, width: 200 }}>Time</Table.Th>
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
                  <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--fainter)" }}>{i + 1}</span>
                </Table.Td>
                <Table.Td>
                  <Stack gap={4}>
                    <Group gap="xs">
                      <Pill color={kindToken(color)}>{label}</Pill>
                      {localTask && (
                        <span style={{ fontFamily: MONO, fontSize: 11.5, color: "var(--fg-2)" }}>{localTask}</span>
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
                  <span style={{ fontFamily: MONO, fontSize: 11, color: "var(--faint)" }}>{formatTimestamp(e.created_at)}</span>
                </Table.Td>
              </Table.Tr>
            );
          })}
        </Table.Tbody>
      </Table>
    </Box>
  );
}
