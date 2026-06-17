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

import { useRef } from "react";

/** Compact "time ago" for a last-fired timestamp (CLOACI-I-0124 / WS-10). */
export function formatAgo(ts: string | null | undefined): string {
  if (!ts) return "never";
  const t = Date.parse(ts);
  if (Number.isNaN(t)) return "—";
  const s = Math.max(0, Math.round((Date.now() - t) / 1000));
  if (s < 2) return "just now";
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  return `${Math.floor(s / 3600)}h ago`;
}

interface GraphFireSample {
  name: string;
  fires?: number;
}

/**
 * Derive recent throughput (fires/min) per graph from the reactor's monotonic
 * `fires` counter across successive `useGraphs` polls (CLOACI-I-0124 / WS-10).
 * The server only exposes the total; we compute the rate from the delta. The
 * baseline is only advanced when `fires` actually increases, so spurious
 * re-renders don't flap the value to 0; `null` until the first delta is seen.
 */
export function useGraphThroughput(items: GraphFireSample[]): Map<string, number | null> {
  const prev = useRef<Map<string, { fires: number; t: number; rate: number | null }>>(new Map());
  const now = Date.now();
  const out = new Map<string, number | null>();
  for (const g of items) {
    const fires = g.fires ?? 0;
    const p = prev.current.get(g.name);
    if (!p) {
      prev.current.set(g.name, { fires, t: now, rate: null });
      out.set(g.name, null);
    } else if (fires > p.fires) {
      const rate = Math.round(((fires - p.fires) / Math.max(now - p.t, 1)) * 60000);
      prev.current.set(g.name, { fires, t: now, rate });
      out.set(g.name, rate);
    } else {
      // No new fires since the last sample — keep the prior rate stable.
      out.set(g.name, p.rate);
    }
  }
  return out;
}
