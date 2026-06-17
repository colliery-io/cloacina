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

import { Group, Text, Tooltip } from "@mantine/core";

import { executionStatusColor } from "../util/status";

/** Minimal recent-run shape (a slice of ExecutionSummary). */
export interface RunDot {
  id: string;
  status: string;
  started_at?: string | null;
}

/** How many recent runs to summarize (Airflow's DAG-runs default cadence). */
const WINDOW = 25;

// Display order for the state circles — active first, then terminal.
const STATE_ORDER = [
  "running",
  "pending",
  "scheduled",
  "completed",
  "failed",
  "cancelled",
  "canceled",
];

function rank(status: string): number {
  const i = STATE_ORDER.indexOf(status);
  return i < 0 ? STATE_ORDER.length : i;
}

/**
 * Airflow-style recent-run summary (CLOACI-I-0124 / WS-10). Rather than one dot
 * per run, this shows one colored circle **per run state** with the **count** of
 * recent runs in that state — i.e. "how many succeeded / failed / are running",
 * mirroring Airflow's DAGs-view circles. Counts are over the most recent
 * `WINDOW` runs. `runs` is expected newest-first (the executions list order).
 */
export function RunCircles({ runs }: { runs: RunDot[] }) {
  if (runs.length === 0) {
    return (
      <Text c="dimmed" size="xs">
        no recent runs
      </Text>
    );
  }

  const counts = new Map<string, number>();
  for (const r of runs.slice(0, WINDOW)) {
    const key = r.status.toLowerCase();
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  const entries = [...counts.entries()].sort((a, b) => rank(a[0]) - rank(b[0]));

  return (
    <Group gap={6} wrap="nowrap">
      {entries.map(([status, count]) => (
        <Tooltip key={status} label={`${count} ${status}`} withArrow openDelay={150}>
          <span
            style={{
              minWidth: 20,
              height: 20,
              padding: "0 5px",
              borderRadius: 10,
              background: `var(--mantine-color-${executionStatusColor(status)}-6)`,
              color: "var(--mantine-color-white)",
              fontSize: 11,
              fontWeight: 600,
              lineHeight: "20px",
              textAlign: "center",
              display: "inline-block",
              flex: "0 0 auto",
            }}
          >
            {count}
          </span>
        </Tooltip>
      ))}
    </Group>
  );
}
