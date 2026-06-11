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
 * Typed error classification (CLOACI-I-0117 / REQ-007). Maps the SDK's
 * `CloacinaApiError` (and anything else) onto an actionable UI kind so views
 * render the right state instead of a generic failure.
 */

import { CloacinaApiError } from "@cloacina/client";

export type ErrorKind =
  | "auth" // 401/403 — re-authenticate
  | "notfound" // 404 — not-found view
  | "validation" // 400/422 — inline, carries server `code`
  | "server" // 5xx — retryable
  | "network" // transport failure (server unreachable)
  | "unknown";

export interface ClassifiedError {
  kind: ErrorKind;
  /** Human-readable message (server `error` field when available). */
  message: string;
  /** Machine-readable server `code`, when present. */
  code?: string;
  status?: number;
}

export function classifyError(err: unknown): ClassifiedError {
  if (err instanceof CloacinaApiError) {
    const status = err.status;
    let kind: ErrorKind = "unknown";
    if (status === 401 || status === 403) kind = "auth";
    else if (status === 404) kind = "notfound";
    else if (status === 400 || status === 422) kind = "validation";
    else if (status >= 500) kind = "server";
    return { kind, message: err.message, code: err.code, status };
  }
  if (err instanceof TypeError) {
    // fetch throws TypeError on network failure (server unreachable / CORS).
    return {
      kind: "network",
      message: "Could not reach the server. Check the URL and that CORS is enabled.",
    };
  }
  return {
    kind: "unknown",
    message: err instanceof Error ? err.message : String(err),
  };
}
