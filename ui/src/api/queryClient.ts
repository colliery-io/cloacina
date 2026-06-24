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

import { classifyError } from "@colliery-io/aurora-dark";
import { QueryClient } from "@tanstack/react-query";


/**
 * Shared TanStack Query client (T-0651). Don't retry auth/validation/
 * not-found errors — they won't resolve by retrying; only retry transient
 * server/network failures, once.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        const kind = classifyError(error).kind;
        if (kind === "server" || kind === "network") return failureCount < 1;
        return false;
      },
      staleTime: 10_000,
      refetchOnWindowFocus: false,
    },
  },
});
