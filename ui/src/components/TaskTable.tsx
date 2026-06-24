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

import { StatusBadge } from "@colliery-io/aurora-dark";
import { Code, Table, Text, Tooltip } from "@mantine/core";

import type { TaskExecutionDetail } from "../api/executions";

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
/** Local task id (last `::` segment) used to look the task up in the DAG rank. */
function localId(taskName: string): string {
  return taskName.split("::").pop() || taskName;
}

export function TaskTable({
  tasks,
  order,
}: {
  tasks: TaskExecutionDetail[];
  /** DAG topological rank (`task id → position`) for fixed nominal run order. */
  order?: Map<string, number>;
}) {
  if (tasks.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No task rows recorded for this execution.
      </Text>
    );
  }

  // Fixed run order that never reshuffles as statuses change during a live run
  // — only the status/duration cells update in place. Primary key is the
  // workflow DAG's topological rank (nominal run order: dependencies before
  // dependents); falls back to the immutable `created_at` (then `id`) until the
  // graph loads or for tasks not in it. (Sorting by `started_at`, the old
  // behavior, made rows jump as each task started — CLOACI-I-0124 / WS-1.)
  const rankOf = (t: TaskExecutionDetail): number =>
    order?.get(localId(t.task_name)) ?? Number.MAX_SAFE_INTEGER;
  const rows = [...tasks].sort((a, b) => {
    const ar = rankOf(a);
    const br = rankOf(b);
    if (ar !== br) return ar - br;
    const ac = Date.parse(a.created_at);
    const bc = Date.parse(b.created_at);
    if (ac !== bc) return ac - bc;
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
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
