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

import { MONO } from "@colliery-io/aurora-dark";
import { Box, Group, Text, Tooltip } from "@mantine/core";
import { type CSSProperties } from "react";

import type { RunTimeline } from "../api/executions";
import type { TaskGraphNode } from "../util/topo";
import { topoRank } from "../util/topo";
import { formatMs } from "../util/format";

/** Five-number summary of a sample (linear-interpolated quantiles). */
function summary(xs: number[]) {
  const s = [...xs].sort((a, b) => a - b);
  const n = s.length;
  const q = (p: number) => {
    if (n === 0) return 0;
    const i = (n - 1) * p;
    const lo = Math.floor(i);
    const hi = Math.ceil(i);
    return s[lo] + (s[hi] - s[lo]) * (i - lo);
  };
  return { n, min: s[0] ?? 0, med: q(0.5), max: s[n - 1] ?? 0 };
}

type Row = {
  name: string;
  /** Runs that contributed timing for this task. */
  n: number;
  startMed: number;
  endMed: number;
  startMin: number;
  startMax: number;
  endMin: number;
  endMax: number;
  durMed: number;
  durMin: number;
  durMax: number;
  waitMed: number;
  waitMin: number;
  waitMax: number;
};

/**
 * Combined-timeline distribution (CLOACI-I-0124 — the requested aggregate of
 * many per-run Gantts). Every run is aligned at t=0 (its earliest task start),
 * and each task becomes one box-and-whisker row on a shared time axis:
 *
 *   - the solid blue box is the task's *median span* (median start → median
 *     end across runs);
 *   - the thin lines through its left and right edges are the min–max spread of
 *     the start edge and the end edge — i.e. how much the task's start and
 *     finish jitter run to run;
 *   - the grey box to its left is the *inter-task wait*: the median gap between
 *     the task's dependencies completing and the task starting (scheduler/queue
 *     latency), with its own min–max whisker. Root tasks measure the wait from
 *     the run's start.
 *
 * A task whose grey box is fat is waiting on the scheduler; a task whose blue
 * box has long edge-whiskers has unstable duration. The DAG (`graph`) supplies
 * both the row order (topological) and the dependency edges used for the wait.
 */
