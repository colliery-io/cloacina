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

import { followExecutionEvents } from "@cloacina/client";
import { useQueries, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import type { ExecutionEvent } from "../components/EventLog";
import { useAuth, useClient, useTenant } from "../auth/AuthContext";
import { isTerminalStatus } from "../util/status";
import { queryKeys } from "./hooks";

/** One per-task row of an execution (CLOACI-I-0124 / WS-1; `/executions/{id}/tasks`). */
export type TaskExecutionDetail = {
  id: string;
  task_name: string;
  status: string;
  started_at: string | null;
  completed_at: string | null;
  attempt: number;
  max_attempts: number;
  created_at: string;
  updated_at: string;
  sub_status: string | null;
  last_error: string | null;
  error_details: string | null;
};

type ExecutionTasksResponse = {
  tenant_id: string;
  execution_id: string;
  tasks: TaskExecutionDetail[];
};

type Connection = { serverUrl: string; apiKey: string };

/**
 * Fetch the per-task rows for one execution. Shared by the single-execution
 * view (`useExecutionTasks`) and the cross-run aggregate
 * (`useWorkflowTaskRuntimes`) so both populate the same query cache. The
 * generated SDK doesn't expose this endpoint yet, so we call it directly.
 */
async function fetchExecutionTasks(
  connection: Connection,
  tenant: string,
  id: string,
): Promise<ExecutionTasksResponse> {
  const base = connection.serverUrl.replace(/\/$/, "");
  const res = await fetch(
    `${base}/v1/tenants/${encodeURIComponent(tenant)}/executions/${encodeURIComponent(id)}/tasks`,
    { headers: { Authorization: `Bearer ${connection.apiKey}` } },
  );
  if (!res.ok) throw new Error(`Failed to load tasks (${res.status})`);
  return res.json();
}

/**
 * Per-task rows for an execution (CLOACI-I-0124 / WS-1). `poll` re-fetches
 * every 2s while the run is in progress.
 */
export function useExecutionTasks(id: string, opts: { poll?: boolean } = {}) {
  const { connection } = useAuth();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.executionTasks(tenant, id),
    enabled: !!connection && !!id,
    queryFn: () => fetchExecutionTasks(connection!, tenant, id),
    refetchInterval: opts.poll ? 2000 : false,
  });
}

/** Aggregate per-task duration across the last N runs of a workflow. */
export type TaskRuntimeStat = {
  /** Local task id (last `::` segment). */
  taskName: string;
  /** Runs that contributed a completed duration for this task. */
  count: number;
  avgMs: number;
  minMs: number;
  maxMs: number;
  /** Duration in the most recent run, or null if it didn't run/complete there. */
  lastMs: number | null;
};

/** A completed task contributes a duration; running/missing rows are skipped. */
function taskDurationMs(t: TaskExecutionDetail): number | null {
  if (!t.started_at || !t.completed_at) return null;
  const s = Date.parse(t.started_at);
  const e = Date.parse(t.completed_at);
  if (Number.isNaN(s) || Number.isNaN(e) || e < s) return null;
  return e - s;
}

const localTaskId = (name: string) => name.split("::").pop() || name;

/**
 * One run's normalized task timing, for the combined-timeline distribution
 * view. `start`/`end` are absolute epoch-ms; offsets are taken against `t0`
 * (the run's earliest task start) by the consumer so runs align at zero.
 */
export type RunTimeline = {
  id: string;
  status: string;
  t0: number;
  /** Local task id → absolute start/end ms (only completed tasks). */
  tasks: Record<string, { start: number; end: number }>;
};

/**
 * Per-task runtime statistics + normalized per-run timelines across the most
 * recent runs of a workflow (CLOACI-I-0124). Lists the recent executions, fans
 * out a task fetch per run (shared cache with the detail view), and reduces to:
 *   - `stats`: mean/min/max duration per task (the "Task Duration" bar chart);
 *     the newest run's duration is surfaced as `lastMs` so a regression stands
 *     out against the mean. Unsorted; the caller orders by DAG rank.
 *   - `runs`: each run's per-task start/end, for the combined box-and-whisker
 *     timeline (start/end-edge jitter + inter-task wait distribution).
 */
