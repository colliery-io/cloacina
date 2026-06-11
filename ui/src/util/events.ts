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

import type { ExecutionEvent } from "../components/EventLog";

/**
 * The OQ-6 history-vs-live merge (T-0656). Dedup on `sequence_num` — the
 * per-execution monotonic key — so the at-least-once delivery WS replaying
 * a row already in the REST history collapses to one entry. Last write wins
 * (live frame overwrites the historical copy of the same sequence). Order
 * is `EventLog`'s job (it sorts by sequence_num).
 */
export function mergeEvents(...sources: ExecutionEvent[][]): ExecutionEvent[] {
  const bySeq = new Map<number, ExecutionEvent>();
  for (const source of sources) {
    for (const e of source) bySeq.set(e.sequence_num, e);
  }
  return [...bySeq.values()];
}
