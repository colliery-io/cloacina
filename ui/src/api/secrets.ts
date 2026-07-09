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

import type { CreateSecretBody, RotateSecretBody, SecretMetadata } from "@cloacina/client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useClient, useTenant } from "../auth/AuthContext";

export type { SecretMetadata };

/** Tenant secrets (CLOACI-I-0133 / T-0862). Tenant-scoped; the server returns
 *  403 for a non-admin key (rendered as an error state). Metadata only — a
 *  value is NEVER returned by list/get. */
export function useSecrets() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: ["secrets", tenant],
    queryFn: () => client.listSecrets(),
  });
}

export function useCreateSecret() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateSecretBody) => client.createSecret(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secrets", tenant] }),
  });
}

export function useRotateSecret() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, body }: { name: string; body: RotateSecretBody }) =>
      client.rotateSecret(name, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secrets", tenant] }),
  });
}

export function useDeleteSecret() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (name: string) => client.deleteSecret(name),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["secrets", tenant] }),
  });
}
