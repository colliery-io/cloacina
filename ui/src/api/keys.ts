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

import type { schemas } from "@cloacina/client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useClient, useTenant } from "../auth/AuthContext";
import { queryKeys } from "./hooks";

export type KeyInfo = schemas["KeyInfo"];
export type KeyRole = schemas["KeyRole"];
export type KeyCreated = schemas["KeyCreatedResponse"];

/** Tenant-scoped API keys (T-0658 / REQ-006, UC-4). Never carries plaintext. */
export function useKeys() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.keys(tenant),
    // CLOACI-T-0784/0786: tenant-scoped key surface. The global /auth/keys is
    // now god-only; the UI always operates within its connected tenant, so it
    // uses the tenant endpoints (a god key can still reach any tenant this way).
    queryFn: () => client.listTenantKeys(),
  });
}

/**
 * Mint a tenant-scoped key. Resolves to `KeyCreatedResponse`, whose `key`
 * field is the one-time plaintext — the caller MUST show it once and drop it.
 * Never written to cache (mutation results aren't cached) or sessionStorage.
 */
export function useCreateKey() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, role }: { name: string; role: KeyRole }) =>
      client.createTenantKey({ name, role }),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.keys(tenant) }),
  });
}

/** Revoke a key by id. Invalidates the list so the revoked state shows. */
export function useRevokeKey() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (keyId: string) => client.revokeTenantKey(keyId),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.keys(tenant) }),
  });
}
