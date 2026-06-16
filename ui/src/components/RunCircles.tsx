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

/** Minimal recent-run shape the circles need (a slice of ExecutionSummary). */
export interface RunDot {
  id: string;
  status: string;
  started_at?: string | null;
}

function whenLabel(ts: string | null | undefined): string {
  if (!ts) return "";
  const t = Date.parse(ts);
  return Number.isNaN(t) ? "" : ` · ${new Date(t).toLocaleString()}`;
}

/**
 * Airflow-style recent-run status circles (CLOACI-I-0124 / WS-10). One dot per
 * recent execution, colored by status (green=completed, red=failed, blue=
 * running, …), oldest→newest left to right so the rightmost dot is the latest.
 * A quick visual read of "how has this workflow been doing lately" on the list
 * pages. `runs` is expected newest-first (the executions list order); we take
 * the most recent `max` and reverse for display.
 */
export function RunCircles({ runs, max = 12 }: { runs: RunDot[]; max?: number }) {
  if (runs.length === 0) {
    return (
      <Text c="dimmed" size="xs">
        no recent runs
      </Text>
    );
  }
  const recent = runs.slice(0, max).reverse();
  return (
    <Group gap={4} wrap="nowrap">
      {recent.map((r) => (
        <Tooltip key={r.id} label={`${r.status}${whenLabel(r.started_at)}`} withArrow openDelay={150}>
          <span
            style={{
              width: 11,
              height: 11,
              borderRadius: "50%",
              background: `var(--mantine-color-${executionStatusColor(r.status)}-6)`,
              display: "inline-block",
              flex: "0 0 auto",
            }}
          />
        </Tooltip>
      ))}
    </Group>
  );
}