export function useWorkflowTaskRuntimes(workflow: string, opts: { runs?: number } = {}) {
  const { connection } = useAuth();
  const tenant = useTenant();
  const limit = opts.runs ?? 20;

  const list = useExecutions({ workflow: workflow || undefined, limit });
  const items = list.data?.items ?? [];
  const ids = items.map((e) => e.id);

  const results = useQueries({
    queries: ids.map((id) => ({
      queryKey: queryKeys.executionTasks(tenant, id),
      enabled: !!connection && !!workflow && !!id,
      queryFn: () => fetchExecutionTasks(connection!, tenant, id),
      staleTime: 30_000,
    })),
  });

  // Accumulate durations per local task name, and build the per-run timeline.
  // The first id is the most recent run (the list is newest-first), so its
  // durations become `lastMs`.
  const acc = new Map<string, { sum: number; min: number; max: number; count: number; last: number | null }>();
  const runs: RunTimeline[] = [];
  results.forEach((r, runIdx) => {
    const tasks: Record<string, { start: number; end: number }> = {};
    let t0 = Infinity;
    for (const t of r.data?.tasks ?? []) {
      const ms = taskDurationMs(t);
      if (ms == null) continue;
      const key = localTaskId(t.task_name);
      const cur = acc.get(key) ?? { sum: 0, min: Infinity, max: 0, count: 0, last: null };
      cur.sum += ms;
      cur.min = Math.min(cur.min, ms);
      cur.max = Math.max(cur.max, ms);
      cur.count += 1;
      if (runIdx === 0) cur.last = ms;
      acc.set(key, cur);

      const start = Date.parse(t.started_at as string);
      const end = Date.parse(t.completed_at as string);
      tasks[key] = { start, end };
      if (start < t0) t0 = start;
    }
    if (Object.keys(tasks).length > 0) {
      runs.push({ id: ids[runIdx], status: items[runIdx]?.status ?? "", t0, tasks });
    }
  });

  const stats: TaskRuntimeStat[] = [...acc.entries()].map(([taskName, a]) => ({
    taskName,
    count: a.count,
    avgMs: a.sum / a.count,
    minMs: a.min,
    maxMs: a.max,
    lastMs: a.last,
  }));

  return {
    stats,
    runs,
    runsCounted: ids.length,
    isPending: list.isPending || results.some((r) => r.isPending),
    isError: list.isError || results.some((r) => r.isError),
  };
}

export type ExecutionsQuery = {
  status?: string;
  workflow?: string;
  limit?: number;
  offset?: number;
};

/** Executions list page for the active tenant (T-0653). */
export function useExecutions(query: ExecutionsQuery) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.executions(tenant, query),
    queryFn: () => client.listExecutions(query),
    placeholderData: (prev) => prev, // keep the page visible while paging/filtering
  });
}

/**
 * Single execution detail (T-0653). With `livePoll`, the status is re-polled
 * every 2s while the run is non-terminal (T-0656) so the badge transitions
 * to its terminal state and the live stream can be torn down; polling stops
 * once terminal.
 */
export function useExecution(id: string, opts: { livePoll?: boolean } = {}) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.execution(tenant, id),
    queryFn: () => client.getExecution(id),
    refetchInterval: opts.livePoll
      ? (query) => (query.state.data && !isTerminalStatus(query.state.data.status) ? 2000 : false)
      : false,
  });
}

/** Execution event log from the REST endpoint (T-0653; the live tail is `useLiveExecutionEvents`). */
export function useExecutionEvents(id: string) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.executionEvents(tenant, id),
    queryFn: () => client.getExecutionEvents(id),
  });
}

/**
 * Live tail over the delivery WS (T-0656 / REQ-004 live half). Bridges the
 * SDK's `followExecutionEvents` async iterator into component state while
 * `enabled` (i.e. the run is in progress), aborting the WS cleanly on
 * unmount or when the execution goes terminal — no leaked sockets (NFR-002).
 *
 * Returns the live events only; the caller merges them with the REST
 * history via `mergeEvents` (the OQ-6 seam — dedup on sequence_num). Dedup
 * here within the live stream is also by sequence_num; reconnect + the
 * at-least-once redelivery are the SDK's responsibility.
 */
export function useLiveExecutionEvents(id: string, enabled: boolean): ExecutionEvent[] {
  const client = useClient();
  const [events, setEvents] = useState<ExecutionEvent[]>([]);

  useEffect(() => {
    setEvents([]); // reset when the execution or enabled-state changes
    if (!enabled) return;

    const controller = new AbortController();
    (async () => {
      try {
        for await (const ev of followExecutionEvents(client, id, {
          signal: controller.signal,
        })) {
          const e = ev as ExecutionEvent;
          if (e && typeof e.sequence_num === "number") {
            setEvents((prev) =>
              prev.some((p) => p.sequence_num === e.sequence_num) ? prev : [...prev, e],
            );
          }
        }
      } catch {
        // Aborted on unmount, or a terminal stream error — the REST history
        // (shown alongside) remains the source of truth.
      }
    })();

    return () => controller.abort();
  }, [client, id, enabled]);

  return events;
}
