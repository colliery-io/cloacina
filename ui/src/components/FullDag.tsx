/*
 *  Copyright 2025-2026 Colliery Software
 *  SPDX-License-Identifier: Apache-2.0
 *
 *  Full status-colored DAG (Aurora Dark spec §"DAG rendering", screens 04 + 09).
 *  Bespoke inline-SVG layered layout for pixel-parity with the design — replaces
 *  the @xyflow/react renderer for the full task graph (Execution detail) and the
 *  computation-graph topology (Graph detail).
 *
 *  Geometry (spec): node 128×38, colGap 148, rowGap 62, pad 20. Edges are cubic
 *  Béziers right-center → left-center. Nodes: --panel fill + status/kind border
 *  at 7a alpha; status dot (task DAG) or kind square (CG); running pulse;
 *  pending/skipped dimmed. Canvas is --inset and scrolls-x when overflowing.
 */
import { useMemo } from "react";

import { statusColor, nodeKindColor } from "../util/tokens";

const MONO = "'IBM Plex Mono', monospace";

const NODE_W = 128;
const NODE_H = 38;
const COLGAP = 148;
const ROWGAP = 62;
const PAD = 20;

const EDGE_IDLE = "#283039";
const EDGE_DONE = "rgba(75,208,127,.35)";
const EDGE_KIND = "#2a3340";
const ICE = "#7fb2ff";

export type DagKind = "accumulator" | "reactor" | "node" | "compute" | "trigger";

export interface FullDagNode {
  id: string;
  label?: string;
  status?: string;
  kind?: DagKind;
}
export interface FullDagEdge {
  from: string;
  to: string;
}

const isDone = (s?: string) => (s ?? "").toLowerCase() === "completed";
const isRunning = (s?: string) => (s ?? "").toLowerCase() === "running";
const isDim = (s?: string) => {
  const v = (s ?? "").toLowerCase();
  return v === "pending" || v === "skipped" || v === "not_started";
};

/** Layered layout: col = longest-path depth; within a col, order by the mean
 *  order-index of parents (barycenter) to reduce crossings; each col is centered
 *  vertically so branches read symmetrically (as in the spec screenshots). */
function layout(nodes: FullDagNode[], edges: FullDagEdge[]) {
  const ids = new Set(nodes.map((n) => n.id));
  const deps = new Map<string, string[]>();
  for (const n of nodes) deps.set(n.id, []);
  for (const e of edges) if (ids.has(e.from) && ids.has(e.to)) deps.get(e.to)!.push(e.from);

  const col = new Map<string, number>();
  const colOf = (id: string, seen: Set<string>): number => {
    if (col.has(id)) return col.get(id)!;
    if (seen.has(id)) return 0;
    seen.add(id);
    const d = deps.get(id) ?? [];
    const v = d.length ? Math.max(...d.map((p) => colOf(p, seen) + 1)) : 0;
    col.set(id, v);
    return v;
  };
  nodes.forEach((n) => colOf(n.id, new Set()));

  const cols = new Map<number, string[]>();
  let maxCol = 0;
  for (const n of nodes) {
    const c = col.get(n.id) ?? 0;
    maxCol = Math.max(maxCol, c);
    if (!cols.has(c)) cols.set(c, []);
    cols.get(c)!.push(n.id);
  }

  // Barycenter ordering, left → right.
  const orderIdx = new Map<string, number>();
  for (let c = 0; c <= maxCol; c++) {
    const list = cols.get(c) ?? [];
    if (c > 0) {
      list.sort((a, b) => {
        const ka = mean((deps.get(a) ?? []).map((p) => orderIdx.get(p) ?? 0));
        const kb = mean((deps.get(b) ?? []).map((p) => orderIdx.get(p) ?? 0));
        return ka - kb;
      });
    }
    list.forEach((id, i) => orderIdx.set(id, i));
  }

  const maxRows = Math.max(1, ...[...cols.values()].map((c) => c.length));
  const H = PAD * 2 + (maxRows - 1) * ROWGAP + NODE_H;
  const W = PAD * 2 + maxCol * COLGAP + NODE_W;
  const midY = H / 2;

  const pos = new Map<string, { x: number; y: number }>();
  for (const [c, list] of cols) {
    list.forEach((id, i) => {
      pos.set(id, {
        x: PAD + c * COLGAP,
        y: midY + (i - (list.length - 1) / 2) * ROWGAP - NODE_H / 2,
      });
    });
  }
  return { pos, W, H };
}

const mean = (xs: number[]) => (xs.length ? xs.reduce((a, b) => a + b, 0) / xs.length : 0);

