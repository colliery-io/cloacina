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

import { Code, Table, Text, Tooltip } from "@mantine/core";

import type { TaskExecutionDetail } from "../api/executions";
import { StatusBadge } from "./StatusBadge";

/** Human duration between two RFC-3339 timestamps (or start→now while running). */
function duration(started: string | null, completed: string | null): string {
  if (!started) return "—";
  const start = Date.parse(started);
  const end = completed ? Date.parse(completed) : Date.now();
  if (Number.isNaN(start) || Number.isNaN(end) || end < start) return "—";
  const ms = end - start;
  if (ms < 1000) return `${ms} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(s < 10 ? 1 : 0)} s`;
  const m = Math.floor(s / 60);
  return `${m}m ${Math.round(s % 60)}s`;
}

function fmtTime(ts: string | null): string {
  if (!ts) return "—";
  const t = Date.parse(ts);
  return Number.isNaN(t) ? "—" : new Date(t).toLocaleTimeString();
}

/**
 * Per-task breakdown of an execution (CLOACI-I-0124 / WS-1). Replaces the
 * raw event log as the primary "what ran, how long, and why it failed" view.
 */
export function TaskTable({ tasks }: { tasks: TaskExecutionDetail[] }) {
  if (tasks.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No task rows recorded for this execution.
      </Text>
    );
  }

  // Order by start time so the table reads as the execution timeline; unstarted
  // tasks sink to the bottom.
  const rows = [...tasks].sort((a, b) => {
    const as = a.started_at ? Date.parse(a.started_at) : Number.MAX_SAFE_INTEGER;
    const bs = b.started_at ? Date.parse(b.started_at) : Number.MAX_SAFE_INTEGER;
    return as - bs;
  });

  return (
    <Table striped highlightOnHover withTableBorder verticalSpacing="xs">
      <Table.Thead>
        <Table.Tr>
          <Table.Th>Task</Table.Th>
          <Table.Th>Status</Table.Th>
          <Table.Th>Started</Table.Th>
          <Table.Th>Duration</Table.Th>
          <Table.Th>Attempt</Table.Th>
          <Table.Th>Error</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>
        {rows.map((t) => {
          const err = t.last_error ?? t.error_details ?? null;
          // Show the local task name (last `::` segment); full namespaced id on hover.
          const localName = t.task_name.split("::").pop() || t.task_name;
          // Prefer the executor's run timestamps; fall back to the row's
          // created/updated bounds when the runner didn't stamp started/completed.
          const start = t.started_at ?? t.created_at;
          const end = t.completed_at ?? t.updated_at;
          return (
            <Table.Tr key={t.id}>
              <Table.Td style={{ maxWidth: 280 }}>
                <Tooltip label={t.task_name} withArrow openDelay={300}>
                  <Text size="sm" fw={500} truncate>
                    {localName}
                  </Text>
                </Tooltip>
              </Table.Td>
              <Table.Td style={{ whiteSpace: "nowrap" }}>
                <StatusBadge status={t.status} />
                {t.sub_status ? (
                  <Text span size="xs" c="dimmed" ml={6}>
                    {t.sub_status}
                  </Text>
                ) : null}
              </Table.Td>
              <Table.Td style={{ whiteSpace: "nowrap" }}>
                <Text size="sm" c="dimmed">
                  {fmtTime(start)}
                </Text>
              </Table.Td>
              <Table.Td style={{ whiteSpace: "nowrap" }}>
                <Text size="sm">{duration(start, end)}</Text>
              </Table.Td>
              <Table.Td>
                <Text size="sm" c={t.attempt > 1 ? "orange" : undefined}>
                  {t.attempt}/{t.max_attempts}
                </Text>
              </Table.Td>
              <Table.Td>
                {err ? (
                  <Tooltip label={err} multiline w={360} withArrow>
                    <Code c="red" style={{ cursor: "help" }}>
                      {err.length > 48 ? `${err.slice(0, 48)}…` : err}
                    </Code>
                  </Tooltip>
                ) : (
                  <Text size="sm" c="dimmed">
                    —
                  </Text>
                )}
              </Table.Td>
            </Table.Tr>
          );
        })}
      </Table.Tbody>
    </Table>
  );
}
