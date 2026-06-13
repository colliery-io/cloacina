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

import { useMemo } from "react";
import dagre from "@dagrejs/dagre";
import {
  Background,
  Controls,
  type Edge,
  MarkerType,
  type Node,
  Position,
  ReactFlow,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

/** A node in a directed graph. */
export interface DagNode {
  id: string;
  /** Display label (defaults to `id`). */
  label?: string;
}

/** A directed edge `from → to`, with an optional label (e.g. routing variant). */
export interface DagEdge {
  from: string;
  to: string;
  label?: string | null;
}

const NODE_W = 172;
const NODE_H = 44;

/**
 * Interactive directed-acyclic-graph view (CLOACI-T-0663 / T-0673). Lays nodes
 * out left→right by topological rank with dagre and renders them as a pan/zoom
 * React Flow graph. Shared by the workflow task DAG and the computation-graph
 * node/edge DAG.
 */
export function Dag({
  nodes,
  edges,
  height = 420,
  testId,
}: {
  nodes: DagNode[];
  edges: DagEdge[];
  height?: number;
  testId?: string;
}) {
  const { rfNodes, rfEdges } = useMemo(() => {
    const ids = new Set(nodes.map((n) => n.id));

    const drawnEdges: Edge[] = edges
      .filter((e) => ids.has(e.from) && ids.has(e.to))
      .map((e, i) => ({
        id: `${e.from}->${e.to}-${i}`,
        source: e.from,
        target: e.to,
        label: e.label ?? undefined,
        markerEnd: { type: MarkerType.ArrowClosed },
      }));

    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: "LR", nodesep: 36, ranksep: 80, marginx: 8, marginy: 8 });
    nodes.forEach((n) => g.setNode(n.id, { width: NODE_W, height: NODE_H }));
    drawnEdges.forEach((e) => g.setEdge(e.source, e.target));
    dagre.layout(g);

    const laidOut: Node[] = nodes.map((n) => {
      const p = g.node(n.id);
      return {
        id: n.id,
        data: { label: n.label ?? n.id },
        position: { x: p.x - NODE_W / 2, y: p.y - NODE_H / 2 },
        sourcePosition: Position.Right,
        targetPosition: Position.Left,
        style: {
          width: NODE_W,
          fontSize: 13,
          borderRadius: 8,
          border: "1px solid var(--mantine-color-default-border)",
          padding: "8px 10px",
        },
      };
    });

    return { rfNodes: laidOut, rfEdges: drawnEdges };
  }, [nodes, edges]);

  return (
    <div style={{ height }} data-testid={testId}>
      <ReactFlow
        nodes={rfNodes}
        edges={rfEdges}
        fitView
        nodesDraggable
        proOptions={{ hideAttribution: true }}
        minZoom={0.2}
      >
        <Background />
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
  );
}
