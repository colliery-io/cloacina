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

/** Computation-graph health hooks (T-0655). The /v1/health/* endpoints are
 *  not tenant-pathed (filtered by the key server-side), but we key the cache
 *  by tenant so different connections don't share results. */

export function useAccumulators() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.accumulators(tenant),
    queryFn: () => client.listAccumulators(),
  });
}

export function useGraphs() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.graphs(tenant),
    queryFn: () => client.listGraphs(),
    // Poll so the operational metrics (fires / throughput / last-fired) stay
    // live and the UI can derive recent throughput from successive samples.
    refetchInterval: 5000,
  });
}

/** Reactors visible to the key (CLOACI-T-0742). Reactor-first: includes
 *  reactors with no graph bound (which the graphs list omits). Polled like
 *  graphs so fire counters / throughput stay live. */
export function useReactors() {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: queryKeys.reactors(tenant),
    queryFn: () => client.listReactors(),
    refetchInterval: 5000,
  });
}

export function useGraph(name: string) {
  const client = useClient();
  const tenant = useTenant();
  return useQuery({
    queryKey: [...queryKeys.graphs(tenant), name],
    queryFn: () => client.getGraph(name),
  });
}
