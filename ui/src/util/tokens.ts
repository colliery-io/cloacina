/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Aurora Dark semantic color tokens (CLOACI-I-0129). The single source for the
 *  status / graph-health / node-kind → hex mappings the spec defines (dots,
 *  pills, pips, DAG nodes/edges all use these). Hexes match `theme.css`.
 *  Matching is case-insensitive with graceful fallbacks (REQ-007 defensive
 *  rendering) — server status strings are not asserted as a fixed enum.
 */

export const TOKEN = {
  ice: "#7fb2ff",
  teal: "#5fd0c5",
  violet: "#9d8cff",
  gold: "#d8a657",
  ok: "#4bd07f",
  bad: "#f06464",
  muted: "#8b95a3",
  faint: "#5b6573",
  edge: "#283039",
} as const;

/** Execution & task status → color. */
const STATUS_COLOR: Record<string, string> = {
  running: TOKEN.ice,
  completed: TOKEN.ok,
  failed: TOKEN.bad,
  scheduled: TOKEN.violet,
  pending: TOKEN.muted,
  cancelled: TOKEN.gold,
  canceled: TOKEN.gold,
  paused: TOKEN.muted,
  skipped: TOKEN.faint,
};

/** Computation-graph health → color. */
const HEALTH_COLOR: Record<string, string> = {
  live: TOKEN.ok,
  running: TOKEN.ok,
  healthy: TOKEN.ok,
  warming: TOKEN.gold,
  connecting: TOKEN.ice,
  socket_only: TOKEN.muted,
  stopped: TOKEN.muted,
  paused: TOKEN.gold,
  unreachable: TOKEN.bad,
};

/** Graph node kind → color (DAG topology). */
const NODE_KIND_COLOR: Record<string, string> = {
  accumulator: TOKEN.ice,
  reactor: TOKEN.violet,
  node: TOKEN.muted,
  compute: TOKEN.muted,
};

export function statusColor(status: string | null | undefined): string {
  return STATUS_COLOR[(status ?? "").toLowerCase()] ?? TOKEN.muted;
}

export function healthColor(health: string | null | undefined): string {
  return HEALTH_COLOR[(health ?? "").toLowerCase()] ?? TOKEN.muted;
}

export function nodeKindColor(kind: string | null | undefined): string {
  return NODE_KIND_COLOR[(kind ?? "").toLowerCase()] ?? TOKEN.muted;
}

/**
 * Tinted pill/badge background: the status color at `1c` alpha (spec §Pills).
 * `#7fb2ff` → `#7fb2ff1c`.
 */
export function pillBg(hex: string): string {
  return `${hex}1c`;
}
