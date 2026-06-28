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

import { CloacinaApiError, type ErrorBody } from "@cloacina/client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useAuth, type Connection } from "../auth/AuthContext";

/** Tenant agent-fleet state (CLOACI-T-0809): desired (provisioned) vs actual
 *  (running) agents, against the tenant's effective per-tenant limit. */
export interface FleetState {
  tenant_id: string;
  desired_count: number;
  actual_count: number;
  effective_limit: number;
  default_max_agents: number;
}

/** Tenant agent limit (CLOACI-T-0808): the platform default, any tenant
 *  override, and the effective limit (override ?? default). Read-only. */
export interface TenantLimit {
  tenant_id: string;
  default_max_agents: number;
  tenant_override: number | null;
  effective_limit: number;
}

/**
 * Hand-written fetch against the fleet/limits endpoints (CLOACI-T-0808/0809),
 * which aren't in the generated `@cloacina/client` paths yet. Reuses the exact
 * auth mechanism the generated client uses — the active connection's bearer
 * key (`Authorization: Bearer <apiKey>`) against its `serverUrl` — and throws a
 * {@link CloacinaApiError} on non-2xx so `classifyError` (and the 409 capacity
 * check) work identically to the generated helpers.
 */
async function fleetFetch<T>(conn: Connection, method: string, path: string): Promise<T> {
  const url = `${conn.serverUrl.replace(/\/+$/, "")}${path}`;
  const res = await fetch(url, {
    method,
    headers: { authorization: `Bearer ${conn.apiKey}` },
  });
  if (!res.ok) {
    let body: ErrorBody | undefined;
    try {
      body = (await res.json()) as ErrorBody;
    } catch {
      // non-JSON error body — surface the bare status
    }
    throw new CloacinaApiError(res.status, body);
  }
  return (await res.json()) as T;
}

/** GET the tenant's fleet state. Tenant-scoped; a non-admin key still reads it
 *  (controls are gated separately in the view). */
export function useFleet() {
  const { connection } = useAuth();
  const tenant = connection?.tenant;
  return useQuery({
    queryKey: ["fleet", tenant],
    queryFn: () => fleetFetch<FleetState>(connection!, "GET", `/v1/tenants/${tenant}/fleet`),
    enabled: connection != null,
  });
}

/** GET the tenant's effective agent limit (read-only display). */
export function useTenantLimit() {
  const { connection } = useAuth();
  const tenant = connection?.tenant;
  return useQuery({
    queryKey: ["fleet-limit", tenant],
    queryFn: () => fleetFetch<TenantLimit>(connection!, "GET", `/v1/tenants/${tenant}/limits`),
    enabled: connection != null,
  });
}

/** Provision +1 agent (tenant-admin). Returns 409 when already at the
 *  effective limit — surfaced as an "at capacity" state by the caller. */
export function useProvision() {
  const { connection } = useAuth();
  const tenant = connection?.tenant;
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () =>
      fleetFetch<FleetState>(connection!, "POST", `/v1/tenants/${tenant}/fleet/provision`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["fleet", tenant] }),
  });
}

/** Deprovision −1 agent (tenant-admin); floors at 0. */
export function useDeprovision() {
  const { connection } = useAuth();
  const tenant = connection?.tenant;
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () =>
      fleetFetch<FleetState>(connection!, "POST", `/v1/tenants/${tenant}/fleet/deprovision`),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["fleet", tenant] }),
  });
}