function truncate(s: string, max = 15): string {
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

export function FullDag({
  nodes,
  edges,
  height = 300,
  onNodeClick,
  testId,
}: {
  nodes: FullDagNode[];
  edges: FullDagEdge[];
  height?: number;
  onNodeClick?: (id: string) => void;
  testId?: string;
}) {
  const lay = useMemo(() => layout(nodes, edges), [nodes, edges]);
  const byId = useMemo(() => new Map(nodes.map((n) => [n.id, n])), [nodes]);
  const statusMode = useMemo(() => nodes.some((n) => n.status != null), [nodes]);

  // Spread connection ports along each node's right/left edge so fan-in / fan-out
  // edges separate instead of bundling at the center port (the muddle on dense
  // graphs). A node with a single edge stays centered — matching the spec's
  // "right-center → left-center" for simple graphs. Ports are ordered by the
  // other endpoint's y to minimize crossings.
  const edgeGeom = useMemo(() => {
    const valid = edges.filter((e) => lay.pos.has(e.from) && lay.pos.has(e.to));
    const out = new Map<string, FullDagEdge[]>();
    const inc = new Map<string, FullDagEdge[]>();
    for (const e of valid) {
      if (!out.has(e.from)) out.set(e.from, []);
      out.get(e.from)!.push(e);
      if (!inc.has(e.to)) inc.set(e.to, []);
      inc.get(e.to)!.push(e);
    }
    for (const list of out.values()) list.sort((a, b) => lay.pos.get(a.to)!.y - lay.pos.get(b.to)!.y);
    for (const list of inc.values()) list.sort((a, b) => lay.pos.get(a.from)!.y - lay.pos.get(b.from)!.y);
    const port = (top: number, idx: number, n: number) => top + ((idx + 1) / (n + 1)) * NODE_H;
    return valid.map((e) => {
      const a = lay.pos.get(e.from)!;
      const b = lay.pos.get(e.to)!;
      const oList = out.get(e.from)!;
      const iList = inc.get(e.to)!;
      return {
        e,
        x1: a.x + NODE_W,
        y1: port(a.y, oList.indexOf(e), oList.length),
        x2: b.x,
        y2: port(b.y, iList.indexOf(e), iList.length),
      };
    });
  }, [edges, lay]);

  if (nodes.length === 0) return null;

  return (
    <div
      data-testid={testId}
      style={{
        background: "var(--inset)",
        border: "1px solid var(--border-soft)",
        borderRadius: 10,
        height,
        overflow: "auto",
      }}
    >
      <svg width={lay.W} height={Math.max(lay.H, height - 2)} style={{ display: "block" }}>
        {/* Edges */}
        {edgeGeom.map(({ e, x1, y1, x2, y2 }, i) => {
          const k = Math.max(34, (x2 - x1) / 2);
          const tgt = byId.get(e.to);
          const srcDone = isDone(byId.get(e.from)?.status);
          const stroke = !statusMode
            ? EDGE_KIND
            : isRunning(tgt?.status)
              ? ICE
              : isDone(tgt?.status) && srcDone
                ? EDGE_DONE
                : EDGE_IDLE;
          return (
            <path
              key={i}
              d={`M ${x1} ${y1} C ${x1 + k} ${y1}, ${x2 - k} ${y2}, ${x2} ${y2}`}
              fill="none"
              stroke={stroke}
              strokeWidth={1.5}
              strokeOpacity={0.92}
            />
          );
        })}

        {/* Nodes */}
        {nodes.map((n) => {
          const p = lay.pos.get(n.id)!;
          const c = n.kind ? nodeKindColor(n.kind) : statusColor(n.status ?? "");
          const dim = isDim(n.status);
          return (
            <g
              key={n.id}
              transform={`translate(${p.x},${p.y})`}
              onClick={onNodeClick ? () => onNodeClick(n.id) : undefined}
              className={isRunning(n.status) ? "cl-pulse" : undefined}
              style={{ cursor: onNodeClick ? "pointer" : "default", opacity: dim ? 0.55 : 1 }}
            >
              <rect width={NODE_W} height={NODE_H} rx={9} fill="var(--panel)" stroke={`${c}7a`} strokeWidth={1.4} />
              {n.kind ? (
                <rect x={12} y={NODE_H / 2 - 4} width={8} height={8} rx={2} fill={c} />
              ) : (
                <circle cx={16} cy={NODE_H / 2} r={4} fill={c} />
              )}
              <text x={28} y={NODE_H / 2 + 4} fontFamily={MONO} fontSize={12.5} fill="var(--fg)">
                {truncate(n.label ?? n.id)}
              </text>
              <title>{`${n.label ?? n.id}${n.status ? ` · ${n.status}` : ""}`}</title>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
