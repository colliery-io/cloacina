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

/**
 * Query-hook convention (CLOACI-I-0117 / T-0651) the feature tasks follow.
 *
 * Rules:
 *  - All server access goes through `@cloacina/client` via `useClient()`.
 *  - Query keys are tenant-scoped through `queryKeys` so caches don't bleed
 *    across connections.
 *  - Errors are `CloacinaApiError`; `classifyError` (api/errors.ts) maps them
 *    to UI states. Retry policy lives in queryClient.ts.
 *
 * Example a feature task writes:
 *
 *   export function useWorkflows() {
 *     const client = useClient();
 *     const { tenant } = useAuth().connection!;
 *     return useQuery({
 *       queryKey: queryKeys.workflows(tenant),
 *       queryFn: () => client.listWorkflows(),
 *     });
 *   }
 */

export const queryKeys = {
  health: (server: string) => ["health", server] as const,
  workflows: (tenant: string) => ["workflows", tenant] as const,
  workflow: (tenant: string, name: string) => ["workflows", tenant, name] as const,
  executions: (tenant: string, query?: unknown) => ["executions", tenant, query] as const,
  execution: (tenant: string, id: string) => ["executions", tenant, id] as const,
  executionEvents: (tenant: string, id: string) => ["executions", tenant, id, "events"] as const,
  executionTasks: (tenant: string, id: string) => ["executions", tenant, id, "tasks"] as const,
  triggers: (tenant: string) => ["triggers", tenant] as const,
  trigger: (tenant: string, name: string) => ["triggers", tenant, name] as const,
  keys: (tenant: string) => ["keys", tenant] as const,
  accumulators: (tenant: string) => ["accumulators", tenant] as const,
  reactors: (tenant: string) => ["reactors", tenant] as const,
  graphs: (tenant: string) => ["graphs", tenant] as const,
} as const;
