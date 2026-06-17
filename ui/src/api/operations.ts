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

import { followOpsMetrics } from "@cloacina/client";
import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { useAuth, useClient } from "../auth/AuthContext";

/**
 * Operations/health hooks (CLOACI-I-0124 / WS-2). The Operations page consumes
 * `useLiveOpsMetrics` — a single WS subscription pushed from the server
 * (CLOACI-T-0718) — instead of per-tile polling. `useServerHealth` remains a
 * light poll for the always-on header liveness dot.
 */

export type AgentInfo = {
  agent_id: string;
  target_triple: string;
  max_concurrency: number;
  in_flight: number;
  available_capacity: number;
  seconds_since_heartbeat: number | null;
  capabilities: string[];
  tenant_id: string | null;
};

export type CompilerStatus = {
  status: string;
  pending: number;
  building: number;
  seconds_since_heartbeat: number | null;
  last_success_at: string | null;
  last_failure_at: string | null;
};

export type ServerHealth = {
  /** `/health` reachable (process alive). */
  alive: boolean;
  /** `/ready` returned 200 (DB pool reachable + no crashed graph). */
  ready: boolean;
  /** 503 reason from `/ready`, when not ready. */
  reason: string | null;
};

export type ReconcilerStatus = {
  /** `"ok"` (no build failures) or `"errors"`. */
  status: string;
  /** Packages built successfully and available to load. */
  built: number;
  /** Packages whose latest build failed. */
  failed: number;
  /** RFC 3339 timestamp of the most recent successful build. */
  last_built_at: string | null;
};

/** One operational-metrics snapshot pushed over WS (CLOACI-T-0718). */
export type OpsMetrics = {
  server: ServerHealth;
  compiler: CompilerStatus;
  fleet: AgentInfo[];
  reconciler: ReconcilerStatus;
  ts: string;
};

function base(serverUrl: string): string {
  return serverUrl.replace(/\/$/, "");
}

const POLL = 5000;

/** Server liveness + readiness (the public `/health` + `/ready` probes). */
export function useServerHealth() {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["ops", "server", connection?.serverUrl],
    enabled: !!connection,
    refetchInterval: POLL,
    queryFn: async (): Promise<ServerHealth> => {
      const root = base(connection!.serverUrl);
      let alive = false;
      try {
        const h = await fetch(`${root}/health`);
        alive = h.ok;
      } catch {
        alive = false;
      }
      let ready = false;
      let reason: string | null = null;
      try {
        const r = await fetch(`${root}/ready`);
        ready = r.ok;
        if (!r.ok) {
          const body = await r.json().catch(() => null);
          reason = body?.reason ?? `HTTP ${r.status}`;
        }
      } catch {
        reason = "unreachable";
      }
      return { alive, ready, reason };
    },
  });
}

/**
 * Live operational metrics over WS (CLOACI-T-0718) — the Operations page's
 * single source for the compiler / fleet / reconciler / server tiles, replacing
 * the per-tile pollers. The server pushes a fresh snapshot every few seconds
 * while subscribed; this returns the latest, or `null` until the first arrives
 * (and across a reconnect the SDK re-subscribes automatically). `enabled` gates
 * the subscription to when the page is mounted.
 */
export function useLiveOpsMetrics(enabled: boolean): OpsMetrics | null {
  const client = useClient();
  const [metrics, setMetrics] = useState<OpsMetrics | null>(null);

  useEffect(() => {
    if (!enabled) return;
    setMetrics(null);
    const controller = new AbortController();
    (async () => {
      try {
        for await (const ev of followOpsMetrics(client, { signal: controller.signal })) {
          setMetrics(ev as OpsMetrics);
        }
      } catch {
        // Aborted on unmount, or a terminal stream error — the SDK reconnects
        // internally; on unmount we simply stop.
      }
    })();
    return () => controller.abort();
  }, [client, enabled]);

  return metrics;
}
