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

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { useClient, useTenant } from "../auth/AuthContext";
import { queryKeys } from "./hooks";

/** Workflows list for the active tenant (T-0652). */
export function useWorkflows() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.workflows(tenant),
    queryFn: () => client.listWorkflows(),
  });
}

/** Single workflow detail by package name (T-0652). `enabled` lets callers
 *  defer the fetch until the package name is known (CLOACI-I-0124 / WS-1). */
export function useWorkflow(name: string, opts?: { enabled?: boolean }) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.workflow(tenant, name),
    queryFn: () => client.getWorkflow(name),
    enabled: opts?.enabled ?? true,
  });
}

// ---- write ops (T-0657) ----

/** Upload a `.cloacina` package (multipart). Invalidates the workflows list. */
export function useUploadWorkflow() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (file: File) => client.uploadWorkflow(file),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.workflows(tenant) }),
  });
}

/** Execute a workflow with optional JSON context. Returns the accepted execution. */
export function useExecuteWorkflow() {
  const client = useClient();
  return useMutation({
    mutationFn: ({ name, context }: { name: string; context?: unknown }) =>
      client.executeWorkflow(name, context === undefined ? {} : { context }),
  });
}

/** Unregister a workflow (idempotent server-side). Invalidates the list. */
export function useDeleteWorkflow() {
  const client = useClient();
  const tenant = useTenant();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ name, version }: { name: string; version: string }) =>
      client.deleteWorkflow(name, version),
    onSuccess: () => qc.invalidateQueries({ queryKey: queryKeys.workflows(tenant) }),
  });
}
