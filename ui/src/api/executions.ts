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
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import type { ExecutionEvent } from "../components/EventLog";
import { useClient, useTenant } from "../auth/AuthContext";
import { isTerminalStatus } from "../util/status";
import { queryKeys } from "./hooks";

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
