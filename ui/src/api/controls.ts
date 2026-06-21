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

/** Force-fire a reactor with its current cache (CLOACI-T-0751). */
export function useFireReactor() {
  const { connection } = useAuth();
  return useMutation({
    mutationFn: async (name: string) => {
      const res = await fetch(
        `${base(connection!.serverUrl)}/v1/health/reactors/${encodeURIComponent(name)}/fire`,
        {
          method: "POST",
          headers: { Authorization: `Bearer ${connection!.apiKey}`, "Content-Type": "application/json" },
          body: JSON.stringify({ mode: "force_fire" }),
        },
      );
      if (!res.ok) throw new Error(`fire failed (${res.status})`);
      return res.json().catch(() => ({}));
    },
  });
}

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
