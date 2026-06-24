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

import { executionStatusColor, isTerminalStatus } from "@colliery-io/aurora-dark";
import { Box, Group, Text, Tooltip } from "@mantine/core";

import type { TaskExecutionDetail } from "../api/executions";
import { formatMs } from "../util/format";

/** Local task id (last `::` segment) for the DAG-rank lookup. */
const localId = (name: string) => name.split("::").pop() || name;

type Bar = {
  id: string;
  name: string;
  fullName: string;
  status: string;
  start: number;
  end: number;
  running: boolean;
};

/**
 * Per-run task Gantt (CLOACI-I-0124 — the Airflow "Gantt" tab). Each task is a
 * horizontal bar on a shared time axis: left edge = start offset from the run's
 * first task, width = duration, colour = status. Reading it top-to-bottom in
 * the fixed DAG order shows where time actually went — serial chains stack
 * diagonally, parallel fan-out lines up vertically, and gaps between bars are
 * scheduler/queue latency. The "wall vs work" footer contrasts total elapsed
 * against summed task time as a parallelism hint.
 *
 * Rows never reshuffle as a live run progresses (same fixed-order contract as
 * the task table — WS-1); only bar geometry and colour update in place.
 */
export function TaskGantt({
  tasks,
  order,
}: {
  tasks: TaskExecutionDetail[];
  /** DAG topological rank (`task id → position`) for fixed nominal run order. */
  order?: Map<string, number>;
}) {
  const now = Date.now();
  const bars: Bar[] = tasks
    .map((t) => {
      // Prefer the executor's run timestamps; fall back to the row's
      // created/updated bounds when started/completed weren't stamped.
      const startTs = Date.parse(t.started_at ?? t.created_at);
      const running = !isTerminalStatus(t.status);
      const endRaw = t.completed_at ?? (running ? null : t.updated_at);
      const end = endRaw ? Date.parse(endRaw) : now;
      if (Number.isNaN(startTs)) return null;
      return {
        id: t.id,
        name: localId(t.task_name),
        fullName: t.task_name,
        status: t.status,
        start: startTs,
        end: Number.isNaN(end) || end < startTs ? startTs : end,
        running,
      } satisfies Bar;
    })
    .filter((b): b is Bar => b !== null);

  if (bars.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No task timing recorded for this run yet.
      </Text>
    );
  }

  const rankOf = (b: Bar) => order?.get(b.name) ?? Number.MAX_SAFE_INTEGER;
  bars.sort((a, b) => rankOf(a) - rankOf(b) || a.start - b.start);

  const t0 = Math.min(...bars.map((b) => b.start));
  const t1 = Math.max(...bars.map((b) => b.end));
  const span = Math.max(t1 - t0, 1);
  const wall = t1 - t0;
  const work = bars.reduce((sum, b) => sum + (b.end - b.start), 0);

  return (
    <Box>
      <Box style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {bars.map((b) => {
          const leftPct = ((b.start - t0) / span) * 100;
          const widthPct = Math.max(((b.end - b.start) / span) * 100, 0.6);
          const color = `var(--mantine-color-${executionStatusColor(b.status)}-6)`;
          const dur = formatMs(b.end - b.start);
          return (
            <Box
              key={b.id}
              style={{ display: "grid", gridTemplateColumns: "180px 1fr", alignItems: "center", gap: 8 }}
            >
              <Tooltip label={b.fullName} withArrow openDelay={300} position="left">
                <Text size="xs" truncate fw={500}>
                  {b.name}
                </Text>
              </Tooltip>
              <Box style={{ position: "relative", height: 18, background: "var(--mantine-color-gray-1)", borderRadius: 3 }}>
                <Tooltip
                  label={`${b.status} · ${dur}${b.running ? " (running)" : ""}`}
                  withArrow
                  openDelay={150}
                >
                  <Box
                    style={{
                      position: "absolute",
                      left: `${leftPct}%`,
                      width: `${widthPct}%`,
                      top: 2,
                      bottom: 2,
                      background: color,
                      borderRadius: 3,
                      minWidth: 3,
                      opacity: b.running ? 0.7 : 1,
                      display: "flex",
                      alignItems: "center",
                      paddingInline: 4,
                      overflow: "hidden",
                    }}
                  >
                    {widthPct > 12 && (
                      <Text size="xs" c="white" style={{ whiteSpace: "nowrap" }}>
                        {dur}
                      </Text>
                    )}
                  </Box>
                </Tooltip>
              </Box>
            </Box>
          );
        })}
      </Box>
      <Group justify="space-between" mt="xs">
        <Text size="xs" c="dimmed">
          Wall-clock {formatMs(wall)}
        </Text>
        <Text size="xs" c="dimmed">
          Σ task time {formatMs(work)}
          {work > wall * 1.05 ? ` · ${(work / Math.max(wall, 1)).toFixed(1)}× parallelism` : ""}
        </Text>
      </Group>
    </Box>
  );
}
