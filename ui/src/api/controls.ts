/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Operator-control mutations (CLOACI-I-0129) for endpoints the generated SDK
 *  doesn't wrap yet: workflow pause/resume (T-0749), reactor manual fire
 *  (T-0751), accumulator inject (T-0753), and the read-only workflow source
 *  view (T-0750). Raw fetch against the connection, mirroring the
 *  execution-tasks pattern in executions.ts.
 */
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useAuth, useTenant } from "../auth/AuthContext";
import { queryKeys } from "./hooks";

function base(url: string): string {
  return url.replace(/\/$/, "");
}

/** Pause or resume a workflow (CLOACI-T-0749). Refreshes the workflows list. */
export function usePauseWorkflow() {
  const { connection } = useAuth();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ name, paused }: { name: string; paused: boolean }) => {
      const action = paused ? "pause" : "resume";
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/tenants/${encodeURIComponent(tenant)}/workflows/${encodeURIComponent(name)}/${action}`,
        { method: "POST", headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`${action} failed (${res.status})`);
      return res.json().catch(() => ({}));
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.workflows(tenant) }),
  });
}

/** Enable/disable a schedule (CLOACI-T-0749 trigger pause/resume). Refreshes
 *  the trigger so the schedule card reflects the new state. */
export function useToggleTrigger() {
  const { connection } = useAuth();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ name, enabled }: { name: string; enabled: boolean }) => {
      const action = enabled ? "resume" : "pause";
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/tenants/${encodeURIComponent(tenant)}/triggers/${encodeURIComponent(name)}/${action}`,
        { method: "POST", headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`${action} failed (${res.status})`);
      return res.json().catch(() => ({}));
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.triggers(tenant) }),
  });
}

/** Fire a reactor: `force_fire` with the current cache, or `fire_with` typed
 *  per-source inputs (CLOACI-T-0751). */
export function useFireReactor() {
  const { connection } = useAuth();
  return useMutation({
    mutationFn: async (args: string | { name: string; mode?: "force_fire" | "fire_with"; inputs?: Record<string, unknown> }) => {
      const { name, mode = "force_fire", inputs } = typeof args === "string" ? { name: args } as { name: string; mode?: "force_fire" | "fire_with"; inputs?: Record<string, unknown> } : args;
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/reactors/${encodeURIComponent(name)}/fire`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${connection!.apiKey}`, "Content-Type": "application/json" },
          body: JSON.stringify({ mode, inputs: inputs ?? {} }),
        },
      );
      if (!res.ok) throw new Error(`fire failed (${res.status})`);
      return res.json().catch(() => ({}));
    },
  });
}

/** One declared input slot (CLOACI-I-0128). `schema` is JSON-Schema; an object
 *  schema carries `properties` for per-field forms. */
export interface InterfaceSlot {
  name: string;
  schema: { type?: string; properties?: Record<string, { type?: string }>; required?: string[] } | null;
  required: boolean;
}
export interface DeclaredSurface {
  kind: string;
  name: string;
  slots: InterfaceSlot[];
}

/** Declared input interface for a reactor (fire-with slots) or accumulator
 *  (inject slots), CLOACI-I-0128 / T-0758. */
function useInterface(kind: "reactors" | "accumulators", name: string | null | undefined, enabled: boolean) {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["surface-interface", kind, name],
    enabled: enabled && !!connection && !!name,
    queryFn: async (): Promise<DeclaredSurface> => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/${kind}/${encodeURIComponent(name!)}/interface`,
        { headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`interface failed (${res.status})`);
      return res.json();
    },
  });
}
export const useReactorInterface = (name: string | null | undefined, opts?: { enabled?: boolean }) =>
  useInterface("reactors", name, opts?.enabled ?? true);
export const useAccumulatorInterface = (name: string | null | undefined, opts?: { enabled?: boolean }) =>
  useInterface("accumulators", name, opts?.enabled ?? true);

/** Inject a single typed event into an accumulator (CLOACI-T-0753). */
export function useInjectAccumulator() {
  const { connection } = useAuth();
  return useMutation({
    mutationFn: async ({ name, event }: { name: string; event: unknown }) => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/accumulators/${encodeURIComponent(name)}/inject`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${connection!.apiKey}`, "Content-Type": "application/json" },
          body: JSON.stringify({ event }),
        },
      );
      if (!res.ok) throw new Error(`inject failed (${res.status})`);
      return res.json().catch(() => ({}));
    },
  });
}

/** One recorded reactor fire (CLOACI-T-0766). */
export interface ReactorFire {
  fired_at: string;
  ok: boolean;
  error: string | null;
  duration_ms: number;
}

/** Recent fires for a reactor (CLOACI-T-0766), newest first. Polls at 5s while
 *  the graph detail is open so the recent-fires + failure count stay live. */
export function useReactorFires(reactor: string | null | undefined, opts?: { limit?: number; poll?: boolean }) {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["reactor-fires", reactor, opts?.limit ?? 50],
    enabled: !!connection && !!reactor,
    refetchInterval: opts?.poll ? 5000 : false,
    queryFn: async (): Promise<{ items: ReactorFire[] }> => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/reactors/${encodeURIComponent(reactor!)}/fires?limit=${opts?.limit ?? 50}`,
        { headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`Failed to load fires (${res.status})`);
      return res.json();
    },
  });
}

/** Per-minute fire counts (last 60 min) for the fire-activity heatmap (T-0766). */
export function useReactorFireTimeseries(reactor: string | null | undefined, opts?: { poll?: boolean }) {
  const { connection } = useAuth();
  return useQuery({
    queryKey: ["reactor-fire-timeseries", reactor],
    enabled: !!connection && !!reactor,
    refetchInterval: opts?.poll ? 5000 : false,
    queryFn: async (): Promise<{ buckets: number[] }> => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/reactors/${encodeURIComponent(reactor!)}/fires/timeseries`,
        { headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`Failed to load fire timeseries (${res.status})`);
      return res.json();
    },
  });
}

/** A source file from a package's retained `.cloacina` archive (T-0750). */
export interface WorkflowSourceFile {
  path: string;
  contents: string;
}

/** Read-only source files for a workflow package (CLOACI-T-0750). */
export function useWorkflowSource(packageName: string, opts?: { enabled?: boolean }) {
  const { connection } = useAuth();
  const tenant = useTenant();
  return useQuery({
    queryKey: ["workflow-source", tenant, packageName],
    enabled: (opts?.enabled ?? true) && !!packageName && !!connection,
    queryFn: async (): Promise<{ files: WorkflowSourceFile[] }> => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/tenants/${encodeURIComponent(tenant)}/workflows/${encodeURIComponent(packageName)}/source`,
        { headers: { Authorization: `Bearer ${connection!.apiKey}` } },
      );
      if (!res.ok) throw new Error(`Failed to load source (${res.status})`);
      return res.json();
    },
  });
}
