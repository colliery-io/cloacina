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

import { useClient, useTenant } from "../auth/AuthContext";
import { queryKeys } from "./hooks";

export type ExecutionsQuery = {
  status?: string;
  workflow?: string;
  limit?: number;
  offset?: number;
};

/** Executions list page for the active tenant (T-0653). */
export function useExecutions(query: ExecutionsQuery) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.executions(tenant, query),
    queryFn: () => client.listExecutions(query),
    placeholderData: (prev) => prev, // keep the page visible while paging/filtering
  });
}

/** Single execution detail (T-0653). */
export function useExecution(id: string) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.execution(tenant, id),
    queryFn: () => client.getExecution(id),
  });
}

/** Execution event log from the REST endpoint (T-0653; T-0656 adds the live tail). */
export function useExecutionEvents(id: string) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.executionEvents(tenant, id),
    queryFn: () => client.getExecutionEvents(id),
  });
}
