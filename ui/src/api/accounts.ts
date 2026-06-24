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

export type AccountInfo = schemas["AccountInfo"];

/** Tenant-admin local accounts (CLOACI-T-0797/0798). Tenant-scoped; the
 *  server returns 403 for a non-admin key (rendered as an error state). */
export function useAccounts() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: ["accounts", tenant],
    queryFn: () => client.listAccounts(),
  });
}

export function useCreateAccount() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: schemas["CreateAccountRequest"]) => client.createAccount(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["accounts", tenant] }),
  });
}

export function useDisableAccount() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (accountId: string) => client.disableAccount(accountId),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["accounts", tenant] }),
  });
}

export function useResetPassword() {
  const client = useClient();
  return useMutation({
    mutationFn: ({ accountId, password }: { accountId: string; password: string }) =>
      client.resetAccountPassword(accountId, { password }),
  });
}
