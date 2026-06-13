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

/** A task node: its id and the ids of the tasks it depends on. */
export interface TaskGraphNode {
  id: string;
  dependencies: string[];
  description?: string | null;
}

const NODE_W = 172;
const NODE_H = 44;

/**
 * Interactive workflow DAG (CLOACI-T-0663). Lays tasks out left→right by
 * topological rank with dagre and renders them as a pan/zoom React Flow graph;
 * edges point from each dependency to the dependent task. Reusable for any
 * dependency graph (e.g. computation-graph views).
 */
export function WorkflowGraph({ tasks }: { tasks: TaskGraphNode[] }) {
  const { nodes, edges } = useMemo(() => {
    const ids = new Set(tasks.map((t) => t.id));

    const rawEdges: Edge[] = tasks.flatMap((t) =>
      t.dependencies
        .filter((dep) => ids.has(dep))
        .map((dep) => ({
          id: `${dep}->${t.id}`,
          source: dep,
          target: t.id,
          markerEnd: { type: MarkerType.ArrowClosed },
        })),
    );

    const g = new dagre.graphlib.Graph();
    g.setDefaultEdgeLabel(() => ({}));
    g.setGraph({ rankdir: "LR", nodesep: 36, ranksep: 80, marginx: 8, marginy: 8 });
    tasks.forEach((t) => g.setNode(t.id, { width: NODE_W, height: NODE_H }));
    rawEdges.forEach((e) => g.setEdge(e.source, e.target));
    dagre.layout(g);

    const rfNodes: Node[] = tasks.map((t) => {
      const p = g.node(t.id);
      return {
        id: t.id,
        data: { label: t.id },
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

    return { nodes: rfNodes, edges: rawEdges };
  }, [tasks]);

  return (
    <div style={{ height: 420 }} data-testid="workflow-graph">
      <ReactFlow
        nodes={nodes}
        edges={edges}
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
