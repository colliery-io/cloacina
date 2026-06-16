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

import { useQuery } from "@tanstack/react-query";

import { useAuth } from "../auth/AuthContext";

/**
 * Operations/health hooks (CLOACI-I-0124 / WS-2). These hit operator endpoints
 * the generated SDK doesn't expose yet, so we call them directly off the active
 * connection. All poll on a slow interval — ops state changes slowly.
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

/** Execution-agent fleet roster (`GET /v1/agents`, admin). */
export function useFleet() {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["ops", "fleet", connection?.serverUrl],
    enabled: !!connection,
    refetchInterval: POLL,
    queryFn: async (): Promise<AgentInfo[]> => {
      const res = await fetch(`${base(connection!.serverUrl)}/v1/agents`, {
        headers: { Authorization: `Bearer ${connection!.apiKey}` },
      });
      if (!res.ok) throw new Error(`Failed to load fleet (${res.status})`);
      const body = await res.json();
      return (body.items ?? body) as AgentInfo[];
    },
  });
}

/** Compiler / build-pipeline status (`GET /v1/compiler/status`, admin). */
export function useCompilerStatus() {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["ops", "compiler", connection?.serverUrl],
    enabled: !!connection,
    refetchInterval: POLL,
    queryFn: async (): Promise<CompilerStatus> => {
      const res = await fetch(`${base(connection!.serverUrl)}/v1/compiler/status`, {
        headers: { Authorization: `Bearer ${connection!.apiKey}` },
      });
      if (!res.ok) throw new Error(`Failed to load compiler status (${res.status})`);
      return res.json();
    },
  });
}
