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

import { Box, Group, Text, Tooltip } from "@mantine/core";

import type { TaskRuntimeStat } from "../api/executions";
import { formatMs } from "../util/format";

/**
 * Cross-run task-duration chart (CLOACI-I-0124 — the Airflow "Task Duration"
 * aggregate). One row per task: the filled bar is the mean duration over the
 * sampled runs, the lighter band behind it spans observed min→max, and a
 * marker shows the most-recent run so a regression reads at a glance against
 * the average. All bars share one scale (the slowest task's max), so the row
 * that dominates the workflow's runtime is the longest bar.
 *
 * Rows arrive pre-ordered by the caller (DAG rank); a stat with no rank sorts
 * last.
 */
export function TaskRuntimeChart({ stats }: { stats: TaskRuntimeStat[] }) {
  if (stats.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No completed task runs in the sampled window yet.
      </Text>
    );
  }

  const scale = Math.max(...stats.map((s) => s.maxMs), 1);
  const pct = (ms: number) => `${Math.max((ms / scale) * 100, 0.6)}%`;

  return (
    <Box style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {stats.map((s) => {
        // A "last run" marker that's notably above the mean is the regression
        // signal; colour it amber past 1.5× the average.
        const slow = s.lastMs != null && s.lastMs > s.avgMs * 1.5 && s.count > 1;
        return (
          <Box
            key={s.taskName}
            style={{ display: "grid", gridTemplateColumns: "180px 1fr 64px", alignItems: "center", gap: 8 }}
          >
            <Tooltip label={`${s.count} run${s.count === 1 ? "" : "s"} sampled`} withArrow openDelay={300} position="left">
              <Text size="xs" truncate fw={500}>
                {s.taskName}
              </Text>
            </Tooltip>
            <Box style={{ position: "relative", height: 18, background: "var(--mantine-color-gray-1)", borderRadius: 3 }}>
              {/* min→max observed range */}
              <Box
                style={{
                  position: "absolute",
                  left: pct(s.minMs),
                  width: `${Math.max(((s.maxMs - s.minMs) / scale) * 100, 0)}%`,
                  top: 6,
                  bottom: 6,
                  background: "var(--mantine-color-blue-2)",
                  borderRadius: 2,
                }}
              />
              {/* mean bar */}
              <Tooltip
                label={`avg ${formatMs(s.avgMs)} · min ${formatMs(s.minMs)} · max ${formatMs(s.maxMs)}`}
                withArrow
                openDelay={150}
              >
                <Box
                  style={{
                    position: "absolute",
                    left: 0,
                    width: pct(s.avgMs),
                    top: 2,
                    bottom: 2,
                    background: "var(--mantine-color-blue-6)",
                    borderRadius: 3,
                  }}
                />
              </Tooltip>
              {/* most-recent run marker */}
              {s.lastMs != null && (
                <Tooltip label={`last run ${formatMs(s.lastMs)}`} withArrow openDelay={150}>
                  <Box
                    style={{
                      position: "absolute",
                      left: pct(s.lastMs),
                      top: 0,
                      bottom: 0,
                      width: 2,
                      background: slow ? "var(--mantine-color-orange-7)" : "var(--mantine-color-dark-5)",
                    }}
                  />
                </Tooltip>
              )}
            </Box>
            <Text size="xs" c={slow ? "orange" : "dimmed"} ta="right" style={{ whiteSpace: "nowrap" }}>
              {formatMs(s.avgMs)}
            </Text>
          </Box>
        );
      })}
      <Group gap="md" mt={4}>
        <Legend swatch="var(--mantine-color-blue-6)" label="mean" />
        <Legend swatch="var(--mantine-color-blue-2)" label="min–max" />
        <Legend swatch="var(--mantine-color-dark-5)" label="last run" />
      </Group>
    </Box>
  );
}

function Legend({ swatch, label }: { swatch: string; label: string }) {
  return (
    <Group gap={4}>
      <Box style={{ width: 10, height: 10, background: swatch, borderRadius: 2 }} />
      <Text size="xs" c="dimmed">
        {label}
      </Text>
    </Group>
  );
}
