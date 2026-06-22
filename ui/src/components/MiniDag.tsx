/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Compact status-colored DAG thumbnail (Aurora Dark spec §"DAG rendering" +
 *  01/02). A lightweight inline-SVG layered graph — the mini variant of the
 *  signature motif. Pure/presentational: callers pass the node set + per-node
 *  status (execution task status, or graph node-kind). Geometry per spec:
 *  node 9×9 filled square, colGap 54, rowGap 21, pad 6; edges are cubic Béziers
 *  from a node's right-center to the target's left-center.
 */
import { useMemo } from "react";

import { statusColor, TOKEN } from "../util/tokens";

export interface MiniNode {
  id: string;
  dependencies: string[];
  /** Execution status (task DAG) — drives fill + pulse + dim. */
  status?: string;
  /** Explicit fill (CG topology mini-DAG colors by kind, not status). */
  color?: string;
}

const NODE = 9;
const COLGAP = 54;
const ROWGAP = 21;
const PAD = 6;

const EDGE_IDLE = "#283039";
const EDGE_DONE = "rgba(75,208,127,.35)";
const ICE = "#7fb2ff";

const isDone = (s?: string) => s != null && s.toLowerCase() === "completed";
const isRunning = (s?: string) => s != null && s.toLowerCase() === "running";
const isSkipped = (s?: string) => s != null && s.toLowerCase() === "skipped";
const isDim = (s?: string) => {
  const v = (s ?? "").toLowerCase();
  // `skipped` keeps its rose color (not dimmed) — branch-not-taken is signal.
  return v === "" || v === "pending" || v === "not_started";
};

function layout(nodes: MiniNode[]) {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const rank = new Map<string, number>();
  const rankOf = (id: string, seen: Set<string>): number => {
    if (rank.has(id)) return rank.get(id)!;
    if (seen.has(id)) return 0; // cycle guard
    seen.add(id);
    const deps = byId.get(id)?.dependencies ?? [];
    const v = deps.length ? Math.max(...deps.map((d) => rankOf(d, seen) + 1)) : 0;
    rank.set(id, v);
    return v;
  };
  nodes.forEach((n) => rankOf(n.id, new Set()));

  const cols = new Map<number, string[]>();
  let maxRank = 0;
  for (const n of nodes) {
    const r = rank.get(n.id) ?? 0;
    maxRank = Math.max(maxRank, r);
    if (!cols.has(r)) cols.set(r, []);
    cols.get(r)!.push(n.id);
  }
  const maxRows = Math.max(1, ...[...cols.values()].map((c) => c.length));
  const H = PAD * 2 + (maxRows - 1) * ROWGAP + NODE;
  const W = PAD * 2 + maxRank * COLGAP + NODE;
  const centerY = H / 2 - NODE / 2;

  const pos = new Map<string, { x: number; y: number }>();
  for (const [r, ids] of cols) {
    ids.forEach((id, i) => {
      pos.set(id, { x: PAD + r * COLGAP, y: centerY + (i - (ids.length - 1) / 2) * ROWGAP });
    });
  }
  return { pos, W, H };
}

export function MiniDag({ nodes }: { nodes: MiniNode[] }) {
  const lay = useMemo(() => layout(nodes), [nodes]);
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);

  if (nodes.length === 0) return null;

  return (
    <svg
      viewBox={`0 0 ${lay.W} ${lay.H}`}
      width={lay.W}
      height={lay.H}
      style={{ maxWidth: "100%", height: "auto", display: "block" }}
      role="img"
      aria-label="task graph"
    >
      {/* Edges: cubic Béziers right-center → left-center. */}
      {nodes.flatMap((n) =>
        n.dependencies
          .filter((d) => lay.pos.has(d) && lay.pos.has(n.id))
          .map((d) => {
            const a = lay.pos.get(d)!;
            const b = lay.pos.get(n.id)!;
            const x1 = a.x + NODE;
            const y1 = a.y + NODE / 2;
            const x2 = b.x;
            const y2 = b.y + NODE / 2;
            const k = Math.max(14, (x2 - x1) * 0.5);
            const dep = byId.get(d);
            const skipEdge = isSkipped(n.status) || isSkipped(dep?.status);
            const stroke = skipEdge
              ? TOKEN.skip
              : isRunning(n.status)
                ? ICE
                : isDone(n.status) && isDone(dep?.status)
                  ? EDGE_DONE
                  : EDGE_IDLE;
            return (
              <path
                key={`${d}->${n.id}`}
                d={`M ${x1} ${y1} C ${x1 + k} ${y1}, ${x2 - k} ${y2}, ${x2} ${y2}`}
                fill="none"
                stroke={stroke}
                strokeWidth={1.25}
                strokeDasharray={skipEdge ? "3 2" : undefined}
              />
            );
          }),
      )}

      {/* Nodes: filled squares, colored by status (or explicit kind color). */}
      {nodes.map((n) => {
        const p = lay.pos.get(n.id)!;
        const fill = n.color ?? statusColor(n.status ?? "");
        return (
          <rect
            key={n.id}
            x={p.x}
            y={p.y}
            width={NODE}
            height={NODE}
            rx={2.5}
            fill={fill}
            className={isRunning(n.status) ? "cl-pulse" : undefined}
            opacity={isDim(n.status) ? 0.4 : 1}
          >
            <title>{`${n.id}${n.status ? ` · ${n.status}` : ""}`}</title>
          </rect>
        );
      })}
    </svg>
  );
}