export function CombinedTimeline({
  runs,
  graph,
}: {
  runs: RunTimeline[];
  graph?: TaskGraphNode[] | null;
}) {
  if (runs.length === 0) {
    return (
      <Text size="sm" c="dimmed">
        No completed runs with task timing in the sampled window yet.
      </Text>
    );
  }

  const rank = graph && graph.length ? topoRank(graph) : undefined;
  const deps = new Map<string, string[]>();
  if (graph) for (const node of graph) deps.set(node.id, node.dependencies ?? []);

  // Every task that appears in any run, in DAG order (fallback: name order).
  const names = [...new Set(runs.flatMap((r) => Object.keys(r.tasks)))].sort((a, b) => {
    const ar = rank?.get(a) ?? Number.MAX_SAFE_INTEGER;
    const br = rank?.get(b) ?? Number.MAX_SAFE_INTEGER;
    return ar - br || (a < b ? -1 : 1);
  });

  const rows: Row[] = names.map((name) => {
    const starts: number[] = [];
    const ends: number[] = [];
    const durs: number[] = [];
    const waits: number[] = [];
    const predNames = deps.get(name) ?? [];
    for (const run of runs) {
      const t = run.tasks[name];
      if (!t) continue;
      const so = t.start - run.t0;
      const eo = t.end - run.t0;
      starts.push(so);
      ends.push(eo);
      durs.push(t.end - t.start);
      if (predNames.length === 0) {
        // Root task: wait = time from run start to this task starting.
        waits.push(Math.max(0, so));
      } else {
        let predEnd = -Infinity;
        let complete = true;
        for (const d of predNames) {
          const pt = run.tasks[d];
          if (!pt) {
            complete = false;
            break;
          }
          predEnd = Math.max(predEnd, pt.end - run.t0);
        }
        if (complete) waits.push(Math.max(0, so - predEnd));
      }
    }
    const st = summary(starts);
    const en = summary(ends);
    const du = summary(durs);
    const wa = summary(waits);
    return {
      name,
      n: st.n,
      startMed: st.med,
      startMin: st.min,
      startMax: st.max,
      endMed: en.med,
      endMin: en.min,
      endMax: en.max,
      durMed: du.med,
      durMin: du.min,
      durMax: du.max,
      waitMed: wa.med,
      waitMin: wa.min,
      waitMax: wa.max,
    };
  });

  // Shared scale: the latest any task finishes across all runs.
  const domain = Math.max(...rows.map((r) => r.endMax), 1);
  const pct = (ms: number) => `${(Math.max(0, ms) / domain) * 100}%`;
  const width = (ms: number) => `${(Math.max(0, ms) / domain) * 100}%`;

  const GRID = "200px 1fr 52px 52px";
  const th: CSSProperties = { fontFamily: MONO, fontSize: 9, letterSpacing: ".06em", textTransform: "uppercase", color: "var(--faint)" };

  return (
    <Box>
      <Box style={{ display: "grid", gridTemplateColumns: GRID, gap: 8, paddingBottom: 8 }}>
        <span style={th} />
        <span style={th} />
        <span style={{ ...th, textAlign: "right" }}>Span</span>
        <span style={{ ...th, textAlign: "right" }}>Wait</span>
      </Box>
      <Box style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {rows.map((r) => {
          // Median span box; if it would be sub-pixel, give it a floor so the
          // bar is still visible.
          const spanW = Math.max(r.endMed - r.startMed, domain * 0.005);
          // Scheduler wait: from dep-completion (startMed - waitMed) to start.
          const waitLeft = Math.max(0, r.startMed - r.waitMed);
          return (
            <Box
              key={r.name}
              style={{ display: "grid", gridTemplateColumns: GRID, alignItems: "center", gap: 8 }}
            >
              <Tooltip label={`${r.name} · ${r.n} run${r.n === 1 ? "" : "s"}`} withArrow openDelay={300} position="left">
                <Text size="xs" truncate fw={500}>
                  {r.name}
                </Text>
              </Tooltip>

              <Tooltip
                label={
                  `span p50 ${formatMs(r.durMed)} (${formatMs(r.durMin)}–${formatMs(r.durMax)}) · ` +
                  `scheduler wait (deps done → start) p50 ${formatMs(r.waitMed)}`
                }
                withArrow
                openDelay={150}
                position="top"
              >
                <Box style={{ position: "relative", height: 18 }}>
                  {/* baseline */}
                  <Box style={{ position: "absolute", inset: "8px 0", height: 1, background: "var(--border-soft)" }} />
                  {/* scheduler wait box: deps-done → task-start (queue latency) */}
                  {r.waitMed > 0 && (
                    <Box
                      style={{
                        position: "absolute",
                        left: pct(waitLeft),
                        width: width(r.waitMed),
                        top: 5,
                        height: 9,
                        background: "#2a3340",
                        borderRadius: 2,
                      }}
                    />
                  )}
                  {/* typical (median) span, solid, on top */}
                  <Box
                    style={{
                      position: "absolute",
                      left: pct(r.startMed),
                      width: width(spanW),
                      top: 3,
                      height: 11,
                      background: "#7fb2ff",
                      borderRadius: 3,
                    }}
                  />
                </Box>
              </Tooltip>

              <Text size="xs" c="dimmed" ta="right" style={{ whiteSpace: "nowrap", fontFamily: MONO }}>
                {formatMs(r.durMed)}
              </Text>
              <Text size="xs" ta="right" style={{ whiteSpace: "nowrap", fontFamily: MONO, color: r.waitMed >= 1.5 ? "#d8a657" : "var(--faint)" }}>
                {formatMs(r.waitMed)}
              </Text>
            </Box>
          );
        })}
      </Box>

      <Group gap="md" mt="sm">
        <Legend swatch="#7fb2ff" label="typical span" />
        <Legend swatch="#2a3340" label="scheduler wait (deps done → task start)" />
        <Text size="xs" c="dimmed">
          a fat grey box = queue latency before the task ran · {runs.length} run{runs.length === 1 ? "" : "s"}
        </Text>
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
